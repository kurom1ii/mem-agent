use crate::core::types::SearchResult;
use crate::embed::engine::EmbeddingEngine;
use crate::embed::tokenizer_embed::TokenizerWrapper;
use crate::mcp::protocol::{CallToolParams, JsonRpcResponse, ToolDefinition};
use crate::search::fts5_search::Fts5Searcher;
use crate::search::hybrid::HybridSearch;

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
            let results = build_fts5_searcher(conn)?
                .search(conn, query, limit)
                .map_err(|e| format!("FTS5 search error: {e}"))?;
            Ok(format_search_results("fts5", &results))
        }
        "vector" => {
            let hybrid = build_hybrid_search(conn)?;
            if hybrid.vector_store.is_empty() {
                return Ok(
                    "No vectors indexed yet. Save memories after model setup or use mode=\"fts5\"."
                        .to_string(),
                );
            }
            let results = hybrid
                .search_vector_only(conn, query, limit)
                .map_err(|e| format!("Vector search error: {e}"))?;
            Ok(format_search_results("vector", &results))
        }
        "hybrid" => match build_hybrid_search(conn) {
            Ok(hybrid) => {
                if hybrid.vector_store.is_empty() {
                    let results = build_fts5_searcher(conn)?
                        .search(conn, query, limit)
                        .map_err(|e| format!("FTS5 fallback error: {e}"))?;
                    Ok(format!(
                        "Hybrid fallback: no vectors indexed yet.\n\n{}",
                        format_search_results("fts5", &results)
                    ))
                } else {
                    let results = hybrid
                        .search(conn, query, limit)
                        .map_err(|e| format!("Hybrid search error: {e}"))?;
                    Ok(format_search_results("hybrid", &results))
                }
            }
            Err(err) => {
                let results = build_fts5_searcher(conn)?
                    .search(conn, query, limit)
                    .map_err(|e| format!("FTS5 fallback error: {e}"))?;
                Ok(format!(
                    "Hybrid fallback: {err}\n\n{}",
                    format_search_results("fts5", &results)
                ))
            }
        },
        other => Err(format!(
            "Unsupported mode '{other}'. Use one of: fts5, vector, hybrid."
        )),
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

    let mems =
        crate::db::ops::list_memories(conn, limit).map_err(|e| format!("List error: {e}"))?;
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

    let deleted =
        crate::db::ops::delete_memory(conn, id).map_err(|e| format!("Delete error: {e}"))?;
    if deleted {
        Ok(format!("Memory {id} deleted"))
    } else {
        Ok(format!("Memory {id} not found"))
    }
}

fn handle_index_stats(conn: &rusqlite::Connection) -> Result<String, String> {
    let stats = crate::db::ops::get_stats(conn).map_err(|e| format!("Stats error: {e}"))?;
    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "memory_count": stats.memory_count,
        "vector_count": stats.vector_count,
    }))
    .unwrap_or_else(|_| format!("{stats:?}")))
}

fn build_fts5_searcher(conn: &rusqlite::Connection) -> Result<Fts5Searcher, String> {
    let mut searcher = Fts5Searcher::new();
    searcher
        .build_index(conn)
        .map_err(|e| format!("BM25 index build error: {e}"))?;
    Ok(searcher)
}

fn build_hybrid_search(conn: &rusqlite::Connection) -> Result<HybridSearch, String> {
    let embed = load_embedding_engine()?;
    let mut hybrid = HybridSearch::new(embed);
    hybrid
        .build_indices(conn)
        .map_err(|e| format!("Index build error: {e}"))?;
    Ok(hybrid)
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

fn format_search_results(mode: &str, results: &[SearchResult]) -> String {
    if results.is_empty() {
        return format!("No results found in {mode} mode.");
    }

    let mut output = String::new();
    output.push_str(&format!(
        "Search mode: {mode}\nResults: {}\n\n",
        results.len()
    ));

    for result in results {
        let preview = if result.snippet.trim().is_empty() {
            result.memory.content.chars().take(200).collect::<String>()
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
