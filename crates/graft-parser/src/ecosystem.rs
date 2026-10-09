//! Ecosystem item extractor: skills, docx/pdf documents, data stores, scripts.

use graft_model::{CodeGraph, EdgeRelation, EdgeV1, NodeKind, NodeV1};
use std::path::Path;

pub fn extract_ecosystem(
    graph: &mut CodeGraph,
    path_str: &str,
    file_name: &str,
    content: &str,
) -> bool {
    let path = Path::new(path_str);
    let p_norm = path_str.replace('\\', "/");

    // 1. Skill extraction: .claude/skills/*/SKILL.md or .agents/skills/*/SKILL.md
    if file_name.eq_ignore_ascii_case("SKILL.md")
        || p_norm.contains("/skills/")
        || p_norm.contains("/.claude/skills/")
    {
        let skill_name = path
            .parent()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| file_name.to_string());

        let node_id = format!("skill:{skill_name}");
        let summary = content.lines().take(15).collect::<Vec<_>>().join(" ");

        graph.add_node(NodeV1 {
            id: node_id.clone(),
            path: path_str.to_string(),
            name: skill_name.clone(),
            kind: NodeKind::Skill,
            span: None,
            search_body: summary,
            file_residual: content.chars().take(2000).collect(),
        });

        // Bağlantı: skill dosyası ile düğüm
        graph.add_edge(EdgeV1 {
            source: format!("file:{path_str}"),
            target: node_id,
            relation: EdgeRelation::Contains,
            confidence: 1.0,
        });
        return true;
    }

    // 2. Documents & data files (.docx, .pdf, .ris, .csl, .md)
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext.to_ascii_lowercase().as_str() {
        "docx" | "pdf" | "ris" | "csl" => {
            let doc_id = format!("doc:{file_name}");
            graph.add_node(NodeV1 {
                id: doc_id.clone(),
                path: path_str.to_string(),
                name: file_name.to_string(),
                kind: NodeKind::Document,
                span: None,
                search_body: file_name.to_string(),
                file_residual: String::new(),
            });
            graph.add_edge(EdgeV1 {
                source: format!("file:{path_str}"),
                target: doc_id,
                relation: EdgeRelation::Contains,
                confidence: 1.0,
            });
            return true;
        }
        "sqlite" | "db" => {
            let store_id = format!("datastore:{file_name}");
            graph.add_node(NodeV1 {
                id: store_id.clone(),
                path: path_str.to_string(),
                name: file_name.to_string(),
                kind: NodeKind::DataStore,
                span: None,
                search_body: file_name.to_string(),
                file_residual: String::new(),
            });
            graph.add_edge(EdgeV1 {
                source: format!("file:{path_str}"),
                target: store_id,
                relation: EdgeRelation::Contains,
                confidence: 1.0,
            });
            return true;
        }
        _ => {}
    }

    false
}

/// Rebuilds cross-ecosystem edges: skills and scripts linking to documents,
/// datastores (Zotero), output artifacts (dn.txt, govde.txt), and helper scripts.
pub fn link(graph: &mut CodeGraph) {
    // 1. Ensure known datastores mentioned in skills exist (e.g., Zotero sqlite)
    let has_zotero = graph.nodes.iter().any(|n| {
        n.name.eq_ignore_ascii_case("zotero.sqlite") || n.id.starts_with("datastore:zotero")
    });
    let mentions_zotero = graph.nodes.iter().any(|n| {
        (n.kind == NodeKind::Skill || n.path.ends_with(".js") || n.path.ends_with(".py"))
            && (n.search_body.to_ascii_lowercase().contains("zotero")
                || n.file_residual.to_ascii_lowercase().contains("zotero"))
    });
    if mentions_zotero && !has_zotero {
        let store_id = "datastore:zotero.sqlite".to_string();
        graph.add_node(NodeV1 {
            id: store_id,
            path: "Zotero/zotero.sqlite".to_string(),
            name: "zotero.sqlite".to_string(),
            kind: NodeKind::DataStore,
            span: None,
            search_body: "Zotero SQLite Database (Keşfü'z-Zunûn)".to_string(),
            file_residual: String::new(),
        });
    }

    // 2. Identify all targets (documents, datastores)
    let doc_targets: Vec<(String, String)> = graph
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Document | NodeKind::DataStore))
        .map(|n| (n.id.clone(), n.name.to_ascii_lowercase()))
        .collect();

    // 3. Connect skills and scripts
    let mut new_edges = Vec::new();
    let mut extra_docs = Vec::new();

    for node in &graph.nodes {
        let is_skill = node.kind == NodeKind::Skill;
        let is_script = node.path.ends_with(".js") || node.path.ends_with(".py");
        if !is_skill && !is_script {
            continue;
        }

        let text = format!("{} {}", node.search_body, node.file_residual).to_ascii_lowercase();

        // Check if script writes known output files (e.g. dn.txt, govde.txt)
        for out_name in ["dn.txt", "govde.txt"] {
            if text.contains(out_name) {
                let out_id = format!("doc:{out_name}");
                if !graph.nodes.iter().any(|n| n.id == out_id)
                    && !extra_docs
                        .iter()
                        .any(|(id, _): &(String, String)| id == &out_id)
                {
                    extra_docs.push((out_id.clone(), out_name.to_string()));
                }
                new_edges.push(EdgeV1 {
                    source: node.id.clone(),
                    target: out_id,
                    relation: EdgeRelation::Produces,
                    confidence: 1.0,
                });
            }
        }

        // Match against documents / datastores
        for (target_id, target_name) in &doc_targets {
            let stem = target_name.split('.').next().unwrap_or(target_name);
            let matched = (!stem.is_empty() && stem.len() >= 3 && text.contains(stem))
                || text.contains(target_name);
            if matched {
                let rel = if text.contains("write") || text.contains("yaz") {
                    EdgeRelation::Writes
                } else if is_skill {
                    EdgeRelation::Transforms
                } else {
                    EdgeRelation::Reads
                };
                new_edges.push(EdgeV1 {
                    source: node.id.clone(),
                    target: target_id.clone(),
                    relation: rel,
                    confidence: 0.9,
                });
            }
        }
    }

    for (id, name) in extra_docs {
        graph.add_node(NodeV1 {
            id,
            path: name.clone(),
            name,
            kind: NodeKind::Document,
            span: None,
            search_body: String::new(),
            file_residual: String::new(),
        });
    }

    for edge in new_edges {
        graph.add_edge(edge);
    }
}
