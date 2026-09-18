//! MCP Tool definitions and execution handlers.

use archify_bridge::GraftToArchifyBridge;
use archify_delta::DeltaEngine;
use archify_render::SvgRenderer;
use graft_model::{CodeGraph, EdgeRelation};
use graft_parser::CodeExtractor;
use graft_search::{build_repo_map, file_skeleton, grep_graph, Bm25Index, GraphRank};
use serde_json::{json, Value};

pub struct ToolHandler;

impl ToolHandler {
    pub fn tools_list() -> Value {
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
                    "name": "graft_find_code",
                    "description": "Locate and understand: ranked BM25 + GraphRank search over the codebase (alias of graft_ask, matches the graft skill's naming)",
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
                    "name": "graft_find_all",
                    "description": "Exhaustive regex search over indexed files: declaration-line matches (name/signature) plus file-body matches with real line numbers. Does NOT group by enclosing symbol yet — the extractor does not populate node spans.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "pattern": { "type": "string" },
                            "ignore_case": { "type": "boolean" }
                        },
                        "required": ["pattern"]
                    }
                },
                {
                    "name": "graft_repo_map",
                    "description": "Token-budgeted repo orientation: directory clusters, per-directory hubs, and global hotspots from the wiring graph",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "max_dirs": { "type": "number" }
                        }
                    }
                },
                {
                    "name": "graft_trace_calls",
                    "description": "Trace upstream callers and downstream callees of a function or class",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "symbol": { "type": "string" }
                        },
                        "required": ["symbol"]
                    }
                },
                {
                    "name": "graft_file_api",
                    "description": "Signatures-only view of one file's indexed symbols (declaration lines), from the currently loaded graph",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string" }
                        },
                        "required": ["path"]
                    }
                },
                {
                    "name": "graft_check_freshness",
                    "description": "Whether the on-disk cache file is newer than every source file under `path` (mtime-based, independent of the in-session graph — needs its own `path`/`cache`)",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string" },
                            "cache": { "type": "string" }
                        },
                        "required": ["path"]
                    }
                },
                {
                    "name": "archify_render_diagram",
                    "description": "Generate an Archify architecture, sequence, dataflow, or delta diagram with neon lighting",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "title": { "type": "string" },
                            "locale": { "type": "string", "enum": ["tr", "ar", "en"] },
                            "diagram_type": { "type": "string", "enum": ["architecture", "sequence", "dataflow", "delta"] },
                            "entrypoint": { "type": "string" }
                        }
                    }
                }
            ]
        })
    }

    pub fn execute_index(path: &str) -> (Result<(CodeGraph, Bm25Index), String>, Value) {
        match CodeExtractor::index_directory(path) {
            Ok(g) => {
                let mut bm25_idx = Bm25Index::new();
                for node in &g.nodes {
                    let content = format!("{} {} {}", node.name, node.path, node.search_body);
                    bm25_idx.add_document(node.id.clone(), &content);
                }
                let node_count = g.nodes.len();
                let edge_count = g.edges.len();

                let resp = json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Indexed {} nodes and {} edges successfully.", node_count, edge_count)
                    }]
                });
                (Ok((g, bm25_idx)), resp)
            }
            Err(e) => (
                Err(e.to_string()),
                json!({ "isError": true, "content": [{ "type": "text", "text": e.to_string() }] }),
            ),
        }
    }

    pub fn execute_ask(
        query: &str,
        limit: usize,
        bm25: Option<&Bm25Index>,
        graph: Option<&CodeGraph>,
    ) -> Value {
        if let (Some(bm25), Some(graph)) = (bm25, graph) {
            let bm25_results = bm25.search(query, limit * 2);
            let ranked = GraphRank::compute(graph, &bm25_results, 0.25, 20);

            let mut formatted = String::new();
            for (id, score) in ranked.iter().take(limit) {
                formatted.push_str(&format!("• [{:.4}] {}\n", score, id));
            }

            json!({ "content": [{ "type": "text", "text": formatted }] })
        } else {
            json!({ "isError": true, "content": [{ "type": "text", "text": "Codebase not yet indexed. Run graft_index first." }] })
        }
    }

    pub fn execute_repo_map(max_dirs: usize, graph: Option<&CodeGraph>) -> Value {
        if let Some(graph) = graph {
            json!({ "content": [{ "type": "text", "text": build_repo_map(graph, max_dirs) }] })
        } else {
            json!({ "isError": true, "content": [{ "type": "text", "text": "Codebase not yet indexed. Run graft_index first." }] })
        }
    }

    pub fn execute_find_all(pattern: &str, ignore_case: bool, graph: Option<&CodeGraph>) -> Value {
        if let Some(graph) = graph {
            match grep_graph(graph, pattern, ignore_case) {
                Ok(text) => json!({ "content": [{ "type": "text", "text": text }] }),
                Err(e) => json!({ "isError": true, "content": [{ "type": "text", "text": format!("Invalid pattern: {e}") }] }),
            }
        } else {
            json!({ "isError": true, "content": [{ "type": "text", "text": "Codebase not yet indexed. Run graft_index first." }] })
        }
    }

    pub fn execute_file_api(path: &str, graph: Option<&CodeGraph>) -> Value {
        if let Some(graph) = graph {
            json!({ "content": [{ "type": "text", "text": file_skeleton(graph, path) }] })
        } else {
            json!({ "isError": true, "content": [{ "type": "text", "text": "Codebase not yet indexed. Run graft_index first." }] })
        }
    }

    /// `graft_index` çağrısıyla RAM'e alınan grafın aksine, bu SÜREÇTEN
    /// BAĞIMSIZ çalışır — diskteki cache dosyasının mtime'ını kaynak
    /// dosyalarınkiyle karşılaştırır (bkz. `CodeExtractor::freshness_report`).
    /// Bu yüzden `path`/`cache` parametre alır, oturumdaki grafı kullanmaz.
    pub fn execute_check_freshness(path: &str, cache: &str) -> Value {
        match CodeExtractor::freshness_report(path.to_string(), cache.to_string()) {
            Ok((true, _)) => json!({ "content": [{ "type": "text", "text": "fresh" }] }),
            Ok((false, bayat)) => json!({
                "content": [{
                    "type": "text",
                    "text": format!("stale: {} dosya kaynaktan daha eski:\n{}", bayat.len(), bayat.join("\n"))
                }]
            }),
            Err(e) => json!({ "isError": true, "content": [{ "type": "text", "text": e.to_string() }] }),
        }
    }

    pub fn execute_trace_calls(symbol: &str, graph: Option<&CodeGraph>) -> Value {
        if let Some(graph) = graph {
            let mut callers = Vec::new();
            let mut callees = Vec::new();

            for edge in &graph.edges {
                if edge.relation == EdgeRelation::Calls {
                    if edge.target.contains(symbol) {
                        callers.push(edge.source.clone());
                    }
                    if edge.source.contains(symbol) {
                        callees.push(edge.target.clone());
                    }
                }
            }

            let output = format!(
                "Call trace for '{}':\nUpstream Callers ({}) -> {:?}\nDownstream Callees ({}) -> {:?}",
                symbol, callers.len(), callers, callees.len(), callees
            );

            json!({ "content": [{ "type": "text", "text": output }] })
        } else {
            json!({ "isError": true, "content": [{ "type": "text", "text": "Codebase not yet indexed. Run graft_index first." }] })
        }
    }

    pub fn execute_render_diagram(args: &Value, graph: Option<&CodeGraph>) -> Value {
        if let Some(graph) = graph {
            let title = args
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or("System Architecture");
            let locale = args.get("locale").and_then(|l| l.as_str()).unwrap_or("tr");
            let diag_type = args
                .get("diagram_type")
                .and_then(|d| d.as_str())
                .unwrap_or("architecture");
            let entrypoint = args
                .get("entrypoint")
                .and_then(|e| e.as_str())
                .unwrap_or("main");

            let svg = match diag_type {
                "sequence" => {
                    let seq =
                        GraftToArchifyBridge::compile_sequence(graph, entrypoint, title, locale);
                    SvgRenderer::render_sequence(&seq)
                }
                "dataflow" => {
                    let df = GraftToArchifyBridge::compile_dataflow(graph, title, locale);
                    SvgRenderer::render_dataflow(&df)
                }
                "delta" => {
                    let delta =
                        DeltaEngine::compute_graph_delta(&CodeGraph::new(), graph, title, locale);
                    SvgRenderer::render_delta(&delta)
                }
                _ => {
                    let diagram = GraftToArchifyBridge::compile(graph, title, locale);
                    SvgRenderer::render(&diagram)
                }
            };

            json!({ "content": [{ "type": "text", "text": svg }] })
        } else {
            json!({ "isError": true, "content": [{ "type": "text", "text": "Codebase not yet indexed. Run graft_index first." }] })
        }
    }
}
