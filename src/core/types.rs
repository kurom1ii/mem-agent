use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub tags: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub memory: Memory,
    pub score: f64,
    pub fts_score: f64,
    pub vector_score: f64,
    pub snippet: String,
}

#[derive(Debug, Clone)]
pub struct VectorMeta {
    pub memory_id: i64,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub memory_id: i64,
    pub score: f32,
}

#[derive(Debug, Clone)]
pub struct VectorEntry {
    pub memory_id: i64,
    pub vector: Vec<f32>,
}
