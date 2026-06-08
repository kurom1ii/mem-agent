use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryKind {
    #[serde(rename = "observation")]
    Observation,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "prompt")]
    Prompt,
    #[serde(rename = "manual")]
    Manual,
}

impl MemoryKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Observation => "observation",
            Self::Summary => "summary",
            Self::Prompt => "prompt",
            Self::Manual => "manual",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "summary" => Self::Summary,
            "prompt" => Self::Prompt,
            "manual" => Self::Manual,
            _ => Self::Observation,
        }
    }
}

impl std::fmt::Display for MemoryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: i64,
    pub kind: String,            // observation|summary|prompt|manual
    pub observation_type: String, // bugfix|feature|discovery|change|...
    pub session_id: Option<String>,
    pub source: String,          // opencode|claude|api|manual
    pub project: String,
    pub title: String,
    pub content: String,
    pub tags: String,
    pub facts: String,           // JSON array: ["fact1","fact2"]
    pub concepts: String,        // JSON array: ["concept1","concept2"]
    pub files_read: String,      // JSON array: ["path1","path2"]
    pub files_modified: String,  // JSON array: ["path1","path2"]
    pub narrative: Option<String>,
    pub created_at_epoch: i64,
    pub updated_at_epoch: i64,
}

impl Memory {
    pub fn is_observation(&self) -> bool {
        self.kind == "observation"
    }
    pub fn is_summary(&self) -> bool {
        self.kind == "summary"
    }
    pub fn is_prompt(&self) -> bool {
        self.kind == "prompt"
    }
    pub fn is_manual(&self) -> bool {
        self.kind == "manual"
    }
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
    pub created_at_epoch: i64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub project: String,
    pub source: String,
    pub title: String,
    pub status: String,          // active|completed|failed
    pub created_at_epoch: i64,
    pub updated_at_epoch: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationType {
    pub id: String,
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationConcept {
    pub id: String,
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeConfig {
    pub name: String,
    pub description: String,
    pub version: String,
    pub observation_types: Vec<ObservationType>,
    pub observation_concepts: Vec<ObservationConcept>,
}
