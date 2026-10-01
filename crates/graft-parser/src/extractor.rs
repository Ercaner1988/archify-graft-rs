//! Code extractor: one file in, a small `CodeGraph` out (file node, symbols, raw imports
//! and facts). Rust goes through `graft-rust` (real body spans, impl owners, test
//! scopes); `Cargo.toml` becomes a crate node; other languages keep the line scanner.

use crate::rust_file;
use graft_model::{CodeGraph, EdgeRelation, EdgeV1, NodeKind, NodeV1};
use std::path::Path;

pub struct AstExtractor;

/// One list for both "should this line even be inspected" and "where does the name
/// start". Covers ESM's `export`/`export default` variants and `async function`.
const TYPE_PREFIXES: [&str; 5] = [
    "pub struct ",
    "struct ",
    "class ",
    "export class ",
    "export default class ",
];
const FN_PREFIXES: [&str; 12] = [
    "pub async fn ",
    "pub fn ",
    "async fn ",
    "fn ",
    "async def ",
    "def ",
    "function ",
    "async function ",
    "export function ",
    "export async function ",
    "export default function ",
    "export default async function ",
];

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
        Self::extract_lines(&mut graph, path_str, &file_id, &content, ext == Some("py"));
        let specs = crate::imports::extract(path_str, &content, None);
        if !specs.is_empty() {
            graph.imports.insert(file_id.clone(), specs);
        }
        let lines = content.lines().filter(|l| !l.trim().is_empty()).count();
        graph.facts.insert(file_id, vec![format!("loc:{lines}")]);
        graph
    }

    /// Line-prefix scanner for JS/TS/Python/Go/Java/C: a class ends where its braces
    /// (or, for Python, its indentation) end, so later functions are not its methods.
    fn extract_lines(graph: &mut CodeGraph, path: &str, file_id: &str, content: &str, py: bool) {
        // (class id, brace depth before its line / indentation of its line)
        let mut classes: Vec<(String, i32)> = Vec::new();
        let mut depth = 0i32;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let indent = (line.len() - line.trim_start().len()) as i32;
            if py {
                while classes.last().is_some_and(|(_, i)| indent <= *i) {
                    classes.pop();
                }
            }
            let mark = if py { indent } else { depth };

            if TYPE_PREFIXES.iter().any(|p| trimmed.starts_with(p)) {
                if let Some(name) = Self::extract_type_name(trimmed, &TYPE_PREFIXES) {
                    let id = format!("{path}:{name}");
                    Self::push(graph, file_id, &id, path, name, NodeKind::Class, trimmed);
                    classes.push((id, mark));
                }
            }
            if let Some(name) = ["pub enum ", "enum ", "export enum "]
                .iter()
                .find_map(|p| trimmed.strip_prefix(p).map(|_| p))
                .and_then(|p| Self::extract_type_name(trimmed, &[p]))
            {
                let id = format!("{path}:{name}");
                Self::push(graph, file_id, &id, path, name, NodeKind::Enum, trimmed);
            }
            if let Some(name) = Self::extract_type_name(
                trimmed,
                &["pub trait ", "trait ", "interface ", "export interface "],
            )
            .filter(|_| {
                ["pub trait ", "trait ", "interface ", "export interface "]
                    .iter()
                    .any(|p| trimmed.starts_with(p))
            }) {
                let id = format!("{path}:{name}");
                Self::push(
                    graph,
                    file_id,
                    &id,
                    path,
                    name,
                    NodeKind::Interface,
                    trimmed,
                );
            }
            if FN_PREFIXES.iter().any(|p| trimmed.starts_with(p)) {
                if let Some(name) = Self::extract_identifier_after_keyword(trimmed) {
                    let parent = classes.last().map(|(id, _)| id.as_str());
                    let id = format!("{path}:{name}");
                    let kind = if parent.is_some() {
                        NodeKind::Method
                    } else {
                        NodeKind::Function
                    };
                    Self::push(
                        graph,
                        parent.unwrap_or(file_id),
                        &id,
                        path,
                        name,
                        kind,
                        trimmed,
                    );
                }
            }
            if !py {
                depth += trimmed.matches('{').count() as i32 - trimmed.matches('}').count() as i32;
                while classes.last().is_some_and(|(_, d)| depth <= *d) {
                    classes.pop();
                }
            }
        }
    }

    fn push(
        graph: &mut CodeGraph,
        parent: &str,
        id: &str,
        path: &str,
        name: &str,
        kind: NodeKind,
        line: &str,
    ) {
        graph.add_node(NodeV1 {
            id: id.to_string(),
            path: path.to_string(),
            name: name.to_string(),
            kind,
            span: None,
            search_body: line.to_string(),
            file_residual: String::new(),
        });
        graph.add_edge(contains(parent, id));
    }

    fn extract_identifier_after_keyword(line: &str) -> Option<&str> {
        for kw in &FN_PREFIXES {
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

    /// Name after the first keyword of `keywords` the line starts with: up to the first
    /// delimiter (whitespace, `{`, `(`, `<`, `;`, `:`), so `struct Foo {`, `Foo<T>`,
    /// `Foo(i32)` and `struct Foo;` all give `Foo` (a bare "last word" gave `{` once).
    fn extract_type_name<'a>(line: &'a str, keywords: &[&str]) -> Option<&'a str> {
        for kw in keywords {
            if let Some(pos) = line.find(kw) {
                let after = &line[pos + kw.len()..];
                let name = after
                    .split(|c: char| c.is_whitespace() || matches!(c, '{' | '(' | '<' | ';' | ':'))
                    .next()?
                    .trim();
                if !name.is_empty() {
                    return Some(name);
                }
            }
        }
        None
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
