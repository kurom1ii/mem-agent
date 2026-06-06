use std::collections::HashMap;

use crate::core::config::{K_RRF, W_FTS, W_VEC};
use crate::core::types::{Memory, SearchHit, SearchResult};

/// Fuse FTS5 and vector search results using Reciprocal Rank Fusion (RRF).
///
/// Each document's combined score:
///   score = W_FTS / (K_RRF + rank_fts) + W_VEC / (K_RRF + rank_vec)
///
/// Documents missing from one list get a penalty rank of `limit * 3`.
pub fn rrf_fuse(
    fts_results: &[(i64, f64)],
    vec_results: &[SearchHit],
    limit: usize,
) -> Vec<SearchResult> {
    let penalty_rank = (limit * 3) as f64;

    let mut doc_map: HashMap<i64, (Option<(usize, f64)>, Option<(usize, f64)>)> =
        HashMap::new();

    for (rank, (id, score)) in fts_results.iter().enumerate() {
        doc_map
            .entry(*id)
            .and_modify(|e| e.0 = Some((rank + 1, *score)))
            .or_insert_with(|| (Some((rank + 1, *score)), None));
    }

    for (rank, hit) in vec_results.iter().enumerate() {
        doc_map
            .entry(hit.memory_id)
            .and_modify(|e| e.1 = Some((rank + 1, hit.score as f64)))
            .or_insert_with(|| (None, Some((rank + 1, hit.score as f64))));
    }

    let mut fused: Vec<(i64, f64, f64, f64)> = doc_map
        .iter()
        .map(|(id, (fts, vec))| {
            let fts_rank = fts.map(|(r, _)| r as f64).unwrap_or(penalty_rank);
            let fts_score = fts.map(|(_, s)| s).unwrap_or(0.0);
            let vec_rank = vec.map(|(r, _)| r as f64).unwrap_or(penalty_rank);
            let vec_score = vec.map(|(_, s)| s).unwrap_or(0.0);

            let combined =
                W_FTS * (1.0 / (K_RRF + fts_rank)) + W_VEC * (1.0 / (K_RRF + vec_rank));

            (*id, combined, fts_score, vec_score)
        })
        .collect();

    fused.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    fused.truncate(limit);

    fused
        .into_iter()
        .map(|(id, score, fts_score, vec_score)| SearchResult {
            memory: Memory {
                id,
                title: String::new(),
                content: String::new(),
                tags: String::new(),
                created_at: String::new(),
                updated_at: String::new(),
            },
            score,
            fts_score,
            vector_score: vec_score,
            snippet: String::new(),
        })
        .collect()
}

/// Normalize `fts_score` and `vector_score` fields to [0, 1] using min-max
/// normalization. When all values are identical, scores are set to 1.0.
pub fn normalize_scores(results: &mut [SearchResult]) {
    if results.is_empty() {
        return;
    }

    let mut min_fts = f64::MAX;
    let mut max_fts = f64::MIN;
    let mut min_vec = f64::MAX;
    let mut max_vec = f64::MIN;

    for r in results.iter() {
        min_fts = min_fts.min(r.fts_score);
        max_fts = max_fts.max(r.fts_score);
        min_vec = min_vec.min(r.vector_score);
        max_vec = max_vec.max(r.vector_score);
    }

    let fts_range = max_fts - min_fts;
    let vec_range = max_vec - min_vec;

    for r in results.iter_mut() {
        r.fts_score = if fts_range > 1e-10 {
            (r.fts_score - min_fts) / fts_range
        } else {
            1.0
        };
        r.vector_score = if vec_range > 1e-10 {
            (r.vector_score - min_vec) / vec_range
        } else {
            1.0
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rrf_fuse_overlap() {
        let fts_results = vec![(1i64, -1.5), (2i64, -2.0), (3i64, -3.0)];
        let vec_results = vec![
            SearchHit {
                memory_id: 2,
                score: 0.95,
            },
            SearchHit {
                memory_id: 3,
                score: 0.80,
            },
            SearchHit {
                memory_id: 4,
                score: 0.60,
            },
        ];

        let fused = rrf_fuse(&fts_results, &vec_results, 5);

        assert!(!fused.is_empty());

        let ids: Vec<i64> = fused.iter().map(|r| r.memory.id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
        assert!(ids.contains(&3));
        assert!(ids.contains(&4));

        let doc2 = fused.iter().find(|r| r.memory.id == 2).unwrap();
        let doc4 = fused.iter().find(|r| r.memory.id == 4).unwrap();
        assert!(doc2.score > doc4.score);
    }

    #[test]
    fn test_rrf_fuse_both_in_both_lists() {
        let fts_results = vec![(1i64, -1.0)];
        let vec_results = vec![SearchHit {
            memory_id: 1,
            score: 0.9,
        }];

        let fused = rrf_fuse(&fts_results, &vec_results, 10);
        assert_eq!(fused.len(), 1);
        assert_eq!(fused[0].memory.id, 1);
        let expected =
            W_FTS * (1.0 / (K_RRF + 1.0)) + W_VEC * (1.0 / (K_RRF + 1.0));
        assert!((fused[0].score - expected).abs() < 0.0001);
    }

    #[test]
    fn test_rrf_fuse_fts_only() {
        let fts_results = vec![(10i64, -2.0), (20i64, -1.0)];
        let vec_results: Vec<SearchHit> = vec![];
        let fused = rrf_fuse(&fts_results, &vec_results, 3);

        assert_eq!(fused.len(), 2);
        assert_eq!(fused[0].memory.id, 10);
        assert!(fused[0].score > fused[1].score);
    }

    #[test]
    fn test_rrf_fuse_vec_only() {
        let fts_results: Vec<(i64, f64)> = vec![];
        let vec_results = vec![
            SearchHit {
                memory_id: 100,
                score: 0.9,
            },
            SearchHit {
                memory_id: 200,
                score: 0.5,
            },
        ];
        let fused = rrf_fuse(&fts_results, &vec_results, 3);

        assert_eq!(fused.len(), 2);
        assert_eq!(fused[0].memory.id, 100);
        assert!(fused[0].score > fused[1].score);
    }

    #[test]
    fn test_rrf_fuse_empty() {
        let fused = rrf_fuse(&[], &[], 10);
        assert!(fused.is_empty());
    }

    #[test]
    fn test_rrf_fuse_limit_truncation() {
        let fts_results: Vec<(i64, f64)> = (0..10).map(|i| (i, -1.0)).collect();
        let vec_results: Vec<SearchHit> = (5..15)
            .map(|i| SearchHit {
                memory_id: i,
                score: 0.5,
            })
            .collect();

        let fused = rrf_fuse(&fts_results, &vec_results, 5);
        assert_eq!(fused.len(), 5);
    }

    #[test]
    fn test_normalize_scores_varied() {
        let mut results = vec![
            SearchResult {
                memory: Memory {
                    id: 1,
                    title: String::new(),
                    content: String::new(),
                    tags: String::new(),
                    created_at: String::new(),
                    updated_at: String::new(),
                },
                score: 0.5,
                fts_score: -3.0,
                vector_score: 0.2,
                snippet: String::new(),
            },
            SearchResult {
                memory: Memory {
                    id: 2,
                    title: String::new(),
                    content: String::new(),
                    tags: String::new(),
                    created_at: String::new(),
                    updated_at: String::new(),
                },
                score: 0.3,
                fts_score: -1.0,
                vector_score: 0.8,
                snippet: String::new(),
            },
        ];

        normalize_scores(&mut results);

        let a = &results[0];
        let b = &results[1];
        assert!((a.fts_score - 0.0).abs() < 0.001);
        assert!((b.fts_score - 1.0).abs() < 0.001);
        assert!((a.vector_score - 0.0).abs() < 0.001);
        assert!((b.vector_score - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_normalize_scores_identical() {
        let mut results = vec![
            SearchResult {
                memory: Memory {
                    id: 1,
                    title: String::new(),
                    content: String::new(),
                    tags: String::new(),
                    created_at: String::new(),
                    updated_at: String::new(),
                },
                score: 0.0,
                fts_score: 5.0,
                vector_score: 0.7,
                snippet: String::new(),
            },
            SearchResult {
                memory: Memory {
                    id: 2,
                    title: String::new(),
                    content: String::new(),
                    tags: String::new(),
                    created_at: String::new(),
                    updated_at: String::new(),
                },
                score: 0.0,
                fts_score: 5.0,
                vector_score: 0.7,
                snippet: String::new(),
            },
        ];

        normalize_scores(&mut results);
        for r in &results {
            assert!((r.fts_score - 1.0).abs() < 0.001);
            assert!((r.vector_score - 1.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_normalize_scores_empty() {
        let mut results: Vec<SearchResult> = vec![];
        normalize_scores(&mut results);
        assert!(results.is_empty());
    }
}
