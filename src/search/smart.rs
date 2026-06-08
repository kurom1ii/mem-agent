use rusqlite::Connection;

use crate::core::error::{MemAgentError, Result};
use crate::core::types::{Memory, SearchResult};
use crate::db::ops::{get_epoch_now, list_memories};
use crate::download::ensure_model_downloaded;
use crate::embed::engine::EmbeddingEngine;
use crate::embed::tokenizer_embed::TokenizerWrapper;
use crate::search::fts5_search::Fts5Searcher;
use crate::search::hybrid::HybridSearch;
use crate::text::snippet::generate_snippet;
const HISTORY_HINTS: &[&str] = &[
    "previous chat",
    "chat history",
    "conversation history",
    "last chat",
    "last conversation",
    "earlier chat",
    "what did i say",
    "what did we say",
    "we discussed",
    "lần trước",
    "trước đó",
    "vừa nãy",
    "hồi nãy",
    "khi nãy",
    "đã nói",
];
const CHAT_HINTS: &[&str] = &[
    "chat",
    "conversation",
    "prompt",
    "message",
    "history",
    "assistant",
    "user",
    "lần trước",
    "trước đó",
    "đã nói",
];
const TOOL_HINTS: &[&str] = &[
    "tool",
    "mcp",
    "bash",
    "command",
    "terminal",
    "patch",
    "grep",
    "search_code",
    "github",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    Fts5,
    Vector,
    Hybrid,
}

impl SearchMode {
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "vector" => Self::Vector,
            "hybrid" => Self::Hybrid,
            _ => Self::Fts5,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fts5 => "fts5",
            Self::Vector => "vector",
            Self::Hybrid => "hybrid",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchScope {
    Auto,
    Chat,
    Tool,
    All,
}

impl SearchScope {
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "chat" => Self::Chat,
            "tool" | "tools" => Self::Tool,
            "all" => Self::All,
            _ => Self::Auto,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Chat => "chat",
            Self::Tool => "tool",
            Self::All => "all",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub mode_used: SearchMode,
    pub scope_used: SearchScope,
    pub note: Option<String>,
}

pub fn search_memories_smart(
    conn: &Connection,
    query: &str,
    limit: usize,
    mode: SearchMode,
    scope: SearchScope,
) -> Result<SearchResponse> {
    let clean_query = query.trim();
    if clean_query.is_empty() {
        return Ok(SearchResponse {
            results: Vec::new(),
            mode_used: mode,
            scope_used: scope,
            note: None,
        });
    }

    let resolved_scope = resolve_scope(clean_query, scope);
    if resolved_scope == SearchScope::Chat && looks_like_history_query(clean_query) {
        return Ok(SearchResponse {
            results: recent_as_results(conn, limit, SearchScope::Chat)?,
            mode_used: SearchMode::Fts5,
            scope_used: SearchScope::Chat,
            note: Some("chat-history route".into()),
        });
    }

    let candidate_limit = limit.saturating_mul(5).max(limit);
    let (results, mode_used, note) = execute_base_search(conn, clean_query, candidate_limit, mode)?;
    let filtered = filter_and_rerank(clean_query, resolved_scope, results, limit);

    if filtered.is_empty() && resolved_scope == SearchScope::Chat {
        return Ok(SearchResponse {
            results: recent_as_results(conn, limit, SearchScope::Chat)?,
            mode_used: SearchMode::Fts5,
            scope_used: SearchScope::Chat,
            note: Some("chat-history fallback".into()),
        });
    }

    Ok(SearchResponse {
        results: filtered,
        mode_used,
        scope_used: resolved_scope,
        note,
    })
}

pub fn list_recent_memories_smart(
    conn: &Connection,
    limit: usize,
    scope: SearchScope,
) -> Result<Vec<Memory>> {
    let fetch_limit = limit.saturating_mul(8).max(limit);
    let mut memories = list_memories(conn, fetch_limit)?;
    memories.retain(|mem| matches_scope(&mem.tags, scope));
    memories.truncate(limit);
    Ok(memories)
}

fn execute_base_search(
    conn: &Connection,
    query: &str,
    limit: usize,
    mode: SearchMode,
) -> Result<(Vec<SearchResult>, SearchMode, Option<String>)> {
    match mode {
        SearchMode::Fts5 => {
            let results = build_fts5_searcher(conn)?.search(conn, query, limit)?;
            Ok((results, SearchMode::Fts5, None))
        }
        SearchMode::Vector => match build_hybrid_search(conn) {
            Ok(hybrid) if !hybrid.vector_store.is_empty() => {
                let results = hybrid.search_vector_only(conn, query, limit)?;
                Ok((results, SearchMode::Vector, None))
            }
            Ok(_) => {
                let results = build_fts5_searcher(conn)?.search(conn, query, limit)?;
                Ok((
                    results,
                    SearchMode::Fts5,
                    Some("vector fallback: no vectors indexed".into()),
                ))
            }
            Err(err) => {
                let results = build_fts5_searcher(conn)?.search(conn, query, limit)?;
                Ok((
                    results,
                    SearchMode::Fts5,
                    Some(format!("vector fallback: {err}")),
                ))
            }
        },
        SearchMode::Hybrid => match build_hybrid_search(conn) {
            Ok(hybrid) if !hybrid.vector_store.is_empty() => {
                let results = hybrid.search(conn, query, limit)?;
                Ok((results, SearchMode::Hybrid, None))
            }
            Ok(_) => {
                let results = build_fts5_searcher(conn)?.search(conn, query, limit)?;
                Ok((
                    results,
                    SearchMode::Fts5,
                    Some("hybrid fallback: no vectors indexed".into()),
                ))
            }
            Err(err) => {
                let results = build_fts5_searcher(conn)?.search(conn, query, limit)?;
                Ok((
                    results,
                    SearchMode::Fts5,
                    Some(format!("hybrid fallback: {err}")),
                ))
            }
        },
    }
}

fn resolve_scope(query: &str, requested: SearchScope) -> SearchScope {
    if requested != SearchScope::Auto {
        return requested;
    }

    let lowered = query.to_ascii_lowercase();
    if HISTORY_HINTS.iter().any(|hint| lowered.contains(hint)) {
        return SearchScope::Chat;
    }
    if TOOL_HINTS.iter().any(|hint| lowered.contains(hint)) {
        return SearchScope::Tool;
    }
    SearchScope::Auto
}

fn looks_like_history_query(query: &str) -> bool {
    let lowered = query.to_ascii_lowercase();
    HISTORY_HINTS.iter().any(|hint| lowered.contains(hint))
}

fn looks_like_chat_query(query: &str) -> bool {
    let lowered = query.to_ascii_lowercase();
    CHAT_HINTS.iter().any(|hint| lowered.contains(hint))
}

fn looks_like_tool_query(query: &str) -> bool {
    let lowered = query.to_ascii_lowercase();
    TOOL_HINTS.iter().any(|hint| lowered.contains(hint))
}

fn recent_as_results(
    conn: &Connection,
    limit: usize,
    scope: SearchScope,
) -> Result<Vec<SearchResult>> {
    let memories = list_recent_memories_smart(conn, limit, scope)?;
    Ok(memories
        .into_iter()
        .enumerate()
        .map(|(idx, memory)| SearchResult {
            snippet: preview_text(&memory.content, 220),
            score: 1.0 / (idx as f64 + 1.0),
            fts_score: 0.0,
            vector_score: 0.0,
            memory,
        })
        .collect())
}

fn filter_and_rerank(
    query: &str,
    scope: SearchScope,
    results: Vec<SearchResult>,
    limit: usize,
) -> Vec<SearchResult> {
    let wants_chat = looks_like_chat_query(query);
    let wants_tool = looks_like_tool_query(query);

    let mut ranked: Vec<(f64, SearchResult)> = results
        .into_iter()
        .enumerate()
        .filter(|(_, result)| matches_scope(&result.memory.tags, scope))
        .map(|(idx, mut result)| {
            let mut effective_rank = idx as f64;
            let tags = result.memory.tags.as_str();

            if is_error_memory(tags) && scope != SearchScope::Tool {
                effective_rank += 8.0;
            }
            if scope == SearchScope::Auto {
                if is_tool_memory(tags) && !wants_tool {
                    effective_rank += 4.0;
                }
                if is_chat_memory(tags) && wants_chat {
                    effective_rank -= 2.0;
                }
            }
            if scope == SearchScope::Chat && is_chat_memory(tags) {
                effective_rank -= 4.0;
            }
            if scope == SearchScope::Tool && is_tool_memory(tags) {
                effective_rank -= 4.0;
            }

            effective_rank -= recency_rank_bonus(result.memory.updated_at_epoch);
            let safe_rank = effective_rank.max(0.0);
            result.score = 1.0 / (safe_rank + 1.0);
            (effective_rank, result)
        })
        .collect();

    ranked.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    ranked.truncate(limit);
    ranked.into_iter().map(|(_, result)| result).collect()
}

fn recency_rank_bonus(updated_at_epoch: i64) -> f64 {
    let now = get_epoch_now();
    let age_hours = (now - updated_at_epoch) as f64 / 3600.0;
    if age_hours <= 24.0 {
        3.0
    } else if age_hours <= 168.0 {
        2.0
    } else if age_hours <= 720.0 {
        1.0
    } else {
        0.0
    }
}

fn matches_scope(tags: &str, scope: SearchScope) -> bool {
    match scope {
        SearchScope::Auto | SearchScope::All => true,
        SearchScope::Chat => {
            is_chat_memory(tags) && !is_tool_memory(tags) && !is_error_memory(tags)
        }
        SearchScope::Tool => is_tool_memory(tags),
    }
}

fn is_chat_memory(tags: &str) -> bool {
    has_any_tag(tags, &["prompt", "user", "assistant", "reply", "chat"])
}

fn is_tool_memory(tags: &str) -> bool {
    has_any_tag(tags, &["tool", "mcp"])
}

fn is_error_memory(tags: &str) -> bool {
    has_any_tag(tags, &["error", "failed"])
}

fn has_any_tag(tags: &str, wanted: &[&str]) -> bool {
    tags.split(',')
        .map(|tag| tag.trim())
        .any(|tag| wanted.iter().any(|wanted_tag| wanted_tag == &tag))
}

fn preview_text(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }

    trimmed.chars().take(max_chars).collect::<String>() + "..."
}

fn build_fts5_searcher(conn: &Connection) -> Result<Fts5Searcher> {
    let mut searcher = Fts5Searcher::new();
    searcher.build_index(conn)?;
    Ok(searcher)
}

fn build_hybrid_search(conn: &Connection) -> Result<HybridSearch> {
    let embed = load_embedding_engine()?;
    let mut hybrid = HybridSearch::new(embed);
    hybrid.build_indices(conn)?;
    Ok(hybrid)
}

fn load_embedding_engine() -> Result<EmbeddingEngine> {
    let (model_path, tokenizer_path) = ensure_model_downloaded().map_err(MemAgentError::Config)?;
    let tokenizer = TokenizerWrapper::from_file(&tokenizer_path).map_err(MemAgentError::Config)?;

    EmbeddingEngine::new(&model_path, tokenizer.clone())
        .init()
        .or_else(|gpu_err| {
            EmbeddingEngine::new(&model_path, tokenizer)
                .cpu_only()
                .init()
                .map_err(|cpu_err| {
                    MemAgentError::Config(format!(
                        "GPU init failed: {gpu_err}; CPU fallback failed: {cpu_err}"
                    ))
                })
        })
}

pub fn format_search_results(response: &SearchResponse) -> String {
    if response.results.is_empty() {
        return format!(
            "No results found in {} mode (scope={}).",
            response.mode_used.as_str(),
            response.scope_used.as_str()
        );
    }

    let mut output = String::new();
    output.push_str(&format!(
        "Search mode: {}\nScope: {}\nResults: {}\n",
        response.mode_used.as_str(),
        response.scope_used.as_str(),
        response.results.len()
    ));
    if let Some(note) = &response.note {
        output.push_str(&format!("Note: {note}\n"));
    }
    output.push('\n');

    for result in &response.results {
        let preview = if result.snippet.trim().is_empty() {
            generate_snippet(&result.memory.content, &result.memory.title, 0)
        } else {
            result.snippet.clone()
        };

        output.push_str(&format!(
            "[ID:{}] {} | score={:.4} | fts={:.4} | vector={:.4}\n  tags={}\n  {}\n\n",
            result.memory.id,
            result.memory.title,
            result.score,
            result.fts_score,
            result.vector_score,
            result.memory.tags,
            preview,
        ));
    }

    output.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ops::insert_memory;
    use crate::db::schema::init_db;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        init_db(&conn).unwrap();
        conn
    }

    #[test]
    fn history_query_routes_to_recent_chat_only() {
        let conn = setup();
        let user_id = insert_memory(
            &conn, "prompt", "change", None, "manual", "",
            "user: hi", "hello there", "auto,prompt,user,chat",
            "[]", "[]", "[]", "[]", None,
        )
        .unwrap();
        let tool_id = insert_memory(
            &conn, "observation", "change", None, "manual", "",
            "tool: bash", "ran rg", "auto,tool,bash,success",
            "[]", "[]", "[]", "[]", None,
        )
        .unwrap();
        conn.execute(
            "UPDATE memories SET updated_at_epoch = 1717776000 WHERE id = ?1",
            [user_id],
        )
        .unwrap();
        conn.execute(
            "UPDATE memories SET updated_at_epoch = 1717779600 WHERE id = ?1",
            [tool_id],
        )
        .unwrap();

        let response = search_memories_smart(
            &conn,
            "previous chat history",
            5,
            SearchMode::Hybrid,
            SearchScope::Auto,
        )
        .unwrap();

        assert_eq!(response.scope_used, SearchScope::Chat);
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].memory.id, user_id);
    }

    #[test]
    fn list_recent_chat_filters_tool_rows() {
        let conn = setup();
        insert_memory(
            &conn, "observation", "change", None, "manual", "",
            "tool: github", "search_code", "auto,tool,github,success",
            "[]", "[]", "[]", "[]", None,
        )
        .unwrap();
        let chat_id = insert_memory(
            &conn, "observation", "change", None, "manual", "",
            "assistant: answer", "here you go", "auto,assistant,reply,chat",
            "[]", "[]", "[]", "[]", None,
        )
        .unwrap();

        let recent = list_recent_memories_smart(&conn, 5, SearchScope::Chat).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].id, chat_id);
    }

    #[test]
    fn scope_parser_handles_aliases() {
        assert_eq!(SearchScope::parse("tools"), SearchScope::Tool);
        assert_eq!(SearchScope::parse("chat"), SearchScope::Chat);
        assert_eq!(SearchScope::parse("weird"), SearchScope::Auto);
    }
}
