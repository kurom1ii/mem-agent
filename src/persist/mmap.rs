use std::fs;
use std::io::{Read, Write};

use crate::core::error::{MemAgentError, Result};

#[derive(Debug)]
pub struct MmapVectorStore {
    pub ids: Vec<i64>,
    pub vectors: Vec<Vec<f32>>,
    dim: usize,
    path: String,
}

impl MmapVectorStore {
    pub fn open(path: &str, dim: usize) -> Result<Self> {
        let mut file = fs::File::open(path).map_err(|e| {
            MemAgentError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to open vector store '{path}': {e}"),
            ))
        })?;

        let mut header = [0u8; 16];
        file.read_exact(&mut header)?;

        let n_docs = u64::from_le_bytes(header[..8].try_into().unwrap()) as usize;
        let file_dim = u64::from_le_bytes(header[8..].try_into().unwrap()) as usize;

        if file_dim != dim {
            return Err(MemAgentError::Dimension(format!(
                "Dimension mismatch: expected {dim}, found {file_dim} in '{path}'"
            )));
        }

        let ids = Self::read_ids(&mut file, n_docs)?;
        let vectors = Self::read_vectors(&mut file, n_docs, dim)?;

        Ok(Self {
            ids,
            vectors,
            dim,
            path: path.to_string(),
        })
    }

    fn read_ids(file: &mut fs::File, n_docs: usize) -> Result<Vec<i64>> {
        if n_docs == 0 {
            return Ok(Vec::new());
        }
        let bytes_per_id = std::mem::size_of::<i64>();
        let mut buf = vec![0u8; n_docs * bytes_per_id];
        file.read_exact(&mut buf)?;

        let ids: Vec<i64> = bytemuck::cast_slice(&buf).to_vec();
        Ok(ids)
    }

    fn read_vectors(file: &mut fs::File, n_docs: usize, dim: usize) -> Result<Vec<Vec<f32>>> {
        if n_docs == 0 {
            return Ok(Vec::new());
        }
        let bytes_per_vec = dim * std::mem::size_of::<f32>();
        let mut buf = vec![0u8; n_docs * bytes_per_vec];
        file.read_exact(&mut buf)?;

        let flat: &[f32] = bytemuck::cast_slice(&buf);

        let vectors: Vec<Vec<f32>> = (0..n_docs)
            .map(|i| {
                let start = i * dim;
                flat[start..start + dim].to_vec()
            })
            .collect();

        Ok(vectors)
    }

    pub fn save(&self) -> Result<()> {
        let n_docs = self.ids.len() as u64;
        let dim = self.dim as u64;

        let mut file = fs::File::create(&self.path)?;

        file.write_all(&n_docs.to_le_bytes())?;
        file.write_all(&dim.to_le_bytes())?;

        if n_docs > 0 {
            let id_bytes: &[u8] = bytemuck::cast_slice(&self.ids);
            file.write_all(id_bytes)?;

            let flat: Vec<f32> = self
                .vectors
                .iter()
                .flat_map(|v| v.iter().copied())
                .collect();
            let vec_bytes: &[u8] = bytemuck::cast_slice(&flat);
            file.write_all(vec_bytes)?;
        }

        Ok(())
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn get(&self, idx: usize) -> Option<&[f32]> {
        self.vectors.get(idx).map(|v| v.as_slice())
    }

    pub fn get_id(&self, idx: usize) -> Option<i64> {
        self.ids.get(idx).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_load_roundtrip() {
        let path = "/tmp/test_mmap_vectors.bin";
        let dim = 4;

        let store = MmapVectorStore {
            ids: vec![10, 20, 30],
            vectors: vec![
                vec![1.0f32, 2.0, 3.0, 4.0],
                vec![5.0f32, 6.0, 7.0, 8.0],
                vec![9.0f32, 10.0, 11.0, 12.0],
            ],
            dim,
            path: path.to_string(),
        };

        store.save().unwrap();

        let loaded = MmapVectorStore::open(path, dim).unwrap();
        assert_eq!(loaded.ids, vec![10, 20, 30]);
        assert_eq!(loaded.vectors.len(), 3);
        assert_eq!(loaded.get(1).unwrap(), &[5.0f32, 6.0, 7.0, 8.0]);

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_save_and_load_empty() {
        let path = "/tmp/test_mmap_empty.bin";
        let dim = 384;

        let store = MmapVectorStore {
            ids: Vec::new(),
            vectors: Vec::new(),
            dim,
            path: path.to_string(),
        };

        store.save().unwrap();
        let loaded = MmapVectorStore::open(path, dim).unwrap();
        assert!(loaded.is_empty());
        assert_eq!(loaded.len(), 0);
        assert!(loaded.get(0).is_none());
        assert!(loaded.get_id(0).is_none());

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_dimension_mismatch() {
        let path = "/tmp/test_mmap_dim.bin";
        let dim = 4;

        let store = MmapVectorStore {
            ids: vec![1],
            vectors: vec![vec![1.0f32, 2.0, 3.0, 4.0]],
            dim,
            path: path.to_string(),
        };
        store.save().unwrap();

        let result = MmapVectorStore::open(path, 8);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Dimension mismatch"));

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_get_and_get_id() {
        let path = "/tmp/test_mmap_get.bin";
        let dim = 2;

        let store = MmapVectorStore {
            ids: vec![100, 200],
            vectors: vec![vec![0.1f32, 0.2], vec![0.3f32, 0.4]],
            dim,
            path: path.to_string(),
        };
        store.save().unwrap();

        let loaded = MmapVectorStore::open(path, dim).unwrap();

        assert_eq!(loaded.get_id(0), Some(100));
        assert_eq!(loaded.get_id(1), Some(200));
        assert_eq!(loaded.get_id(2), None);

        assert_eq!(loaded.get(0), Some(&[0.1f32, 0.2][..]));
        assert_eq!(loaded.get(1), Some(&[0.3f32, 0.4][..]));
        assert_eq!(loaded.get(2), None);

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_open_missing_file() {
        let result = MmapVectorStore::open("/tmp/nonexistent_mmap_test_99999.bin", 128);
        assert!(result.is_err());
    }
}
