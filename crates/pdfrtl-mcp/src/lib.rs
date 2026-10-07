//! pdfrtl MCP server — exposes the CLI contract to agents as MCP tools.
//!
//! Design rule: this crate is a *thin adapter*. It must never re-implement logic
//! that lives in `pdfrtl-core`; it maps MCP tool calls onto the same functions the
//! CLI calls, so the JSON contract and the MCP surface cannot drift apart.
//!
//! Implements the Model Context Protocol (MCP) JSON-RPC 2.0 stdio server.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::Path;

/// Name of the MCP server as advertised to clients.
pub const SERVER_NAME: &str = "pdfrtl";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Tools this server exposes.
pub const PLANNED_TOOLS: &[&str] = &["inspect", "extract", "search", "generate", "edit", "sign"];

/// JSON-RPC 2.0 Request envelope
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 Response envelope
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Handle a single JSON-RPC line request and return the JSON response string, if any.
pub fn handle_request(req_str: &str) -> Option<String> {
    let req: JsonRpcRequest = match serde_json::from_str(req_str) {
        Ok(r) => r,
        Err(e) => {
            let res = JsonRpcResponse {
                jsonrpc: "2.0",
                id: None,
                result: None,
                error: Some(JsonRpcError {
                    code: -32700,
                    message: format!("Parse error: {e}"),
                    data: None,
                }),
            };
            return Some(serde_json::to_string(&res).unwrap_or_default());
        }
    };

    let id = req.id.clone();
    let res = match req.method.as_str() {
        "initialize" => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": SERVER_NAME,
                    "version": SERVER_VERSION
                }
            })),
            error: None,
        },
        "notifications/initialized" => {
            // Notifications do not have responses
            return None;
        }
        "ping" => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(json!({})),
            error: None,
        },
        "tools/list" => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(json!({
                "tools": list_tools()
            })),
            error: None,
        },
        "tools/call" => {
            let params = req.params.unwrap_or(Value::Null);
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);

            match call_tool(name, arguments) {
                Ok(content) => JsonRpcResponse {
                    jsonrpc: "2.0",
                    id,
                    result: Some(json!({
                        "content": [
                            {
                                "type": "text",
                                "text": content
                            }
                        ]
                    })),
                    error: None,
                },
                Err(err) => JsonRpcResponse {
                    jsonrpc: "2.0",
                    id,
                    result: Some(json!({
                        "isError": true,
                        "content": [
                            {
                                "type": "text",
                                "text": format!("Error: {err:#}")
                            }
                        ]
                    })),
                    error: None,
                },
            }
        }
        _ => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("Method not found: {}", req.method),
                data: None,
            }),
        },
    };

    Some(serde_json::to_string(&res).unwrap_or_default())
}

/// Tools metadata advertised to MCP clients.
pub fn list_tools() -> Vec<Value> {
    vec![
        json!({
            "name": "inspect",
            "description": "Report PDF metadata, producer fingerprint, page count, and encryption status",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "Path to the PDF file"
                    }
                },
                "required": ["file"]
            }
        }),
        json!({
            "name": "extract_text",
            "description": "Extract proven logical-order text page by page with justifications",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "Path to the PDF file"
                    },
                    "include_unproven": {
                        "type": "boolean",
                        "description": "Include raw stored text of unproven/withheld lines"
                    }
                },
                "required": ["file"]
            }
        }),
        json!({
            "name": "extract_layout",
            "description": "Extract spatial reading layout blocks, lines, and spans with bounding boxes [x0, y0, x1, y1]",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "Path to the PDF file"
                    }
                },
                "required": ["file"]
            }
        }),
        json!({
            "name": "search",
            "description": "Search PDF text using bilingual Arabic/Persian/Hebrew normalisation",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "Path to the PDF file"
                    },
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    }
                },
                "required": ["file", "query"]
            }
        }),
    ]
}

/// Execute a tool invocation by name and arguments.
pub fn call_tool(name: &str, arguments: Value) -> anyhow::Result<String> {
    let file_str = arguments
        .get("file")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required argument 'file'"))?;
    let path = Path::new(file_str);

    match name {
        "inspect" => {
            let info = pdfrtl_core::inspect(path)?;
            Ok(serde_json::to_string_pretty(&info)?)
        }
        "extract_text" => {
            let include_unproven = arguments
                .get("include_unproven")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let pages = pdfrtl_core::extract(path)?;
            let text = pages
                .iter()
                .filter(|p| !p.text.is_empty())
                .map(|p| p.text.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            let unproven_lines: usize = pages.iter().map(|p| p.unproven.len()).sum();
            let unordered_chars: usize = pages.iter().map(|p| p.unordered_chars).sum();
            let page_list: Vec<Value> = pages
                .iter()
                .map(|p| {
                    json!({
                        "page": p.page,
                        "text": p.text,
                        "ok": p.is_decoded(),
                        "reasons": p.reasons.iter().map(|r| r.as_str()).collect::<Vec<_>>(),
                        "unordered_chars": p.unordered_chars,
                        "unproven": if include_unproven {
                            serde_json::to_value(&p.unproven).unwrap_or(Value::Null)
                        } else {
                            Value::Array(Vec::new())
                        }
                    })
                })
                .collect();
            let res = json!({
                "text": text,
                "unordered_chars": unordered_chars,
                "unproven_lines": unproven_lines,
                "pages": page_list
            });
            Ok(serde_json::to_string_pretty(&res)?)
        }
        "extract_layout" => {
            let layout = pdfrtl_core::extract_layout(path)?;
            Ok(serde_json::to_string_pretty(&layout)?)
        }
        "search" => {
            let query = arguments
                .get("query")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing required argument 'query'"))?;
            let norm_query = pdfrtl_core::search::normalize_for_search(query);
            let pages = pdfrtl_core::extract(path)?;
            let mut matches = Vec::new();
            for page in &pages {
                if !page.is_ordered() || page.text.is_empty() {
                    continue;
                }
                for (line_idx, line) in page.text.lines().enumerate() {
                    let norm_line = pdfrtl_core::search::normalize_for_search(line);
                    if norm_line.contains(&norm_query) {
                        matches.push(json!({
                            "page": page.page,
                            "line_number": line_idx + 1,
                            "text": line
                        }));
                    }
                }
            }
            let res = json!({
                "query": query,
                "normalized_query": norm_query,
                "matches": matches
            });
            Ok(serde_json::to_string_pretty(&res)?)
        }
        _ => anyhow::bail!("Unsupported tool: {name}"),
    }
}

/// Run the stdio loop, processing line-delimited JSON-RPC messages.
pub fn run_stdio() -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(resp) = handle_request(&line) {
            writeln!(stdout, "{resp}")?;
            stdout.flush()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planned_tools_are_the_contract_verbs() {
        assert_eq!(SERVER_NAME, "pdfrtl");
        assert_eq!(PLANNED_TOOLS[0], "inspect");
        assert!(PLANNED_TOOLS.contains(&"extract"));
        assert_eq!(PLANNED_TOOLS.len(), 6);
    }

    #[test]
    fn initialize_request_returns_server_capabilities() {
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        })
        .to_string();
        let resp_str = handle_request(&req).expect("response");
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(resp["id"], 1);
        assert_eq!(resp["result"]["serverInfo"]["name"], "pdfrtl");
        assert_eq!(resp["result"]["protocolVersion"], "2024-11-05");
    }

    #[test]
    fn tools_list_returns_advertised_tools() {
        let req = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        })
        .to_string();
        let resp_str = handle_request(&req).expect("response");
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(resp["id"], 2);
        let tools = resp["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"inspect"));
        assert!(names.contains(&"extract_text"));
        assert!(names.contains(&"extract_layout"));
        assert!(names.contains(&"search"));
    }
}
