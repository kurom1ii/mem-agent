use crate::embed::engine::EmbeddingEngine;
use crate::embed::tokenizer_embed::TokenizerWrapper;
use crate::mcp::protocol::{CallToolParams, JsonRpcResponse, ToolDefinition};
use crate::search::smart::{
    format_search_results, list_recent_memories_smart, search_memories_smart, SearchMode,
    SearchScope,
};

pub fn list_tools_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "memory_search".into(),
            description: "Hybrid search BM25 + vector semantic over memory store".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query text"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max results (default 10)",
                        "default": 10
                    },
                    "mode": {
                        "type": "string",
                        "description": "Search mode: hybrid, fts5, or vector",
                        "enum": ["hybrid", "fts5", "vector"],
                        "default": "hybrid"
                    },
                    "scope": {
                        "type": "string",
                        "description": "Result scope: auto, chat, tool, or all",
                        "enum": ["auto", "chat", "tool", "all"],
                        "default": "auto"
                    }
                },
                "required": ["query"]
            }),
        },
        ToolDefinition {
            name: "memory_add".into(),
            description: "Add a new memory entry to the store".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Memory title"
                    },
                    "content": {
                        "type": "string",
                        "description": "Memory content body"
                    },
                    "tags": {
                        "type": "string",
                        "description": "Comma-separated tags"
                    },
                    "kind": {
                        "type": "string",
                        "description": "Memory kind: observation, summary, prompt, manual",
                        "enum": ["observation", "summary", "prompt", "manual"],
                        "default": "manual"
                    },
                    "type": {
                        "type": "string",
                        "description": "Observation type: bugfix, feature, refactor, change, discovery, decision, security_alert, security_note",
                        "enum": ["bugfix", "feature", "refactor", "change", "discovery", "decision", "security_alert", "security_note"],
                        "default": "change"
                    },
                    "session_id": {
                        "type": "string",
                        "description": "Session ID for grouping"
                    },
                    "source": {
                        "type": "string",
                        "description": "Memory source: opencode, claude, api, manual",
                        "default": "manual"
                    },
                    "project": {
                        "type": "string",
                        "description": "Project path",
                        "default": ""
                    }
                },
                "required": ["title", "content"]
            }),
        },
        ToolDefinition {
            name: "memory_get".into(),
            description: "Get a memory entry by ID".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "integer",
                        "description": "Memory ID"
                    }
                },
                "required": ["id"]
            }),
        },
        ToolDefinition {
            name: "memory_list".into(),
            description: "List recent memory entries".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "description": "Max results (default 20)",
                        "default": 20
                    },
                    "scope": {
                        "type": "string",
                        "description": "List scope: all, chat, or tool",
                        "enum": ["all", "chat", "tool"],
                        "default": "all"
                    }
                }
            }),
        },
        ToolDefinition {
            name: "memory_delete".into(),
            description: "Delete a memory entry by ID".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "integer",
                        "description": "Memory ID to delete"
                    }
                },
                "required": ["id"]
            }),
        },
        ToolDefinition {
            name: "index_stats".into(),
            description: "Get memory index statistics".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        ToolDefinition {
            name: "memory_observe".into(),
            description: "Classify a tool execution using the observer pipeline and store".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "kind": {
                        "type": "string",
                        "description": "Memory kind (default: observation)",
                        "default": "observation"
                    },
                    "tool_name": {
                        "type": "string",
                        "description": "Name of the tool that was executed"
                    },
                    "tool_input": {
                        "type": "string",
                        "description": "Tool input/arguments JSON"
                    },
                    "tool_output": {
                        "type": "string",
                        "description": "Tool output/result"
                    },
                    "session_id": {
                        "type": "string",
                        "description": "Session ID for grouping"
                    },
                    "source": {
                        "type": "string",
                        "description": "Memory source",
                        "default": "opencode"
                    },
                    "project": {
                        "type": "string",
                        "description": "Project path",
                        "default": ""
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Working directory",
                        "default": "."
                    },
                    "user_prompt": {
                        "type": "string",
                        "description": "User prompt that triggered this"
                    },
                    "tags": {
                        "type": "string",
                        "description": "Comma-separated tags"
                    },
                    "files_read": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Files read during tool execution"
                    },
                    "files_modified": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Files modified during tool execution"
                    }
                },
                "required": ["tool_name", "tool_output"]
            }),
        },
        ToolDefinition {
            name: "memory_summarize".into(),
            description: "Summarize a session using the observer pipeline".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Session ID to summarize"
                    },
                    "project": {
                        "type": "string",
                        "description": "Project path",
                        "default": ""
                    },
                    "source": {
                        "type": "string",
                        "description": "Source platform",
                        "default": "opencode"
                    }
                },
                "required": ["session_id"]
            }),
        },
    ]
}

pub fn handle_tool_call(
    params: &CallToolParams,
    conn: &rusqlite::Connection,
) -> Result<JsonRpcResponse, String> {
    let result = match params.name.as_str() {
        "memory_search" => handle_memory_search(params, conn),
        "memory_add" => handle_memory_add(params, conn),
        "memory_get" => handle_memory_get(params, conn),
        "memory_list" => handle_memory_list(params, conn),
        "memory_delete" => handle_memory_delete(params, conn),
        "memory_observe" => handle_memory_observe(params, conn),
        "memory_summarize" => handle_memory_summarize(params, conn),
        "index_stats" => handle_index_stats(conn),
        name => Err(format!("Unknown tool: {name}")),
    };

    match result {
        Ok(text) => Ok(JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: None,
            result: Some(serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": text
                }],
                "isError": false
            })),
            error: None,
        }),
        Err(e) => Ok(JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: None,
            result: Some(serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": format!("Error: {e}")
                }],
                "isError": true
            })),
            error: None,
        }),
    }
}

fn handle_memory_search(
    params: &CallToolParams,
    conn: &rusqlite::Connection,
) -> Result<String, String> {
    let default_args = serde_json::json!({});
    let args = params.arguments.as_ref().unwrap_or(&default_args);
    let query = args["query"]
        .as_str()
        .ok_or_else(|| "Missing 'query' parameter".to_string())?;
    let limit = args["limit"].as_u64().unwrap_or(10) as usize;
    let mode = SearchMode::parse(args["mode"].as_str().unwrap_or("hybrid"));
    let scope = SearchScope::parse(args["scope"].as_str().unwrap_or("auto"));

    let response = search_memories_smart(conn, query, limit, mode, scope)
        .map_err(|e| format!("Search error: {e}"))?;
    Ok(format_search_results(&response))
}

fn handle_memory_add(
    params: &CallToolParams,
    conn: &rusqlite::Connection,
) -> Result<String, String> {
    let default_args = serde_json::json!({});
    let args = params.arguments.as_ref().unwrap_or(&default_args);
    let title = args["title"]
        .as_str()
        .ok_or_else(|| "Missing 'title'".to_string())?;
    let content = args["content"]
        .as_str()
        .ok_or_else(|| "Missing 'content'".to_string())?;
    let tags = args["tags"].as_str().unwrap_or("");
    let kind = args["kind"].as_str().unwrap_or("manual");
    let obs_type = args["type"].as_str().unwrap_or("change");
    let session_id = args["session_id"].as_str();
    let source = args["source"].as_str().unwrap_or("manual");
    let project = args["project"].as_str().unwrap_or("");

    if let Some(sid) = session_id {
        crate::db::ops::ensure_session(conn, sid, project, source)
            .map_err(|e| format!("Session error: {e}"))?;
    }

    let id = crate::db::ops::insert_memory(
        conn, kind, obs_type, session_id, source, project,
        title, content, tags,
        "[]", "[]", "[]", "[]", None,
    )
    .map_err(|e| format!("Insert error: {e}"))?;

    let combined = format!("title: {title}\ncontent: {content}");
    let vector_status = match embed_memory_document(&combined) {
        Ok(vector) => match crate::db::ops::insert_vector(conn, id, &vector) {
            Ok(()) => "vector indexed".to_string(),
            Err(e) => format!("memory stored, vector insert failed: {e}"),
        },
        Err(e) => format!("memory stored, vector skipped: {e}"),
    };

    Ok(format!("Memory added with ID: {id} ({vector_status})"))
}

fn handle_memory_get(
    params: &CallToolParams,
    conn: &rusqlite::Connection,
) -> Result<String, String> {
    let default_args = serde_json::json!({});
    let args = params.arguments.as_ref().unwrap_or(&default_args);
    let id = args["id"]
        .as_i64()
        .ok_or_else(|| "Missing 'id'".to_string())?;

    let mem = crate::db::ops::get_memory(conn, id).map_err(|e| format!("Get error: {e}"))?;
    Ok(serde_json::to_string_pretty(&mem).unwrap_or_else(|_| format!("{mem:?}")))
}

fn handle_memory_list(
    params: &CallToolParams,
    conn: &rusqlite::Connection,
) -> Result<String, String> {
    let default_args = serde_json::json!({});
    let args = params.arguments.as_ref().unwrap_or(&default_args);
    let limit = args["limit"].as_u64().unwrap_or(20) as usize;
    let scope = SearchScope::parse(args["scope"].as_str().unwrap_or("all"));

    let mems =
        list_recent_memories_smart(conn, limit, scope).map_err(|e| format!("List error: {e}"))?;

    let mut output = String::new();
    for mem in &mems {
        output.push_str(&format!(
            "[ID:{}] {} [kind={}] [type={}] | tags: {} | session: {}\n",
            mem.id,
            mem.title,
            mem.kind,
            mem.observation_type,
            mem.tags,
            mem.session_id.as_deref().unwrap_or("-"),
        ));
    }
    Ok(output.trim().to_string())
}

fn handle_memory_delete(
    params: &CallToolParams,
    conn: &rusqlite::Connection,
) -> Result<String, String> {
    let default_args = serde_json::json!({});
    let args = params.arguments.as_ref().unwrap_or(&default_args);
    let id = args["id"]
        .as_i64()
        .ok_or_else(|| "Missing 'id'".to_string())?;

    let deleted =
        crate::db::ops::delete_memory(conn, id).map_err(|e| format!("Delete error: {e}"))?;
    if deleted {
        Ok(format!("Memory {id} deleted"))
    } else {
        Ok(format!("Memory {id} not found"))
    }
}

fn handle_memory_observe(
    params: &CallToolParams,
    conn: &rusqlite::Connection,
) -> Result<String, String> {
    let default_args = serde_json::json!({});
    let args = params.arguments.as_ref().unwrap_or(&default_args);
    let kind = args["kind"].as_str().unwrap_or("observation");
    let tool_name = args["tool_name"]
        .as_str()
        .ok_or_else(|| "Missing 'tool_name'".to_string())?;
    let tool_input = args["tool_input"].as_str().unwrap_or("");
    let tool_output = args["tool_output"]
        .as_str()
        .ok_or_else(|| "Missing 'tool_output'".to_string())?;
    let session_id = args["session_id"].as_str();
    let source = args["source"].as_str().unwrap_or("opencode");
    let project = args["project"].as_str().unwrap_or("");
    let cwd = args["cwd"].as_str().unwrap_or(".");
    let user_prompt = args["user_prompt"].as_str();
    let tags = args["tags"].as_str().unwrap_or("");

    let files_read: Vec<String> = args["files_read"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let files_modified: Vec<String> = args["files_modified"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let now = crate::db::ops::get_epoch_now();
    let timestamp = chrono::DateTime::from_timestamp(now, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let result = crate::observer::observe_and_store(
        conn, kind, session_id, source, project,
        tool_name, tool_input, tool_output, cwd, &timestamp,
        user_prompt, tags,
        &files_read, &files_modified,
    )
    .map_err(|e| format!("Observer error: {e}"))?;

    let combined = format!("title: {tool_name}\ncontent: {tool_input}");
    let vector_status = match embed_memory_document(&combined) {
        Ok(vector) => match crate::db::ops::insert_vector(conn, result, &vector) {
            Ok(()) => "vector indexed".to_string(),
            Err(e) => format!("vector insert failed: {e}"),
        },
        Err(e) => format!("vector skipped: {e}"),
    };

    Ok(format!(
        "Observed and stored: memory ID {result} ({vector_status})"
    ))
}

fn handle_memory_summarize(
    params: &CallToolParams,
    conn: &rusqlite::Connection,
) -> Result<String, String> {
    let default_args = serde_json::json!({});
    let args = params.arguments.as_ref().unwrap_or(&default_args);
    let session_id = args["session_id"]
        .as_str()
        .ok_or_else(|| "Missing 'session_id'".to_string())?;
    let project = args["project"].as_str().unwrap_or("");
    let source = args["source"].as_str().unwrap_or("opencode");

    match crate::observer::summarize_session(conn, session_id, project, source)
        .map_err(|e| format!("Summarize error: {e}"))?
    {
        Some(id) => Ok(format!("Summary stored: memory ID {id}")),
        None => Ok("No memories to summarize in this session".into()),
    }
}

fn handle_index_stats(conn: &rusqlite::Connection) -> Result<String, String> {
    let stats = crate::db::ops::get_stats(conn).map_err(|e| format!("Stats error: {e}"))?;
    let by_kind = crate::db::ops::get_stats_by_kind(conn).unwrap_or_default();
    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "memory_count": stats.memory_count,
        "vector_count": stats.vector_count,
        "session_count": stats.session_count,
        "by_kind": by_kind.into_iter().map(|(k, c)| serde_json::json!({ "kind": k, "count": c })).collect::<Vec<_>>(),
    }))
    .unwrap_or_else(|_| format!("{stats:?}")))
}

fn load_embedding_engine() -> Result<EmbeddingEngine, String> {
    let (model_path, tokenizer_path) =
        crate::download::ensure_model_downloaded().map_err(|e| format!("Model download: {e}"))?;
    let tokenizer =
        TokenizerWrapper::from_file(&tokenizer_path).map_err(|e| format!("Tokenizer load: {e}"))?;

    EmbeddingEngine::new(&model_path, tokenizer.clone())
        .init()
        .or_else(|gpu_err| {
            EmbeddingEngine::new(&model_path, tokenizer)
                .cpu_only()
                .init()
                .map_err(|cpu_err| {
                    format!("GPU init failed: {gpu_err}; CPU fallback failed: {cpu_err}")
                })
        })
}

fn embed_memory_document(text: &str) -> Result<Vec<f32>, String> {
    load_embedding_engine()?
        .embed_document(text)
        .map_err(|e| format!("Embedding failed: {e}"))
}
