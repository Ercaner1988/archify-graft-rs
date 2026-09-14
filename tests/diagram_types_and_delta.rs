//! Integration test: Archify 5 diagram types, sequence call trace, dataflow pipelines, and Delta diff SVG rendering.

use archify_bridge::GraftToArchifyBridge;
use archify_delta::DeltaEngine;
use archify_ir::{ArchitectureDiagram, Component, Connection, DiagramMeta, SemanticRole, VisualPreset};
use archify_render::SvgRenderer;
use graft_model::{CodeGraph, EdgeRelation, EdgeV1, NodeKind, NodeV1};

#[test]
fn test_delta_svg_rendering_and_legend() {
    let meta = DiagramMeta {
        title: "Architecture Diff Test".to_string(),
        subtitle: None,
        locale: "tr".to_string(),
        visual_preset: VisualPreset::SignalFlow,
    };

    let before = ArchitectureDiagram {
        meta: meta.clone(),
        components: vec![
            Component { id: "auth".into(), label: "AuthService".into(), sublabel: None, role: SemanticRole::Security, x: 50.0, y: 50.0, width: 140.0, height: 60.0 },
            Component { id: "legacy_db".into(), label: "LegacyDB".into(), sublabel: None, role: SemanticRole::Database, x: 250.0, y: 50.0, width: 140.0, height: 60.0 },
        ],
        connections: vec![
            Connection { from: "auth".into(), to: "legacy_db".into(), label: None, line_style: "default".into() },
        ],
        story_beats: vec![],
    };

    let after = ArchitectureDiagram {
        meta,
        components: vec![
            Component { id: "auth".into(), label: "AuthServiceV2".into(), sublabel: None, role: SemanticRole::Security, x: 50.0, y: 50.0, width: 140.0, height: 60.0 },
            Component { id: "cloud_db".into(), label: "CloudPostgres".into(), sublabel: None, role: SemanticRole::Database, x: 250.0, y: 50.0, width: 140.0, height: 60.0 },
        ],
        connections: vec![
            Connection { from: "auth".into(), to: "cloud_db".into(), label: None, line_style: "default".into() },
        ],
        story_beats: vec![],
    };

    let delta = DeltaEngine::compute_delta(&before, &after);
    let svg = SvgRenderer::render_delta(&delta);

    // Verify SVG structure and neon diff tokens
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("[+] Added"));
    assert!(svg.contains("[-] Removed"));
    assert!(svg.contains("[Δ] Modified"));
    assert!(svg.contains("#22c55e"), "Neon emerald for added");
    assert!(svg.contains("#f43f5e"), "Neon rose for removed");
    assert!(svg.contains("#f59e0b"), "Neon amber for modified");
    assert!(svg.ends_with("</svg>\n"));
}

#[test]
fn test_sequence_diagram_compilation_and_svg() {
    let mut graph = CodeGraph::new();

    let entry = NodeV1 {
        id: "main.rs:handle_request".to_string(),
        path: "main.rs".to_string(),
        name: "handle_request".to_string(),
        kind: NodeKind::Function,
        span: None,
        search_body: "fn handle_request() { authenticate(); query_data(); }".to_string(),
        file_residual: "".to_string(),
    };
    let auth = NodeV1 {
        id: "auth.rs:authenticate".to_string(),
        path: "auth.rs".to_string(),
        name: "authenticate".to_string(),
        kind: NodeKind::Function,
        span: None,
        search_body: "fn authenticate() -> bool { true }".to_string(),
        file_residual: "".to_string(),
    };
    let db = NodeV1 {
        id: "db.rs:query_data".to_string(),
        path: "db.rs".to_string(),
        name: "query_data".to_string(),
        kind: NodeKind::Function,
        span: None,
        search_body: "fn query_data() {}".to_string(),
        file_residual: "".to_string(),
    };

    graph.add_node(entry);
    graph.add_node(auth);
    graph.add_node(db);

    graph.add_edge(EdgeV1 { source: "main.rs:handle_request".to_string(), target: "auth.rs:authenticate".to_string(), relation: EdgeRelation::Calls, confidence: 1.0 });
    graph.add_edge(EdgeV1 { source: "main.rs:handle_request".to_string(), target: "db.rs:query_data".to_string(), relation: EdgeRelation::Calls, confidence: 1.0 });

    let seq = GraftToArchifyBridge::compile_sequence(&graph, "handle_request", "Request Call Sequence", "en");
    assert_eq!(seq.participants.len(), 3);
    assert_eq!(seq.messages.len(), 2);

    let svg = SvgRenderer::render_sequence(&seq);
    assert!(svg.contains("<svg"));
    assert!(svg.contains("Request Call Sequence"));
    assert!(svg.contains("calls authenticate()"));
    assert!(svg.contains("calls query_data()"));
    assert!(svg.contains("stroke-dasharray=\"4,4\""), "Vertical lifelines");
    assert!(svg.ends_with("</svg>\n"));
}

#[test]
fn test_dataflow_diagram_compilation_and_svg() {
    let mut graph = CodeGraph::new();

    let node_in = NodeV1 {
        id: "stream_in".to_string(),
        path: "stream_in.rs".to_string(),
        name: "IngestPipeline".to_string(),
        kind: NodeKind::Class,
        span: None,
        search_body: "class IngestPipeline".to_string(),
        file_residual: "".to_string(),
    };
    let node_out = NodeV1 {
        id: "stream_out".to_string(),
        path: "stream_out.rs".to_string(),
        name: "StorageSink".to_string(),
        kind: NodeKind::Class,
        span: None,
        search_body: "class StorageSink".to_string(),
        file_residual: "".to_string(),
    };

    graph.add_node(node_in);
    graph.add_node(node_out);
    graph.add_edge(EdgeV1 { source: "stream_in".to_string(), target: "stream_out".to_string(), relation: EdgeRelation::Calls, confidence: 0.95 });

    let df = GraftToArchifyBridge::compile_dataflow(&graph, "Data Streaming Map", "en");
    assert_eq!(df.nodes.len(), 2);
    assert_eq!(df.pipelines.len(), 1);

    let svg = SvgRenderer::render_dataflow(&df);
    assert!(svg.contains("Data Streaming Map"));
    assert!(svg.contains("⚡ Direct DMA"));
    assert!(svg.contains("95%"));
    assert!(svg.ends_with("</svg>\n"));
}
