use std::io::{self, BufRead, Write};

use crate::core::error::MemAgentError;
use crate::mcp::protocol::{
    InitializeResult, JsonRpcRequest, JsonRpcResponse, ListToolsResult, ServerCapabilities,
    ServerInfo, PROTOCOL_VERSION,
};
use crate::mcp::tools::{handle_tool_call, list_tools_definitions};

pub struct McpServer {
    db: rusqlite::Connection,
}

impl McpServer {
    pub fn new(db_path: &str) -> Result<Self, MemAgentError> {
        let db = crate::db::schema::get_connection(db_path)?;
        Ok(Self { db })
    }

    pub fn run_stdio(&self) {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut stdout = stdout.lock();

        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("stdin read error: {e}");
                    break;
                }
            };

            if line.trim().is_empty() {
                continue;
            }

            let request: JsonRpcRequest = match serde_json::from_str(&line) {
                Ok(r) => r,
                Err(e) => {
                    let resp = JsonRpcResponse::parse_error(&format!("Parse error: {e}"));
                    Self::write_response(&mut stdout, &resp);
                    continue;
                }
            };

            let response = self.dispatch(&request);
            Self::write_response(&mut stdout, &response);
        }
    }

    fn dispatch(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            "initialize" => {
                let capabilities = ServerCapabilities {
                    tools: Some(serde_json::json!({"listChanged": true})),
                };
                let result = InitializeResult {
                    protocol_version: PROTOCOL_VERSION.to_string(),
                    capabilities,
                    server_info: ServerInfo {
                        name: "mem-agent".to_string(),
                        version: env!("CARGO_PKG_VERSION").to_string(),
                    },
                };
                JsonRpcResponse::success(
                    request.id.clone(),
                    serde_json::to_value(result).unwrap_or_default(),
                )
            }
            "tools/list" => {
                let result = ListToolsResult {
                    tools: list_tools_definitions(),
                };
                JsonRpcResponse::success(
                    request.id.clone(),
                    serde_json::to_value(result).unwrap_or_default(),
                )
            }
            "tools/call" => {
                let params: Result<crate::mcp::protocol::CallToolParams, _> =
                    serde_json::from_value(request.params.clone().unwrap_or_default());
                match params {
                    Ok(p) => match handle_tool_call(&p, &self.db) {
                        Ok(response) => {
                            if let Some(id) = &request.id {
                                JsonRpcResponse {
                                    jsonrpc: "2.0".into(),
                                    id: Some(id.clone()),
                                    result: response.result,
                                    error: None,
                                }
                            } else {
                                response
                            }
                        }
                        Err(e) => JsonRpcResponse::internal_error(request.id.clone(), &e),
                    },
                    Err(e) => JsonRpcResponse::internal_error(
                        request.id.clone(),
                        &format!("Invalid params: {e}"),
                    ),
                }
            }
            "notifications/initialized" => JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id: None,
                result: Some(serde_json::Value::Null),
                error: None,
            },
            _ => JsonRpcResponse::method_not_found(request.id.clone()),
        }
    }

    fn write_response(stdout: &mut impl Write, response: &JsonRpcResponse) {
        if let Ok(json) = serde_json::to_string(response) {
            let _ = writeln!(stdout, "{json}");
            let _ = stdout.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatch_initialize() {
        let server = McpServer::new(":memory:").unwrap();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(1)),
            method: "initialize".into(),
            params: Some(serde_json::json!({"protocolVersion": "2024-11-05"})),
        };
        let resp = server.dispatch(&req);
        assert!(resp.error.is_none());
        assert!(resp.result.is_some());
    }

    #[test]
    fn test_dispatch_tools_list() {
        let server = McpServer::new(":memory:").unwrap();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(2)),
            method: "tools/list".into(),
            params: None,
        };
        let resp = server.dispatch(&req);
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_dispatch_unknown_method() {
        let server = McpServer::new(":memory:").unwrap();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(3)),
            method: "unknown".into(),
            params: None,
        };
        let resp = server.dispatch(&req);
        assert!(resp.error.is_some());
    }
}
