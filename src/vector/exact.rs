use crate::core::types::SearchHit;
use crate::vector::distance::cosine;
use crate::vector::store::VectorStore;
use rayon::prelude::*;

pub fn exact_knn(query: &[f32], store: &VectorStore, k: usize) -> Vec<SearchHit> {
    let n = store.len();
    if n == 0 || k == 0 {
        return Vec::new();
    }

    let effective_k = k.min(n);

    let scores: Vec<(usize, f32)> = (0..n)
        .into_par_iter()
        .map(|idx| {
            let vec = store.get_vector(idx).unwrap();
            let score = cosine(query, vec);
            (idx, score)
        })
        .collect();

    let mut with_indices: Vec<(usize, f32)> = scores;
    with_indices
        .sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    with_indices.truncate(effective_k);

    with_indices
        .into_iter()
        .map(|(idx, score)| SearchHit {
            memory_id: store.get_id(idx).unwrap_or(0),
            score,
        })
        .collect()
}

pub fn exact_knn_single(
    query: &[f32],
    vectors: &[Vec<f32>],
    ids: &[i64],
    k: usize,
) -> Vec<SearchHit> {
    let n = vectors.len();
    if n == 0 || k == 0 {
        return Vec::new();
    }

    let effective_k = k.min(n);

    let scores: Vec<(usize, f32)> = (0..n)
        .into_par_iter()
        .map(|idx| {
            let score = cosine(query, &vectors[idx]);
            (idx, score)
        })
        .collect();

    let mut with_indices = scores;
    with_indices
        .sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    with_indices.truncate(effective_k);

    with_indices
        .into_iter()
        .map(|(idx, score)| SearchHit {
            memory_id: ids[idx],
            score,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::VectorMeta;

    fn make_store() -> VectorStore {
        let mut store = VectorStore::new(4);
        let meta = VectorMeta {
            memory_id: 0,
            created_at_epoch: 0,
        };

        store
            .insert(1, vec![1.0, 0.0, 0.0, 0.0], meta.clone())
            .unwrap();

        let meta = VectorMeta {
            memory_id: 1,
            created_at_epoch: 0,
        };
        store
            .insert(2, vec![0.0, 1.0, 0.0, 0.0], meta.clone())
            .unwrap();

        let meta = VectorMeta {
            memory_id: 2,
            created_at_epoch: 0,
        };
        store.insert(3, vec![0.5, 0.0, 0.0, 0.0], meta).unwrap();

        store
    }

    #[test]
    fn test_exact_knn_empty() {
        let store = VectorStore::new(4);
        let result = exact_knn(&[1.0; 4], &store, 10);
        assert!(result.is_empty());
    }

    #[test]
    fn test_exact_knn_top1() {
        let store = make_store();
        let query = vec![1.0, 0.0, 0.0, 0.0];
        let results = exact_knn(&query, &store, 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].memory_id, 1);
        assert!((results[0].score - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_exact_knn_ordering() {
        let store = make_store();
        let query = vec![1.0, 0.0, 0.0, 0.0];
        let results = exact_knn(&query, &store, 3);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].memory_id, 1);
        assert!(results[0].score >= results[1].score);
        assert!(results[1].score >= results[2].score);
    }

    #[test]
    fn test_exact_knn_single_matches_store() {
        let store = make_store();
        let query = vec![0.0, 1.0, 0.0, 0.0];
        let r1 = exact_knn(&query, &store, 2);
        let r2 = exact_knn_single(&query, &store.vectors, &store.ids, 2);
        assert_eq!(r1.len(), r2.len());
        for (a, b) in r1.iter().zip(r2.iter()) {
            assert_eq!(a.memory_id, b.memory_id);
            assert!((a.score - b.score).abs() < 0.001);
        }
    }
}
