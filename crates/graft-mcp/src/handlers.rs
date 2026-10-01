//! MCP Tool definitions and execution handlers.

use archify_bridge::GraftToArchifyBridge;
use archify_delta::DeltaEngine;
use archify_render::SvgRenderer;
use graft_model::{CodeGraph, EdgeRelation};
use graft_parser::CodeExtractor;
use graft_search::{build_repo_map, file_skeleton, grep_graph, Bm25Index, GraphRank, GraphStorage};
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
                    "description": "Generate an Archify architecture, sequence, dataflow, or delta diagram with neon lighting and write it to disk. Returns the written path and a short summary, not the raw diagram markup — write `output` ending in `.html` for a self-contained, pannable/zoomable viewer, or `.svg` for the raw static image.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "title": { "type": "string" },
                            "locale": { "type": "string", "enum": ["tr", "ar", "en"] },
                            "diagram_type": { "type": "string", "enum": ["architecture", "sequence", "dataflow", "delta"] },
                            "entrypoint": { "type": "string" },
                            "output": { "type": "string", "description": "File path to write; extension `.html`/`.htm` wraps the SVG in an explorable pan/zoom viewer, anything else writes raw SVG. Defaults to `<diagram_type>.svg` in the server's working directory." }
                        }
                    }
                }
            ]
        })
    }

    /// Reuses `.cache/graft-graph.bin` via an incremental rescan when it exists
    /// (same cache the CLI's `index` command writes) instead of always doing a
    /// full re-parse — an MCP session used to pay that cost every single time.
    pub fn execute_index(path: &str) -> (Result<(CodeGraph, Bm25Index), String>, Value) {
        match GraphStorage::index_or_refresh(path, ".cache/graft-graph.bin") {
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
                Err(e) => {
                    json!({ "isError": true, "content": [{ "type": "text", "text": format!("Invalid pattern: {e}") }] })
                }
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
            Err(e) => {
                json!({ "isError": true, "content": [{ "type": "text", "text": e.to_string() }] })
            }
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

    /// Renders and WRITES the diagram to disk, returning only a path + a short summary.
    /// The full SVG/HTML markup used to come back as the tool's `text` content — for any
    /// non-trivial codebase that is tens or hundreds of KB of XML dumped straight into
    /// the agent's context (and easily truncated by the caller's own limits), with
    /// nothing written anywhere the human can actually open. That is very likely why an
    /// earlier "diagram attempt" through this tool looked broken: there was no file, just
    /// an unreadable text blob.
    pub fn execute_render_diagram(args: &Value, graph: Option<&CodeGraph>) -> Value {
        let Some(graph) = graph else {
            return json!({ "isError": true, "content": [{ "type": "text", "text": "Codebase not yet indexed. Run graft_index first." }] });
        };

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

        let (svg, summary) = match diag_type {
            "sequence" => {
                let seq = GraftToArchifyBridge::compile_sequence(graph, entrypoint, title, locale);
                let summary = format!(
                    "{} participants, {} messages",
                    seq.participants.len(),
                    seq.messages.len()
                );
                (SvgRenderer::render_sequence(&seq), summary)
            }
            "dataflow" => {
                let df = GraftToArchifyBridge::compile_dataflow(graph, title, locale);
                let summary = format!("{} nodes, {} pipelines", df.nodes.len(), df.pipelines.len());
                (SvgRenderer::render_dataflow(&df), summary)
            }
            "delta" => {
                let delta =
                    DeltaEngine::compute_graph_delta(&CodeGraph::new(), graph, title, locale);
                let summary = format!(
                    "{} components, {} connections (vs. empty baseline)",
                    delta.components.len(),
                    delta.connections.len()
                );
                (SvgRenderer::render_delta(&delta), summary)
            }
            _ => {
                let diagram = GraftToArchifyBridge::compile(graph, title, locale);
                let summary = format!(
                    "{} components, {} connections, {} regions",
                    diagram.components.len(),
                    diagram.connections.len(),
                    diagram.regions.len()
                );
                (SvgRenderer::render(&diagram), summary)
            }
        };

        let output = args
            .get("output")
            .and_then(|o| o.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| format!("{diag_type}.svg"));
        let is_html = std::path::Path::new(&output)
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("html") || e.eq_ignore_ascii_case("htm"));
        let contents = if is_html {
            archify_render::wrap_html(&svg, title)
        } else {
            svg
        };
        let bytes = contents.len();

        match std::fs::write(&output, contents) {
            Ok(()) => json!({ "content": [{
                "type": "text",
                "text": format!(
                    "Wrote {} diagram to '{}' ({} bytes, {}).{}",
                    diag_type, output, bytes, summary,
                    if is_html { " Open it in a browser: drag to pan, scroll to zoom." } else { "" }
                )
            }] }),
            Err(e) => json!({ "isError": true, "content": [{
                "type": "text",
                "text": format!("Rendered the diagram ({summary}) but could not write '{output}': {e}")
            }] }),
        }
    }
}

#[cfg(test)]
mod render_diagram_tests {
    use super::*;
    use graft_model::{NodeKind, NodeV1};

    fn sample_graph() -> CodeGraph {
        let mut g = CodeGraph::new();
        g.add_node(NodeV1 {
            id: "file:a.rs".into(),
            path: "a.rs".into(),
            name: "a.rs".into(),
            kind: NodeKind::File,
            span: None,
            search_body: String::new(),
            file_residual: String::new(),
        });
        g
    }

    /// The tool used to return the entire SVG/HTML markup as the MCP text content —
    /// tens of KB of XML dumped into the agent's context, with no file written anywhere
    /// a human could open it. That is the behavior a prior "the diagram doesn't work"
    /// report almost certainly hit. This locks in the fix: a file on disk, a short
    /// summary in the response.
    #[test]
    fn writes_svg_to_disk_and_returns_a_summary_not_the_markup() {
        let dir = std::env::temp_dir().join(format!("agr-mcp-svg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("d.svg");
        let resp = ToolHandler::execute_render_diagram(
            &json!({ "output": out.to_string_lossy() }),
            Some(&sample_graph()),
        );
        let text = resp["content"][0]["text"].as_str().unwrap();
        assert!(text.starts_with("Wrote architecture diagram"), "{text}");
        assert!(
            !text.contains("<svg"),
            "response must not embed the raw diagram markup: {text}"
        );
        let written = std::fs::read_to_string(&out).unwrap();
        assert!(written.contains("<svg"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn html_output_wraps_the_svg_without_any_script() {
        let dir = std::env::temp_dir().join(format!("agr-mcp-html-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("d.html");
        ToolHandler::execute_render_diagram(
            &json!({ "output": out.to_string_lossy() }),
            Some(&sample_graph()),
        );
        let written = std::fs::read_to_string(&out).unwrap();
        assert!(written.contains("<html"));
        assert!(!written.contains("<script"), "output must stay script-free");
        assert!(
            written.contains("<svg"),
            "the diagram itself must still be embedded"
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
