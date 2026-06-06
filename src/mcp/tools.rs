use crate::mcp::protocol::{
    CallToolParams, JsonRpcResponse, ToolDefinition,
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
    let mode = args["mode"].as_str().unwrap_or("hybrid");

    match mode {
        "fts5" => {
            let results = crate::db::fts::fts5_search_raw(conn, query, limit)
                .map_err(|e| format!("FTS5 search error: {e}"))?;
            let mut output = String::new();
            for (id, score) in &results {
                match crate::db::ops::get_memory(conn, *id) {
                    Ok(mem) => {
                        output.push_str(&format!(
                            "[ID:{}] {} (score: {:.4})\n  {}\n\n",
                            mem.id, mem.title, score,
                            &mem.content[..mem.content.len().min(200)],
                        ));
                    }
                    Err(_) => {
                        output.push_str(&format!("[ID:{}] (score: {:.4})\n\n", id, score));
                    }
                }
            }
            Ok(output.trim().to_string())
        }
        _ => {
            let results = crate::db::fts::fts5_search_raw(conn, query, limit)
                .map_err(|e| format!("Search error: {e}"))?;
            let mut output = String::new();
            for (id, score) in &results {
                match crate::db::ops::get_memory(conn, *id) {
                    Ok(mem) => {
                        output.push_str(&format!(
                            "[ID:{}] {} (score: {:.4})\n  {}\n\n",
                            mem.id, mem.title, score,
                            &mem.content[..mem.content.len().min(200)],
                        ));
                    }
                    Err(_) => {
                        output.push_str(&format!("[ID:{}] (score: {:.4})\n\n", id, score));
                    }
                }
            }
            Ok(output.trim().to_string())
        }
    }
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

    let id = crate::db::ops::insert_memory(conn, title, content, tags)
        .map_err(|e| format!("Insert error: {e}"))?;
    Ok(format!("Memory added with ID: {id}"))
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

    let mem = crate::db::ops::get_memory(conn, id)
        .map_err(|e| format!("Get error: {e}"))?;
    Ok(serde_json::to_string_pretty(&mem).unwrap_or_else(|_| format!("{mem:?}")))
}

fn handle_memory_list(
    params: &CallToolParams,
    conn: &rusqlite::Connection,
) -> Result<String, String> {
    let default_args = serde_json::json!({});
    let args = params.arguments.as_ref().unwrap_or(&default_args);
    let limit = args["limit"].as_u64().unwrap_or(20) as usize;

    let mems = crate::db::ops::list_memories(conn, limit)
        .map_err(|e| format!("List error: {e}"))?;
    let mut output = String::new();
    for mem in &mems {
        output.push_str(&format!(
            "[ID:{}] {} | tags: {} | updated: {}\n",
            mem.id, mem.title, mem.tags, mem.updated_at
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

    let deleted = crate::db::ops::delete_memory(conn, id)
        .map_err(|e| format!("Delete error: {e}"))?;
    if deleted {
        Ok(format!("Memory {id} deleted"))
    } else {
        Ok(format!("Memory {id} not found"))
    }
}

fn handle_index_stats(conn: &rusqlite::Connection) -> Result<String, String> {
    let stats = crate::db::ops::get_stats(conn)
        .map_err(|e| format!("Stats error: {e}"))?;
    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "memory_count": stats.memory_count,
        "vector_count": stats.vector_count,
    }))
    .unwrap_or_else(|_| format!("{stats:?}")))
}
