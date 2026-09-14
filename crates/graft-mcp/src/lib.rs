//! graft-mcp: High-performance pure-Rust Model Context Protocol (MCP) server.

pub mod handlers;

use std::io::{self, BufRead, Write};
use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json::{json, Value};
use graft_model::CodeGraph;
use graft_search::Bm25Index;
use handlers::ToolHandler;

pub struct McpServer {
    graph: Arc<Mutex<Option<CodeGraph>>>,
    bm25: Arc<Mutex<Option<Bm25Index>>>,
}

impl McpServer {
    pub fn new() -> Self {
        Self {
            graph: Arc::new(Mutex::new(None)),
            bm25: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn run_stdio(&self) -> anyhow::Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let reader = stdin.lock();

        for line_res in reader.lines() {
            let line = match line_res {
                Ok(l) => l,
                Err(_) => break,
            };

            if line.trim().is_empty() {
                continue;
            }

            let request: Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(e) => {
                    let err_resp = json!({
                        "jsonrpc": "2.0",
                        "error": { "code": -32700, "message": format!("Parse error: {}", e) },
                        "id": null
                    });
                    writeln!(stdout, "{}", err_resp)?;
                    stdout.flush()?;
                    continue;
                }
            };

            let id = request.get("id").cloned();
            let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
            let params = request.get("params").cloned().unwrap_or(json!({}));

            let response = self.handle_method(method, params).await;

            let full_resp = json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": response
            });

            writeln!(stdout, "{}", full_resp)?;
            stdout.flush()?;
        }

        Ok(())
    }

    pub async fn handle_method(&self, method: &str, params: Value) -> Value {
        match method {
            "initialize" => {
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "archify-graft-rs", "version": "0.1.0" }
                })
            }
            "tools/list" => ToolHandler::tools_list(),
            "tools/call" => {
                let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));

                match tool_name {
                    "graft_index" => {
                        let path = args.get("path").and_then(|p| p.as_str()).unwrap_or(".");
                        let (res, resp) = ToolHandler::execute_index(path);
                        if let Ok((g, bm25)) = res {
                            *self.graph.lock().await = Some(g);
                            *self.bm25.lock().await = Some(bm25);
                        }
                        resp
                    }
                    "graft_ask" => {
                        let query = args.get("query").and_then(|q| q.as_str()).unwrap_or("");
                        let limit = args.get("limit").and_then(|l| l.as_u64()).unwrap_or(10) as usize;
                        let bm25_guard = self.bm25.lock().await;
                        let graph_guard = self.graph.lock().await;
                        ToolHandler::execute_ask(query, limit, bm25_guard.as_ref(), graph_guard.as_ref())
                    }
                    "graft_trace_calls" => {
                        let symbol = args.get("symbol").and_then(|s| s.as_str()).unwrap_or("");
                        let graph_guard = self.graph.lock().await;
                        ToolHandler::execute_trace_calls(symbol, graph_guard.as_ref())
                    }
                    "archify_render_diagram" => {
                        let graph_guard = self.graph.lock().await;
                        ToolHandler::execute_render_diagram(&args, graph_guard.as_ref())
                    }
                    _ => json!({ "isError": true, "content": [{ "type": "text", "text": format!("Unknown tool: {}", tool_name) }] }),
                }
            }
            _ => json!({ "error": { "code": -32601, "message": "Method not found" } }),
        }
    }
}
