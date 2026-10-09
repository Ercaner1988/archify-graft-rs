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
