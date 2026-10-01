//! Code extractor: one file in, a small `CodeGraph` out (file node, symbols, raw imports
//! and facts). Rust goes through `graft-rust` (real body spans, impl owners, test
//! scopes); `Cargo.toml` becomes a crate node; other languages keep the line scanner.

use crate::rust_file;
use graft_model::{CodeGraph, EdgeRelation, EdgeV1, NodeKind, NodeV1};
use std::path::Path;

pub struct AstExtractor;

pub(crate) fn file_node(path_str: &str, file_name: &str, content: &str) -> NodeV1 {
    NodeV1 {
        id: format!("file:{path_str}"),
        path: path_str.to_string(),
        name: file_name.to_string(),
        kind: NodeKind::File,
        span: None,
        search_body: content.chars().take(4000).collect(),
        file_residual: content.chars().take(8000).collect(),
    }
}

pub(crate) fn contains(source: &str, target: &str) -> EdgeV1 {
    EdgeV1 {
        source: source.to_string(),
        target: target.to_string(),
        relation: EdgeRelation::Contains,
        confidence: 1.0,
    }
}

impl AstExtractor {
    pub fn extract_content(path_str: &str, file_name: &str, bytes: &[u8]) -> CodeGraph {
        let content = String::from_utf8_lossy(bytes);
        let ext = Path::new(file_name).extension().and_then(|e| e.to_str());
        let mut graph = CodeGraph::new();
        graph.add_node(file_node(path_str, file_name, &content));
        let file_id = format!("file:{path_str}");

        if file_name == "Cargo.toml" {
            rust_file::manifest(&mut graph, path_str, &file_id, &content);
            return graph;
        }
        if ext == Some("rs") {
            rust_file::extract(&mut graph, path_str, &file_id, &content);
            return graph;
        }
        if graft_langs::extract(&mut graph, path_str, &file_id, &content) {
            let specs = crate::imports::extract(path_str, &content, None);
            if !specs.is_empty() {
                graph.imports.insert(file_id, specs);
            }
        }
        graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(graph: &CodeGraph) -> Vec<&str> {
        graph.nodes.iter().map(|n| n.name.as_str()).collect()
    }

    #[test]
    fn esm_export_and_async_declarations_are_extracted() {
        let src = "export function alpha() {}\n\
                    async function beta() {}\n\
                    export async function gamma() {}\n\
                    export default function delta() {}\n\
                    export class Widget {}\n";
        let graph = AstExtractor::extract_content("a.mjs", "a.mjs", src.as_bytes());
        let found = names(&graph);
        for n in ["alpha", "beta", "gamma", "delta", "Widget"] {
            assert!(found.contains(&n), "missing {n} in {found:?}");
        }
    }

    #[test]
    fn plain_rust_and_python_declarations_still_work() {
        let src = "pub fn alpha() {}\npub async fn gamma() {}\nstruct Foo { x: i32 }\n";
        let graph = AstExtractor::extract_content("a.rs", "a.rs", src.as_bytes());
        let found = names(&graph);
        for n in ["alpha", "gamma", "Foo"] {
            assert!(found.contains(&n), "missing {n} in {found:?}");
        }
        let py = AstExtractor::extract_content("a.py", "a.py", b"def beta():\n    pass\n");
        assert!(names(&py).contains(&"beta"));
    }

    #[test]
    fn a_class_ends_with_its_braces_or_indentation() {
        let js = "class A {\n  m() {}\n}\nfunction free() {}\nfunction later() {}\n";
        let g = AstExtractor::extract_content("a.js", "a.js", js.as_bytes());
        let kind = |n: &str| g.nodes.iter().find(|x| x.name == n).unwrap().kind.clone();
        assert_eq!(kind("free"), NodeKind::Function);
        assert_eq!(kind("later"), NodeKind::Function);

        let py = "class A:\n    def m(self):\n        pass\ndef free():\n    pass\n";
        let g = AstExtractor::extract_content("a.py", "a.py", py.as_bytes());
        let kind = |n: &str| g.nodes.iter().find(|x| x.name == n).unwrap().kind.clone();
        assert_eq!(kind("m"), NodeKind::Method);
        assert_eq!(kind("free"), NodeKind::Function);
    }
}
