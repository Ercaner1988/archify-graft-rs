//! Integration test: CodeGraph, GraphRank convergence, topologic reachability, and Neon SVG rendering.

use archify_geometry::ReachabilityEngine;
use archify_ir::{
    ArchitectureDiagram, Component, Connection, DiagramMeta, SemanticRole, VisualPreset,
};
use archify_render::SvgRenderer;
use graft_model::{CodeGraph, EdgeRelation, EdgeV1, NodeKind, NodeV1};
use graft_search::GraphRank;

#[test]
fn test_graphrank_convergence_on_cyclical_graph() {
    let mut graph = CodeGraph::new();

    let node_a = NodeV1 {
        id: "A".to_string(),
        path: "a.rs".to_string(),
        name: "ServiceA".to_string(),
        kind: NodeKind::Class,
        span: None,
        search_body: "service a".to_string(),
        file_residual: "".to_string(),
    };
    let node_b = NodeV1 {
        id: "B".to_string(),
        path: "b.rs".to_string(),
        name: "ServiceB".to_string(),
        kind: NodeKind::Class,
        span: None,
        search_body: "service b".to_string(),
        file_residual: "".to_string(),
    };
    let node_c = NodeV1 {
        id: "C".to_string(),
        path: "c.rs".to_string(),
        name: "ServiceC".to_string(),
        kind: NodeKind::Class,
        span: None,
        search_body: "service c".to_string(),
        file_residual: "".to_string(),
    };

    graph.add_node(node_a);
    graph.add_node(node_b);
    graph.add_node(node_c);

    // Cycle: A -> B -> C -> A
    graph.add_edge(EdgeV1 {
        source: "A".to_string(),
        target: "B".to_string(),
        relation: EdgeRelation::Calls,
        confidence: 1.0,
    });
    graph.add_edge(EdgeV1 {
        source: "B".to_string(),
        target: "C".to_string(),
        relation: EdgeRelation::Calls,
        confidence: 1.0,
    });
    graph.add_edge(EdgeV1 {
        source: "C".to_string(),
        target: "A".to_string(),
        relation: EdgeRelation::Calls,
        confidence: 1.0,
    });

    let seeds = vec![("A".to_string(), 1.0f32)];
    let ranks = GraphRank::compute(&graph, &seeds, 0.25, 30);

    assert_eq!(ranks.len(), 3);
    // In a symmetric cycle with seed at A, A receives highest return probability
    assert!(ranks[0].0 == "A");
    // All nodes should have positive rank
    for (_, score) in ranks {
        assert!(score > 0.0);
    }
}

#[test]
fn test_reachability_route_probe() {
    let diagram = ArchitectureDiagram {
        meta: DiagramMeta {
            title: "Test Mesh".to_string(),
            subtitle: None,
            locale: "en".to_string(),
            visual_preset: VisualPreset::SignalFlow,
        },
        components: vec![
            Component {
                id: "ui".to_string(),
                label: "Frontend".to_string(),
                sublabel: None,
                role: SemanticRole::Frontend,
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
            Component {
                id: "api".to_string(),
                label: "Gateway".to_string(),
                sublabel: None,
                role: SemanticRole::Backend,
                x: 150.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
            Component {
                id: "db".to_string(),
                label: "Database".to_string(),
                sublabel: None,
                role: SemanticRole::Database,
                x: 300.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
            Component {
                id: "isolated".to_string(),
                label: "Orphan".to_string(),
                sublabel: None,
                role: SemanticRole::External,
                x: 450.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
        ],
        connections: vec![
            Connection {
                from: "ui".to_string(),
                to: "api".to_string(),
                label: None,
                line_style: "default".to_string(),
            },
            Connection {
                from: "api".to_string(),
                to: "db".to_string(),
                label: None,
                line_style: "default".to_string(),
            },
        ],
        story_beats: vec![],
    };

    // UI to DB path exists: ui -> api -> db
    let route = ReachabilityEngine::find_route(&diagram, "ui", "db");
    assert_eq!(
        route,
        Some(vec!["ui".to_string(), "api".to_string(), "db".to_string()])
    );

    // UI to isolated path does not exist
    let unreachable = ReachabilityEngine::find_route(&diagram, "ui", "isolated");
    assert!(unreachable.is_none());
}

#[test]
fn test_neon_svg_rendering_structure() {
    let diagram = ArchitectureDiagram {
        meta: DiagramMeta {
            title: "مخطط بنية النظام".to_string(),
            subtitle: None,
            locale: "ar".to_string(),
            visual_preset: VisualPreset::SignalFlow,
        },
        components: vec![Component {
            id: "server".to_string(),
            label: "الخادم".to_string(),
            sublabel: Some("api.rs".to_string()),
            role: SemanticRole::Backend,
            x: 50.0,
            y: 50.0,
            width: 160.0,
            height: 70.0,
        }],
        connections: vec![],
        story_beats: vec![],
    };

    let svg = SvgRenderer::render(&diagram);

    // Verify critical XML and visual tokens
    assert!(svg.starts_with("<svg"));
    assert!(
        svg.contains("dir=\"rtl\""),
        "Arabic SVG must have dir='rtl' attribute"
    );
    assert!(
        svg.contains("filter:drop-shadow(0 0 8px #34d399)"),
        "Must contain Proof Green neon bloom for Backend role"
    );
    assert!(svg.contains("الخادم"), "Must render Arabic text label");
    assert!(svg.ends_with("</svg>\n"));
}
