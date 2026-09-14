//! archify-bridge: Translates Graft's AST CodeGraph into Archify Architecture Diagrams.

use archify_ir::{
    ArchitectureDiagram, Component, Connection, DiagramMeta, SemanticRole, VisualPreset,
};
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

        let mut story_beats = Vec::new();
        let fe_ids: Vec<String> = components
            .iter()
            .filter(|c| c.role == SemanticRole::Frontend)
            .map(|c| c.id.clone())
            .collect();
        if !fe_ids.is_empty() {
            story_beats.push(archify_ir::StoryBeat {
                step: 1,
                title: match locale {
                    "tr" => "1. Kullanıcı ve Giriş Katmanı",
                    "ar" => "١. طبقة المستخدم والواجهة",
                    _ => "1. Client Ingress Layer",
                }
                .to_string(),
                description: Some(
                    "Entrypoint surfaces handling inbound client interactions".to_string(),
                ),
                highlighted_nodes: fe_ids,
            });
        }

        let be_ids: Vec<String> = components
            .iter()
            .filter(|c| c.role == SemanticRole::Backend || c.role == SemanticRole::Messagebus)
            .map(|c| c.id.clone())
            .collect();
        if !be_ids.is_empty() {
            story_beats.push(archify_ir::StoryBeat {
                step: story_beats.len() + 1,
                title: match locale {
                    "tr" => "2. Çekirdek Servis ve Mantık Katmanı",
                    "ar" => "٢. طبقة الخدمات والمنطق",
                    _ => "2. Core Application Services",
                }
                .to_string(),
                description: Some(
                    "Business logic, orchestrators, and internal message routing".to_string(),
                ),
                highlighted_nodes: be_ids,
            });
        }

        let db_ids: Vec<String> = components
            .iter()
            .filter(|c| c.role == SemanticRole::Database)
            .map(|c| c.id.clone())
            .collect();
        if !db_ids.is_empty() {
            story_beats.push(archify_ir::StoryBeat {
                step: story_beats.len() + 1,
                title: match locale {
                    "tr" => "3. Kalıcılık ve Veri Katmanı",
                    "ar" => "٣. طبقة البيانات والتخزين",
                    _ => "3. State & Persistence Storage",
                }
                .to_string(),
                description: Some(
                    "Memory-mapped indices, repositories, and transactional stores".to_string(),
                ),
                highlighted_nodes: db_ids,
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
            story_beats,
        }
    }

    pub fn compile_dataflow(
        graph: &CodeGraph,
        title: &str,
        locale: &str,
    ) -> archify_ir::DataflowDiagram {
        let mut nodes = Vec::new();
        let mut pipelines = Vec::new();

        for node in &graph.nodes {
            if node.kind == NodeKind::File || node.kind == NodeKind::Class {
                let role = infer_role(&node.name, &node.path);
                nodes.push(archify_ir::DataflowNode {
                    id: node.id.clone(),
                    label: node.name.clone(),
                    role,
                    stream_rate: Some("Direct DMA".to_string()),
                });
            }
        }

        for edge in &graph.edges {
            pipelines.push(archify_ir::DataPipeline {
                from: edge.source.clone(),
                to: edge.target.clone(),
                schema: Some("AST Call/Data".to_string()),
                throughput: Some(format!("{:.0}%", edge.confidence * 100.0)),
            });
        }

        archify_ir::DataflowDiagram {
            meta: DiagramMeta {
                title: title.to_string(),
                subtitle: Some("Dataflow and Stream Pipelines".to_string()),
                locale: locale.to_string(),
                visual_preset: VisualPreset::SignalFlow,
            },
            nodes,
            pipelines,
        }
    }

    pub fn compile_sequence(
        graph: &CodeGraph,
        root_fn: &str,
        title: &str,
        locale: &str,
    ) -> archify_ir::SequenceDiagram {
        let mut participants = Vec::new();
        let mut messages = Vec::new();
        let mut visited = std::collections::HashSet::new();

        let root_node = graph
            .nodes
            .iter()
            .find(|n| n.name == root_fn || n.id == root_fn);
        let root_id = root_node
            .map(|n| n.id.clone())
            .unwrap_or_else(|| root_fn.to_string());
        let root_label = root_node
            .map(|n| n.name.clone())
            .unwrap_or_else(|| root_fn.to_string());

        participants.push(archify_ir::SequenceParticipant {
            id: root_id.clone(),
            label: root_label,
            role: SemanticRole::Frontend,
        });
        visited.insert(root_id.clone());

        let mut order = 1;
        for edge in &graph.edges {
            if edge.source == root_id || edge.source.ends_with(&format!(":{}", root_fn)) {
                if let Some(target_node) = graph.nodes.iter().find(|n| n.id == edge.target) {
                    if !visited.contains(&target_node.id) {
                        visited.insert(target_node.id.clone());
                        participants.push(archify_ir::SequenceParticipant {
                            id: target_node.id.clone(),
                            label: target_node.name.clone(),
                            role: infer_role(&target_node.name, &target_node.path),
                        });
                    }

                    messages.push(archify_ir::SequenceMessage {
                        order,
                        from: edge.source.clone(),
                        to: target_node.id.clone(),
                        action: format!("calls {}()", target_node.name),
                        is_async: false,
                    });
                    order += 1;
                }
            }
        }

        archify_ir::SequenceDiagram {
            meta: DiagramMeta {
                title: title.to_string(),
                subtitle: Some(format!("Call Sequence Flow for '{}'", root_fn)),
                locale: locale.to_string(),
                visual_preset: VisualPreset::SignalFlow,
            },
            participants,
            messages,
        }
    }
}

fn infer_role(name: &str, path: &str) -> SemanticRole {
    let lower = format!("{} {}", name, path).to_lowercase();
    if lower.contains("db")
        || lower.contains("storage")
        || lower.contains("sql")
        || lower.contains("repo")
    {
        SemanticRole::Database
    } else if lower.contains("api")
        || lower.contains("server")
        || lower.contains("http")
        || lower.contains("service")
    {
        SemanticRole::Backend
    } else if lower.contains("ui")
        || lower.contains("view")
        || lower.contains("gui")
        || lower.contains("css")
        || lower.contains("render")
    {
        SemanticRole::Frontend
    } else if lower.contains("auth")
        || lower.contains("token")
        || lower.contains("crypto")
        || lower.contains("security")
    {
        SemanticRole::Security
    } else if lower.contains("bus")
        || lower.contains("queue")
        || lower.contains("event")
        || lower.contains("stream")
    {
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
