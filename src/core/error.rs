use std::{fmt, io};
use rusqlite;

pub type Result<T> = std::result::Result<T, MemAgentError>;

#[derive(Debug)]
pub enum MemAgentError {
    Db(rusqlite::Error),
    Io(io::Error),
    Json(serde_json::Error),
    Bincode(String),
    NotExist(String),
    AlreadyExists(String),
    Embed(String),
    Search(String),
    Dimension(String),
    Config(String),
    Serialize(String),
}

impl fmt::Display for MemAgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Db(e) => write!(f, "Database error: {e}"),
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::Json(e) => write!(f, "JSON error: {e}"),
            Self::Bincode(e) => write!(f, "Bincode error: {e}"),
            Self::NotExist(msg) => write!(f, "Not found: {msg}"),
            Self::AlreadyExists(msg) => write!(f, "Already exists: {msg}"),
            Self::Embed(msg) => write!(f, "Embedding error: {msg}"),
            Self::Search(msg) => write!(f, "Search error: {msg}"),
            Self::Dimension(msg) => write!(f, "Dimension error: {msg}"),
            Self::Config(msg) => write!(f, "Config error: {msg}"),
            Self::Serialize(msg) => write!(f, "Serialization error: {msg}"),
        }
    }
}

impl std::error::Error for MemAgentError {}

impl From<rusqlite::Error> for MemAgentError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Db(e)
    }
}

impl From<io::Error> for MemAgentError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for MemAgentError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
