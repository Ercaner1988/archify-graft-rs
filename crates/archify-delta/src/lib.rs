//! archify-delta: Computes visual diff states (Added, Removed, Modified, Unchanged)
//! between two architectural revisions or commit graphs.

use std::collections::{HashMap, HashSet};
use archify_ir::{ArchitectureDiagram, Component, DeltaComponent, DeltaConnection, DeltaDiagram, DiagramMeta, DiffStatus};
use graft_model::CodeGraph;
use archify_bridge::GraftToArchifyBridge;

pub struct DeltaEngine;

impl DeltaEngine {
    /// Compares two architecture diagrams and produces a DeltaDiagram with visual statuses
    pub fn compute_delta(before: &ArchitectureDiagram, after: &ArchitectureDiagram) -> DeltaDiagram {
        let before_comps: HashMap<&str, &Component> = before.components.iter().map(|c| (c.id.as_str(), c)).collect();
        let after_comps: HashMap<&str, &Component> = after.components.iter().map(|c| (c.id.as_str(), c)).collect();

        let mut delta_components = Vec::new();

        // 1. Check components in 'after' (Added, Modified, or Unchanged)
        for comp in &after.components {
            if let Some(old) = before_comps.get(comp.id.as_str()) {
                let is_modified = old.label != comp.label || old.role != comp.role || old.sublabel != comp.sublabel;
                let status = if is_modified {
                    DiffStatus::Modified
                } else {
                    DiffStatus::Unchanged
                };
                delta_components.push(DeltaComponent {
                    component: (*comp).clone(),
                    status,
                });
            } else {
                delta_components.push(DeltaComponent {
                    component: (*comp).clone(),
                    status: DiffStatus::Added,
                });
            }
        }

        // 2. Check components only in 'before' (Removed)
        for comp in &before.components {
            if !after_comps.contains_key(comp.id.as_str()) {
                delta_components.push(DeltaComponent {
                    component: (*comp).clone(),
                    status: DiffStatus::Removed,
                });
            }
        }

        // 3. Check connections
        let before_conns: HashSet<(&str, &str)> = before.connections.iter().map(|c| (c.from.as_str(), c.to.as_str())).collect();
        let after_conns: HashSet<(&str, &str)> = after.connections.iter().map(|c| (c.from.as_str(), c.to.as_str())).collect();

        let mut delta_connections = Vec::new();

        for conn in &after.connections {
            let key = (conn.from.as_str(), conn.to.as_str());
            let status = if before_conns.contains(&key) {
                DiffStatus::Unchanged
            } else {
                DiffStatus::Added
            };
            delta_connections.push(DeltaConnection {
                connection: conn.clone(),
                status,
            });
        }

        for conn in &before.connections {
            let key = (conn.from.as_str(), conn.to.as_str());
            if !after_conns.contains(&key) {
                delta_connections.push(DeltaConnection {
                    connection: conn.clone(),
                    status: DiffStatus::Removed,
                });
            }
        }

        DeltaDiagram {
            meta: DiagramMeta {
                title: format!("Δ Delta: {}", after.meta.title),
                subtitle: Some("Automated Architectural Diff Analysis".to_string()),
                locale: after.meta.locale.clone(),
                visual_preset: after.meta.visual_preset,
            },
            components: delta_components,
            connections: delta_connections,
        }
    }

    /// Compares two AST code graphs directly and produces a DeltaDiagram
    pub fn compute_graph_delta(before_graph: &CodeGraph, after_graph: &CodeGraph, title: &str, locale: &str) -> DeltaDiagram {
        let before_diag = GraftToArchifyBridge::compile(before_graph, title, locale);
        let after_diag = GraftToArchifyBridge::compile(after_graph, title, locale);
        Self::compute_delta(&before_diag, &after_diag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archify_ir::{Connection, DiagramMeta, SemanticRole, VisualPreset};

    #[test]
    fn test_delta_detection_added_removed_modified() {
        let meta = DiagramMeta {
            title: "Test".to_string(),
            subtitle: None,
            locale: "en".to_string(),
            visual_preset: VisualPreset::SignalFlow,
        };

        let before = ArchitectureDiagram {
            meta: meta.clone(),
            components: vec![
                Component { id: "a".into(), label: "Worker".into(), sublabel: None, role: SemanticRole::Backend, x: 0.0, y: 0.0, width: 100.0, height: 50.0 },
                Component { id: "b".into(), label: "Database".into(), sublabel: None, role: SemanticRole::Database, x: 0.0, y: 0.0, width: 100.0, height: 50.0 },
            ],
            connections: vec![
                Connection { from: "a".into(), to: "b".into(), label: None, line_style: "default".into() },
            ],
        };

        let after = ArchitectureDiagram {
            meta,
            components: vec![
                // "a" modified role to Cloud
                Component { id: "a".into(), label: "Worker".into(), sublabel: None, role: SemanticRole::Cloud, x: 0.0, y: 0.0, width: 100.0, height: 50.0 },
                // "b" is removed
                // "c" is added
                Component { id: "c".into(), label: "Redis".into(), sublabel: None, role: SemanticRole::Database, x: 0.0, y: 0.0, width: 100.0, height: 50.0 },
            ],
            connections: vec![
                Connection { from: "a".into(), to: "c".into(), label: None, line_style: "default".into() },
            ],
        };

        let delta = DeltaEngine::compute_delta(&before, &after);

        let a_status = delta.components.iter().find(|c| c.component.id == "a").map(|c| c.status);
        let b_status = delta.components.iter().find(|c| c.component.id == "b").map(|c| c.status);
        let c_status = delta.components.iter().find(|c| c.component.id == "c").map(|c| c.status);

        assert_eq!(a_status, Some(DiffStatus::Modified));
        assert_eq!(b_status, Some(DiffStatus::Removed));
        assert_eq!(c_status, Some(DiffStatus::Added));

        assert_eq!(delta.connections.len(), 2);
    }
}
