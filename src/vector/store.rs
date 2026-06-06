use crate::core::config::DEFAULT_DIM;
use crate::core::error::{MemAgentError, Result};
use crate::core::types::VectorMeta;

#[derive(Clone, Debug)]
pub struct VectorStore {
    pub ids: Vec<i64>,
    pub vectors: Vec<Vec<f32>>,
    pub metadata: Vec<VectorMeta>,
    dim: usize,
}

impl VectorStore {
    pub fn new(dim: usize) -> Self {
        Self {
            ids: Vec::new(),
            vectors: Vec::new(),
            metadata: Vec::new(),
            dim,
        }
    }

    pub fn with_capacity(dim: usize, capacity: usize) -> Self {
        Self {
            ids: Vec::with_capacity(capacity),
            vectors: Vec::with_capacity(capacity),
            metadata: Vec::with_capacity(capacity),
            dim,
        }
    }

    pub fn default_dim() -> Self {
        Self::new(DEFAULT_DIM)
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn dim(&self) -> usize {
        self.dim
    }

    pub fn insert(&mut self, memory_id: i64, vector: Vec<f32>, meta: VectorMeta) -> Result<()> {
        if vector.len() != self.dim {
            return Err(MemAgentError::Dimension(format!(
                "Expected {} dims, got {}",
                self.dim,
                vector.len()
            )));
        }
        self.ids.push(memory_id);
        self.vectors.push(vector);
        self.metadata.push(meta);
        Ok(())
    }

    pub fn remove(&mut self, memory_id: i64) -> bool {
        if let Some(pos) = self.ids.iter().position(|&id| id == memory_id) {
            self.ids.remove(pos);
            self.vectors.remove(pos);
            self.metadata.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn get(&self, memory_id: i64) -> Option<&[f32]> {
        self.ids
            .iter()
            .position(|&id| id == memory_id)
            .map(|idx| self.vectors[idx].as_slice())
    }

    pub fn get_vector(&self, index: usize) -> Option<&[f32]> {
        self.vectors.get(index).map(|v| v.as_slice())
    }

    pub fn get_id(&self, index: usize) -> Option<i64> {
        self.ids.get(index).copied()
    }

    pub fn index_of(&self, memory_id: i64) -> Option<usize> {
        self.ids.iter().position(|&id| id == memory_id)
    }
}

impl Default for VectorStore {
    fn default() -> Self {
        Self::default_dim()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_store_insert_get() {
        let mut store = VectorStore::new(4);
        let vec = vec![1.0, 2.0, 3.0, 4.0];
        let meta = VectorMeta {
            memory_id: 1,
            created_at: "2024-01-01".into(),
        };
        store.insert(1, vec.clone(), meta).unwrap();
        assert_eq!(store.len(), 1);
        assert_eq!(store.get(1).unwrap(), &[1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_vector_store_remove() {
        let mut store = VectorStore::new(3);
        let meta = VectorMeta {
            memory_id: 42,
            created_at: "2024-01-01".into(),
        };
        store.insert(42, vec![0.0; 3], meta).unwrap();
        assert!(store.remove(42));
        assert!(!store.remove(42));
        assert!(store.is_empty());
    }

    #[test]
    fn test_dimension_mismatch() {
        let mut store = VectorStore::new(3);
        let meta = VectorMeta {
            memory_id: 1,
            created_at: "".into(),
        };
        assert!(store.insert(1, vec![0.0; 5], meta).is_err());
    }
}
