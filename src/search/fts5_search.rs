use std::collections::HashMap;

use crate::core::error::Result;
use crate::core::types::SearchResult;
use crate::db::fts::fts5_search_raw;
use crate::db::ops::{get_memory, list_memories};
use crate::text::bm25::Bm25Index;
use crate::text::snippet::generate_snippet;
use crate::text::tokenizer::tokenize;

pub struct Fts5Searcher {
    bm25: Bm25Index,
    id_to_idx: HashMap<i64, usize>,
    idx_to_id: HashMap<usize, i64>,
}

impl Fts5Searcher {
    pub fn new() -> Self {
        Self {
            bm25: Bm25Index::new(),
            id_to_idx: HashMap::new(),
            idx_to_id: HashMap::new(),
        }
    }

    /// Build BM25 index from all memories in the database.
    pub fn build_index(&mut self, conn: &rusqlite::Connection) -> Result<()> {
        self.bm25 = Bm25Index::new();
        self.id_to_idx.clear();
        self.idx_to_id.clear();

        let memories = list_memories(conn, usize::MAX)?;

        if memories.is_empty() {
            return Ok(());
        }

        for (idx, mem) in memories.iter().enumerate() {
            self.id_to_idx.insert(mem.id, idx);
            self.idx_to_id.insert(idx, mem.id);

            let content_tokens = tokenize(&mem.content);
            let mut all_tokens = content_tokens;
            let title_tokens = tokenize(&mem.title);
            all_tokens.extend(title_tokens);
            self.bm25.add_document(idx, &all_tokens);
        }

        Ok(())
    }

    /// FTS5 pre-filter then BM25 re-rank. Returns top `limit` SearchResults.
    pub fn search(
        &self,
        conn: &rusqlite::Connection,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let query_tokens = tokenize(query);

        if query_tokens.is_empty() || self.id_to_idx.is_empty() {
            return Ok(Vec::new());
        }

        let candidates = fts5_search_raw(conn, query, limit * 3)?;

        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        let mut scored: Vec<(i64, f64)> = candidates
            .iter()
            .filter_map(|(id, _fts5_score)| {
                let bm25_idx = self.id_to_idx.get(id)?;
                let bm25_score = self.bm25.score(*bm25_idx, &query_tokens);
                if bm25_score > 0.0 {
                    Some((*id, bm25_score))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        let mut results = Vec::with_capacity(scored.len());
        for (id, score) in scored {
            let memory = get_memory(conn, id)?;
            let snippet = generate_snippet(&memory.content, query, 0);
            results.push(SearchResult {
                memory,
                score,
                fts_score: score,
                vector_score: 0.0,
                snippet,
            });
        }

        Ok(results)
    }

    /// Number of indexed documents.
    pub fn doc_count(&self) -> usize {
        self.id_to_idx.len()
    }

    /// BM25 document index for a given memory ID.
    pub fn bm25_idx(&self, memory_id: i64) -> Option<usize> {
        self.id_to_idx.get(&memory_id).copied()
    }
}

impl Default for Fts5Searcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ops::insert_memory;
    use crate::db::schema::init_db;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        init_db(&conn).unwrap();

        let entries = [
            (
                "Rust Programming",
                "Rust is a systems programming language with zero cost abstractions",
                "rust,systems",
            ),
            (
                "Python Data Science",
                "Python is great for data science and machine learning workflows",
                "python,data",
            ),
            (
                "Async Rust",
                "Rust async programming with tokio runtime for concurrent applications",
                "rust,async",
            ),
            (
                "JavaScript Ecosystem",
                "JavaScript runs in the browser and on the server with Node.js",
                "javascript,web",
            ),
            (
                "Memory Management",
                "Understanding memory management in Rust and other systems languages",
                "rust,memory",
            ),
        ];

        for (title, content, tags) in entries {
            insert_memory(&conn, title, content, tags).unwrap();
        }

        conn
    }

    #[test]
    fn test_build_index_and_doc_count() {
        let conn = setup_db();
        let mut searcher = Fts5Searcher::new();
        searcher.build_index(&conn).unwrap();
        assert_eq!(searcher.doc_count(), 5);
    }

    #[test]
    fn test_search_returns_results() {
        let conn = setup_db();
        let mut searcher = Fts5Searcher::new();
        searcher.build_index(&conn).unwrap();

        let results = searcher
            .search(&conn, "rust systems programming", 5)
            .unwrap();
        assert!(!results.is_empty());

        let titles: Vec<&str> = results.iter().map(|r| r.memory.title.as_str()).collect();
        assert!(titles.contains(&"Rust Programming"));
    }

    #[test]
    fn test_search_no_match() {
        let conn = setup_db();
        let mut searcher = Fts5Searcher::new();
        searcher.build_index(&conn).unwrap();

        let results = searcher.search(&conn, "xyznonexistent123", 5).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_limit() {
        let conn = setup_db();
        let mut searcher = Fts5Searcher::new();
        searcher.build_index(&conn).unwrap();

        let results = searcher.search(&conn, "programming", 2).unwrap();
        assert!(results.len() <= 2);
    }

    #[test]
    fn test_search_score_ordering() {
        let conn = setup_db();
        let mut searcher = Fts5Searcher::new();
        searcher.build_index(&conn).unwrap();

        let results = searcher.search(&conn, "rust", 5).unwrap();
        for i in 1..results.len() {
            assert!(results[i - 1].score >= results[i].score);
        }
    }

    #[test]
    fn test_empty_index_search() {
        let conn = setup_db();
        let searcher = Fts5Searcher::new();
        let results = searcher.search(&conn, "anything", 5).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_bm25_idx_mapping() {
        let conn = setup_db();
        let mut searcher = Fts5Searcher::new();
        searcher.build_index(&conn).unwrap();

        let memories = list_memories(&conn, 5).unwrap();
        for mem in &memories {
            let idx = searcher.bm25_idx(mem.id).unwrap();
            assert_eq!(searcher.idx_to_id[&idx], mem.id);
        }
    }
}
