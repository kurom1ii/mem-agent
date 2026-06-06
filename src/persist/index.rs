use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::core::error::{MemAgentError, Result};

const CURRENT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
pub struct PersistedIndex {
    pub version: u32,
    pub dim: usize,
    pub ids: Vec<i64>,
    pub vectors: Vec<Vec<f32>>,
}

pub fn save_index(path: &str, ids: &[i64], vectors: &[Vec<f32>], dim: usize) -> Result<()> {
    let index = PersistedIndex {
        version: CURRENT_VERSION,
        dim,
        ids: ids.to_vec(),
        vectors: vectors.to_vec(),
    };

    let data = bincode::serde::encode_to_vec(&index, bincode::config::standard())
        .map_err(|e| MemAgentError::Bincode(e.to_string()))?;

    let mut file = fs::File::create(path)?;
    file.write_all(&data)?;

    Ok(())
}

pub fn load_index(path: &str) -> Result<(Vec<i64>, Vec<Vec<f32>>, usize)> {
    let mut file = fs::File::open(path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    let (index, _): (PersistedIndex, _) =
        bincode::serde::decode_from_slice(&data, bincode::config::standard())
            .map_err(|e| MemAgentError::Bincode(e.to_string()))?;

    if index.ids.len() != index.vectors.len() {
        return Err(MemAgentError::Serialize(
            "Index ids/vectors length mismatch".to_string(),
        ));
    }

    Ok((index.ids, index.vectors, index.dim))
}

pub fn save_index_json(path: &str, ids: &[i64], vectors: &[Vec<f32>]) -> Result<()> {
    let json = serde_json::json!({
        "ids": ids,
        "vectors": vectors,
    });

    let data = serde_json::to_string_pretty(&json)?;
    let mut file = fs::File::create(path)?;
    file.write_all(data.as_bytes())?;

    Ok(())
}

pub fn index_exists(path: &str) -> bool {
    Path::new(path).exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_data() -> (Vec<i64>, Vec<Vec<f32>>) {
        let ids: Vec<i64> = vec![1, 2, 3, 42];
        let vectors: Vec<Vec<f32>> = vec![
            vec![1.0f32, 0.5, 0.25],
            vec![0.0f32, 1.0, 0.5],
            vec![0.5f32, 0.0, 1.0],
            vec![0.25f32, 0.75, 0.0],
        ];
        (ids, vectors)
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let (ids, vectors) = make_test_data();
        let dim = 3;
        let path = "/tmp/test_index_roundtrip.bin";

        save_index(path, &ids, &vectors, dim).unwrap();
        assert!(index_exists(path));

        let (loaded_ids, loaded_vectors, loaded_dim) = load_index(path).unwrap();

        assert_eq!(loaded_ids, ids);
        assert_eq!(loaded_vectors, vectors);
        assert_eq!(loaded_dim, dim);

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_save_and_load_empty() {
        let path = "/tmp/test_index_empty.bin";

        save_index(path, &[], &[], 384).unwrap();
        assert!(index_exists(path));

        let (loaded_ids, loaded_vectors, loaded_dim) = load_index(path).unwrap();
        assert!(loaded_ids.is_empty());
        assert!(loaded_vectors.is_empty());
        assert_eq!(loaded_dim, 384);

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_load_missing_file() {
        let result = load_index("/tmp/nonexistent_file_for_test_12345.bin");
        assert!(result.is_err());
    }

    #[test]
    fn test_index_exists() {
        let path = "/tmp/test_index_exists.bin";

        assert!(!index_exists(path));

        let (ids, vectors) = make_test_data();
        save_index(path, &ids, &vectors, 3).unwrap();

        assert!(index_exists(path));
        fs::remove_file(path).ok();
    }

    #[test]
    fn test_save_json() {
        let (ids, vectors) = make_test_data();
        let path = "/tmp/test_index_json.json";

        save_index_json(path, &ids, &vectors).unwrap();
        assert!(index_exists(path));

        let content = fs::read_to_string(path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(json.get("ids").is_some());
        assert!(json.get("vectors").is_some());

        fs::remove_file(path).ok();
    }
}
