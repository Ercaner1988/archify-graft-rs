//! graft-parser: Direct DMA unbuffered file reader and parallel AST extractor.

use std::path::{Path, PathBuf};
use rayon::prelude::*;
use graft_model::{CodeGraph, EdgeRelation, EdgeV1, NodeKind, NodeV1};
use lowlevel_sys::DirectReader;

pub struct CodeExtractor;

impl CodeExtractor {
    pub fn index_directory<P: AsRef<Path>>(root: P) -> anyhow::Result<CodeGraph> {
        let root_path = root.as_ref().to_path_buf();
        let mut file_paths = Vec::new();
        Self::collect_files(&root_path, &mut file_paths)?;

        let file_graphs: Vec<CodeGraph> = file_paths
            .par_iter()
            .filter_map(|path| Self::extract_file(path).ok())
            .collect();

        let mut master = CodeGraph::new();
        for sub in file_graphs {
            for node in sub.nodes {
                master.add_node(node);
            }
            for edge in sub.edges {
                master.add_edge(edge);
            }
        }

        master.rebuild_index();
        Ok(master)
    }

    fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) -> anyhow::Result<()> {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

                if file_name.starts_with('.')
                    || file_name == "target"
                    || file_name == "node_modules"
                    || file_name == "dist"
                    || file_name == "build"
                {
                    continue;
                }

                if path.is_dir() {
                    Self::collect_files(&path, files)?;
                } else if Self::is_supported_extension(&path) {
                    files.push(path);
                }
            }
        }
        Ok(())
    }

    fn is_supported_extension(path: &Path) -> bool {
        matches!(
            path.extension().and_then(|s| s.to_str()),
            Some("rs") | Some("ts") | Some("js") | Some("py") | Some("go") | Some("java")
                | Some("c") | Some("cpp") | Some("h") | Some("hpp") | Some("json")
        )
    }

    pub fn extract_file(path: &Path) -> anyhow::Result<CodeGraph> {
        let bytes = DirectReader::read_file(path)?;
        let content = String::from_utf8_lossy(&bytes);
        let path_str = path.to_string_lossy().to_string();

        let mut graph = CodeGraph::new();

        let file_node_id = format!("file:{}", path_str);
        graph.add_node(NodeV1 {
            id: file_node_id.clone(),
            path: path_str.clone(),
            name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
            kind: NodeKind::File,
            span: None,
            search_body: content.chars().take(4000).collect(),
            file_residual: content.chars().take(8000).collect(),
        });

        let mut current_class: Option<String> = None;

        for (_line_idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            if trimmed.starts_with("pub struct ") || trimmed.starts_with("struct ") || trimmed.starts_with("class ") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    let name = parts[parts.len() - 1].trim_end_matches('{').trim_end_matches(';');
                    let class_id = format!("{}:{}", path_str, name);
                    current_class = Some(class_id.clone());

                    graph.add_node(NodeV1 {
                        id: class_id.clone(),
                        path: path_str.clone(),
                        name: name.to_string(),
                        kind: NodeKind::Class,
                        span: None,
                        search_body: trimmed.to_string(),
                        file_residual: String::new(),
                    });

                    graph.add_edge(EdgeV1 {
                        source: file_node_id.clone(),
                        target: class_id,
                        relation: EdgeRelation::Contains,
                        confidence: 1.0,
                    });
                }
            }

            if trimmed.starts_with("pub fn ")
                || trimmed.starts_with("fn ")
                || trimmed.starts_with("def ")
                || trimmed.starts_with("function ")
            {
                if let Some(fn_name) = extract_identifier_after_keyword(trimmed) {
                    let fn_id = format!("{}:{}", path_str, fn_name);

                    graph.add_node(NodeV1 {
                        id: fn_id.clone(),
                        path: path_str.clone(),
                        name: fn_name.to_string(),
                        kind: if current_class.is_some() {
                            NodeKind::Method
                        } else {
                            NodeKind::Function
                        },
                        span: None,
                        search_body: trimmed.to_string(),
                        file_residual: String::new(),
                    });

                    let parent = current_class.as_ref().unwrap_or(&file_node_id);
                    graph.add_edge(EdgeV1 {
                        source: parent.clone(),
                        target: fn_id.clone(),
                        relation: EdgeRelation::Contains,
                        confidence: 1.0,
                    });
                }
            }
        }

        Ok(graph)
    }
}

fn extract_identifier_after_keyword(line: &str) -> Option<&str> {
    for kw in &["pub fn ", "fn ", "def ", "function "] {
        if let Some(pos) = line.find(kw) {
            let after = &line[pos + kw.len()..];
            let name = after.split('(').next()?.trim();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}
