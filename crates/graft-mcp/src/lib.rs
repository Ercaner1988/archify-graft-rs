//! graft-mcp: High-performance pure-Rust Model Context Protocol (MCP) server.

use std::io::{self, BufRead, Write};
use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json::{json, Value};
use graft_model::CodeGraph;
use graft_parser::CodeExtractor;
use graft_search::{Bm25Index, GraphRank};
use archify_bridge::GraftToArchifyBridge;
use archify_render::SvgRenderer;

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

    async fn handle_method(&self, method: &str, params: Value) -> Value {
        match method {
            "initialize" => {
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "archify-graft-rs", "version": "0.1.0" }
                })
            }
            "tools/list" => {
                json!({
                    "tools": [
                        {
                            "name": "graft_index",
                            "description": "Index codebase using direct DMA unbuffered AST extractor",
                            "inputSchema": {
                                "type": "object",
                                "properties": { "path": { "type": "string" } },
                                "required": ["path"]
                            }
                        },
                        {
                            "name": "graft_ask",
                            "description": "Search code using trilingual BM25 (TR/AR/EN) and GraphRank",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "query": { "type": "string" },
                                    "limit": { "type": "number" }
                                },
                                "required": ["query"]
                            }
                        },
                        {
                            "name": "archify_render_diagram",
                            "description": "Generate an Archify architecture diagram with neon lighting from the indexed codebase",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "title": { "type": "string" },
                                    "locale": { "type": "string", "enum": ["tr", "ar", "en"] }
                                }
                            }
                        }
                    ]
                })
            }
            "tools/call" => {
                let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));

                match tool_name {
                    "graft_index" => {
                        let path = args.get("path").and_then(|p| p.as_str()).unwrap_or(".");
                        match CodeExtractor::index_directory(path) {
                            Ok(g) => {
                                let mut bm25_idx = Bm25Index::new();
                                for node in &g.nodes {
                                    let content = format!("{} {} {}", node.name, node.path, node.search_body);
                                    bm25_idx.add_document(node.id.clone(), &content);
                                }
                                let node_count = g.nodes.len();
                                let edge_count = g.edges.len();

                                *self.graph.lock().await = Some(g);
                                *self.bm25.lock().await = Some(bm25_idx);

                                json!({
                                    "content": [{
                                        "type": "text",
                                        "text": format!("Indexed {} nodes and {} edges successfully.", node_count, edge_count)
                                    }]
                                })
                            }
                            Err(e) => json!({ "isError": true, "content": [{ "type": "text", "text": e.to_string() }] }),
                        }
                    }
                    "graft_ask" => {
                        let query = args.get("query").and_then(|q| q.as_str()).unwrap_or("");
                        let limit = args.get("limit").and_then(|l| l.as_u64()).unwrap_or(10) as usize;

                        let bm25_guard = self.bm25.lock().await;
                        let graph_guard = self.graph.lock().await;

                        if let (Some(bm25), Some(graph)) = (bm25_guard.as_ref(), graph_guard.as_ref()) {
                            let bm25_results = bm25.search(query, limit * 2);
                            let ranked = GraphRank::compute(graph, &bm25_results, 0.25, 20);

                            let mut formatted = String::new();
                            for (id, score) in ranked.iter().take(limit) {
                                formatted.push_str(&format!("• [{:.4}] {}\n", score, id));
                            }

                            json!({
                                "content": [{ "type": "text", "text": formatted }]
                            })
                        } else {
                            json!({ "isError": true, "content": [{ "type": "text", "text": "Codebase not yet indexed. Run graft_index first." }] })
                        }
                    }
                    "archify_render_diagram" => {
                        let title = args.get("title").and_then(|t| t.as_str()).unwrap_or("System Architecture");
                        let locale = args.get("locale").and_then(|l| l.as_str()).unwrap_or("tr");

                        let graph_guard = self.graph.lock().await;
                        if let Some(graph) = graph_guard.as_ref() {
                            let diagram = GraftToArchifyBridge::compile(graph, title, locale);
                            let svg = SvgRenderer::render(&diagram);

                            json!({
                                "content": [{ "type": "text", "text": svg }]
                            })
                        } else {
                            json!({ "isError": true, "content": [{ "type": "text", "text": "Codebase not yet indexed. Run graft_index first." }] })
                        }
                    }
                    _ => json!({ "isError": true, "content": [{ "type": "text", "text": format!("Unknown tool: {}", tool_name) }] }),
                }
            }
            _ => json!({ "error": { "code": -32601, "message": "Method not found" } }),
        }
    }
}
