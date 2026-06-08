pub mod classifier;
pub mod client;
pub mod config;
pub mod prompt;

use rusqlite::Connection;

use crate::core::error::Result;
use crate::db::ops;
use crate::observer::classifier::classify_tool_fallback;
use crate::observer::client::ObserverClient;

pub fn observe_and_store(
    conn: &Connection,
    kind: &str,
    session_id: Option<&str>,
    source: &str,
    project: &str,
    tool_name: &str,
    tool_input: &str,
    tool_output: &str,
    cwd: &str,
    timestamp: &str,
    user_prompt: Option<&str>,
    tags: &str,
    files_read: &[String],
    files_modified: &[String],
) -> Result<i64> {
    let output = if let Some(client) = ObserverClient::from_env_or_config() {
        client.classify_observation(
            tool_name, tool_input, tool_output, cwd, timestamp, user_prompt,
        )
    } else {
        None
    };

    let classified = match output {
        Some(o) => o,
        None => {
            let mut fallback = classify_tool_fallback(tool_name, tool_input, tool_output);
            if !files_read.is_empty() {
                fallback.files_read = files_read.to_vec();
            }
            if !files_modified.is_empty() {
                fallback.files_modified = files_modified.to_vec();
            }
            fallback
        }
    };

    let facts_json =
        serde_json::to_string(&classified.facts).unwrap_or_else(|_| "[]".to_string());
    let concepts_json =
        serde_json::to_string(&classified.concepts).unwrap_or_else(|_| "[]".to_string());
    let files_read_json = if classified.files_read.is_empty() {
        serde_json::to_string(files_read).unwrap_or_else(|_| "[]".to_string())
    } else {
        serde_json::to_string(&classified.files_read).unwrap_or_else(|_| "[]".to_string())
    };
    let files_modified_json = if classified.files_modified.is_empty() {
        serde_json::to_string(files_modified).unwrap_or_else(|_| "[]".to_string())
    } else {
        serde_json::to_string(&classified.files_modified)
            .unwrap_or_else(|_| "[]".to_string())
    };

    if let Some(sid) = session_id {
        let _ = ops::ensure_session(conn, sid, project, source);
    }

    ops::insert_memory(
        conn,
        kind,
        &classified.observation_type,
        session_id,
        source,
        project,
        &classified.title,
        &format!(
            "tool: {tool_name}\ninput: {tool_input}\n\n{tool_output}"
        ),
        tags,
        &facts_json,
        &concepts_json,
        &files_read_json,
        &files_modified_json,
        Some(&classified.narrative),
    )
}

pub fn summarize_session(
    conn: &Connection,
    session_id: &str,
    project: &str,
    source: &str,
) -> Result<Option<i64>> {
    let memories = ops::list_memories_by_session(conn, session_id)?;
    if memories.is_empty() {
        return Ok(None);
    }

    let title = &memories
        .first()
        .map(|m| m.title.clone())
        .unwrap_or_else(|| "Session".to_string());

    let summary_content = if let Some(client) = ObserverClient::from_env_or_config() {
        client.summarize_session(title, memories.len())
    } else {
        None
    };

    let content = match summary_content {
        Some(c) => c,
        None => {
            let titles: Vec<String> =
                memories.iter().map(|m| format!("- {}", m.title)).collect();
            format!("Session summary:\n{}", titles.join("\n"))
        }
    };

    let id = ops::insert_memory(
        conn,
        "summary",
        "change",
        Some(session_id),
        source,
        project,
        &format!("Summary: {title}"),
        &content,
        "auto,summary",
        "[]",
        "[]",
        "[]",
        "[]",
        None,
    )?;

    let _ = ops::update_session_status(conn, session_id, "completed");

    Ok(Some(id))
}

pub struct IngestInput<'a> {
    pub kind: &'a str,
    pub session_id: Option<&'a str>,
    pub source: &'a str,
    pub project: &'a str,
    pub tool_name: &'a str,
    pub tool_input: &'a str,
    pub tool_output: &'a str,
    pub cwd: &'a str,
    pub user_prompt: Option<&'a str>,
    pub tags: &'a str,
    pub files_read: &'a [String],
    pub files_modified: &'a [String],
}

pub fn ingest(conn: &Connection, input: IngestInput) -> Result<i64> {
    let now = ops::get_epoch_now();
    let timestamp = chrono::DateTime::from_timestamp(now, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "unknown".to_string());

    observe_and_store(
        conn,
        input.kind,
        input.session_id,
        input.source,
        input.project,
        input.tool_name,
        input.tool_input,
        input.tool_output,
        input.cwd,
        &timestamp,
        input.user_prompt,
        input.tags,
        input.files_read,
        input.files_modified,
    )
}
