use crate::core::error::{MemAgentError, Result};
use crate::core::types::{SearchResult, VectorMeta};
use crate::db::ops::{get_memory, load_all_vectors};
use crate::embed::engine::EmbeddingEngine;
use crate::search::fts5_search::Fts5Searcher;
use crate::search::ranking::rrf_fuse;
use crate::text::snippet::generate_snippet;
use crate::vector::exact::exact_knn_single;
use crate::vector::store::VectorStore;

pub struct HybridSearch {
    pub fts5: Fts5Searcher,
    pub embed: EmbeddingEngine,
    pub vector_store: VectorStore,
}

impl HybridSearch {
    pub fn new(embed: EmbeddingEngine) -> Self {
        Self {
            fts5: Fts5Searcher::new(),
            embed,
            vector_store: VectorStore::default_dim(),
        }
    }

    /// Full hybrid search: embed query → FTS5 + vector → RRF fuse → enrich.
    pub fn search(
        &self,
        conn: &rusqlite::Connection,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let query_vector = self
            .embed
            .embed(query)
            .map_err(MemAgentError::Embed)?;

        let fts_results = self.fts5.search(conn, query, limit * 3)?;

        let vec_hits = exact_knn_single(
            &query_vector,
            &self.vector_store.vectors,
            &self.vector_store.ids,
            limit * 3,
        );

        let fts_pairs: Vec<(i64, f64)> = fts_results
            .iter()
            .map(|r| (r.memory.id, r.fts_score))
            .collect();

        let mut fused = rrf_fuse(&fts_pairs, &vec_hits, limit);

        for result in fused.iter_mut() {
            let id = result.memory.id;
            result.memory = get_memory(conn, id).unwrap_or_else(|_| {
                let mut m = result.memory.clone();
                m.title = "[deleted]".into();
                m.content = "[deleted]".into();
                m
            });
            result.snippet = generate_snippet(&result.memory.content, query, 0);
        }

        Ok(fused)
    }

    /// FTS5-only search (no embedding, no vector search).
    pub fn search_fts5_only(
        &self,
        conn: &rusqlite::Connection,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        self.fts5.search(conn, query, limit)
    }

    /// Vector-only search (embed query, no FTS5).
    pub fn search_vector_only(
        &self,
        conn: &rusqlite::Connection,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let query_vector = self
            .embed
            .embed(query)
            .map_err(MemAgentError::Embed)?;

        let vec_hits = exact_knn_single(
            &query_vector,
            &self.vector_store.vectors,
            &self.vector_store.ids,
            limit,
        );

        let mut results = Vec::with_capacity(vec_hits.len());
        for hit in vec_hits {
            let memory = get_memory(conn, hit.memory_id)?;
            let snippet = generate_snippet(&memory.content, query, 0);

            results.push(SearchResult {
                memory,
                score: hit.score as f64,
                fts_score: 0.0,
                vector_score: hit.score as f64,
                snippet,
            });
        }

        Ok(results)
    }

    /// Load all vectors from the database into the internal VectorStore.
    /// Dimension is detected automatically from the first loaded vector.
    pub fn load_vectors(&mut self, conn: &rusqlite::Connection) -> Result<()> {
        let entries = load_all_vectors(conn)?;

        if entries.is_empty() {
            self.vector_store = VectorStore::default_dim();
            return Ok(());
        }

        let dim = entries[0].vector.len();
        self.vector_store = VectorStore::new(dim);

        for entry in entries {
            let meta = VectorMeta {
                memory_id: entry.memory_id,
                created_at: String::new(),
            };
            self.vector_store
                .insert(entry.memory_id, entry.vector, meta)?;
        }

        Ok(())
    }

    /// Build both FTS5 and vector indices from database.
    pub fn build_indices(&mut self, conn: &rusqlite::Connection) -> Result<()> {
        self.fts5.build_index(conn)?;
        self.load_vectors(conn)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::SearchHit;
    use crate::db::ops::insert_memory;
    use crate::db::ops::insert_vector;
    use crate::db::schema::init_db;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        init_db(&conn).unwrap();

        let entries = [
            (
                "Rust Programming",
                "Rust is a systems programming language",
                "rust",
            ),
            ("Python Guide", "Python is great for data science", "python"),
            (
                "Async Rust",
                "Rust async programming with tokio",
                "rust,async",
            ),
            ("JavaScript", "JavaScript runs in the browser", "javascript"),
            (
                "Memory Safety",
                "Rust guarantees memory safety without GC",
                "rust,memory",
            ),
        ];

        for (title, content, tags) in entries {
            insert_memory(&conn, title, content, tags).unwrap();
        }

        conn
    }

    fn insert_test_vectors(conn: &Connection, ids: &[i64]) {
        let vectors = [
            vec![1.0f32, 0.0, 0.0, 0.0],
            vec![0.9, 0.1, 0.0, 0.0],
            vec![0.0, 1.0, 0.0, 0.0],
            vec![0.0, 0.0, 1.0, 0.0],
            vec![0.8, 0.0, 0.2, 0.0],
        ];

        for (id, vec) in ids.iter().zip(vectors.iter()) {
            insert_vector(conn, *id, vec).unwrap();
        }
    }

    #[test]
    fn test_load_vectors() {
        let conn = setup_db();
        let ids: Vec<i64> = (1..=5).collect();
        insert_test_vectors(&conn, &ids);

        let engine =
            EmbeddingEngine::new("", crate::embed::tokenizer_embed::make_test_tokenizer()).unwrap();
        let mut hybrid = HybridSearch::new(engine);
        hybrid.load_vectors(&conn).unwrap();

        assert_eq!(hybrid.vector_store.len(), 5);
    }

    #[test]
    fn test_search_fts5_only() {
        let conn = setup_db();
        let engine =
            EmbeddingEngine::new("", crate::embed::tokenizer_embed::make_test_tokenizer()).unwrap();
        let mut hybrid = HybridSearch::new(engine);
        hybrid.fts5.build_index(&conn).unwrap();

        let results = hybrid.search_fts5_only(&conn, "rust systems", 5).unwrap();
        assert!(!results.is_empty());

        for r in &results {
            assert!(r.vector_score == 0.0);
            assert!(r.fts_score > 0.0);
        }
    }

    #[test]
    fn test_rrf_fuse_in_hybrid_context() {
        let fts_pairs: Vec<(i64, f64)> = vec![(1, -2.0), (2, -1.5), (5, -1.0)];
        let vec_hits = vec![
            SearchHit {
                memory_id: 2,
                score: 0.95,
            },
            SearchHit {
                memory_id: 5,
                score: 0.70,
            },
            SearchHit {
                memory_id: 3,
                score: 0.50,
            },
        ];

        let fused = rrf_fuse(&fts_pairs, &vec_hits, 5);

        assert!(fused.len() >= 3);
        let ids: Vec<i64> = fused.iter().map(|r| r.memory.id).collect();
        assert!(ids.contains(&2));
        assert!(ids.contains(&5));
        assert!(ids.contains(&1));
    }

    #[test]
    fn test_vector_only_with_preloaded_vectors() {
        let conn = setup_db();
        let ids: Vec<i64> = (1..=5).collect();
        insert_test_vectors(&conn, &ids);

        let engine =
            EmbeddingEngine::new("", crate::embed::tokenizer_embed::make_test_tokenizer()).unwrap();
        let mut hybrid = HybridSearch::new(engine);
        hybrid.load_vectors(&conn).unwrap();

        let query_vec = vec![1.0f32, 0.0, 0.0, 0.0];
        let hits = exact_knn_single(
            &query_vec,
            &hybrid.vector_store.vectors,
            &hybrid.vector_store.ids,
            3,
        );

        assert!(!hits.is_empty());
        assert_eq!(hits[0].memory_id, 1);

        let mut results: Vec<SearchResult> = Vec::new();
        for hit in hits {
            let memory = get_memory(&conn, hit.memory_id).unwrap();
            results.push(SearchResult {
                memory,
                score: hit.score as f64,
                fts_score: 0.0,
                vector_score: hit.score as f64,
                snippet: String::new(),
            });
        }

        assert!(!results.is_empty());
        assert_eq!(results[0].memory.id, 1);
    }

    #[test]
    fn test_build_indices() {
        let conn = setup_db();
        let ids: Vec<i64> = (1..=5).collect();
        insert_test_vectors(&conn, &ids);

        let engine =
            EmbeddingEngine::new("", crate::embed::tokenizer_embed::make_test_tokenizer()).unwrap();
        let mut hybrid = HybridSearch::new(engine);
        hybrid.build_indices(&conn).unwrap();

        assert_eq!(hybrid.fts5.doc_count(), 5);
        assert_eq!(hybrid.vector_store.len(), 5);
    }

    #[test]
    fn test_hybrid_search_without_embed_model() {
        let conn = setup_db();
        let ids: Vec<i64> = (1..=5).collect();
        insert_test_vectors(&conn, &ids);

        let engine =
            EmbeddingEngine::new("", crate::embed::tokenizer_embed::make_test_tokenizer()).unwrap();
        let mut hybrid = HybridSearch::new(engine);
        hybrid.build_indices(&conn).unwrap();

        let fts_results = hybrid.search_fts5_only(&conn, "rust", 3).unwrap();
        assert!(!fts_results.is_empty());
    }
}
