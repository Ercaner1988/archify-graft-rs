//! archify-bridge: Translates Graft's AST CodeGraph into Archify Architecture Diagrams.

use archify_ir::{ArchitectureDiagram, Component, Connection, DiagramMeta, SemanticRole, VisualPreset};
use graft_model::{CodeGraph, NodeKind};

pub struct GraftToArchifyBridge;

impl GraftToArchifyBridge {
    pub fn compile(graph: &CodeGraph, title: &str, locale: &str) -> ArchitectureDiagram {
        let mut components = Vec::new();
        let mut connections = Vec::new();

        let cell_w = 160.0f32;
        let cell_h = 70.0f32;
        let gap_x = 40.0f32;
        let gap_y = 50.0f32;
        let cols = 4;

        let mut current_col = 0;
        let mut current_row = 0;

        for node in &graph.nodes {
            if node.kind == NodeKind::File || node.kind == NodeKind::Class {
                let role = infer_role(&node.name, &node.path);
                let x = 60.0 + current_col as f32 * (cell_w + gap_x);
                let y = 80.0 + current_row as f32 * (cell_h + gap_y);

                components.push(Component {
                    id: node.id.clone(),
                    label: node.name.clone(),
                    sublabel: Some(shorten_path(&node.path)),
                    role,
                    x,
                    y,
                    width: cell_w,
                    height: cell_h,
                });

                current_col += 1;
                if current_col >= cols {
                    current_col = 0;
                    current_row += 1;
                }
            }
        }

        for edge in &graph.edges {
            connections.push(Connection {
                from: edge.source.clone(),
                to: edge.target.clone(),
                label: None,
                line_style: "default".to_string(),
            });
        }

        ArchitectureDiagram {
            meta: DiagramMeta {
                title: title.to_string(),
                subtitle: Some("Automatically generated from Graft AST Call Graph".to_string()),
                locale: locale.to_string(),
                visual_preset: VisualPreset::SignalFlow,
            },
            components,
            connections,
        }
    }
}

fn infer_role(name: &str, path: &str) -> SemanticRole {
    let lower = format!("{} {}", name, path).to_lowercase();
    if lower.contains("db") || lower.contains("storage") || lower.contains("sql") || lower.contains("repo") {
        SemanticRole::Database
    } else if lower.contains("api") || lower.contains("server") || lower.contains("http") || lower.contains("service") {
        SemanticRole::Backend
    } else if lower.contains("ui") || lower.contains("view") || lower.contains("gui") || lower.contains("css") || lower.contains("render") {
        SemanticRole::Frontend
    } else if lower.contains("auth") || lower.contains("token") || lower.contains("crypto") || lower.contains("security") {
        SemanticRole::Security
    } else if lower.contains("bus") || lower.contains("queue") || lower.contains("event") || lower.contains("stream") {
        SemanticRole::Messagebus
    } else if lower.contains("cloud") || lower.contains("aws") || lower.contains("mesh") {
        SemanticRole::Cloud
    } else {
        SemanticRole::External
    }
}

fn shorten_path(p: &str) -> String {
    let path = std::path::Path::new(p);
    path.file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(p)
        .to_string()
}
