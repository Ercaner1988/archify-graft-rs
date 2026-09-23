//! AST Code Extractor: extracts files, structs, classes, functions, and methods.

use graft_model::{CodeGraph, EdgeRelation, EdgeV1, NodeKind, NodeV1};

pub struct AstExtractor;

/// One list for both "should this line even be inspected" and "where does the name
/// start" — a prefix here that is missing from the other call site is exactly the kind
/// of drift `extract_type_name`'s doc comment already warns about. Covers ESM's
/// `export`/`export default` variants and `async function`, which plain `"function "`
/// alone does not: real-world `.mjs` code is close to half `export function`/`async
/// function` declarations, not bare `function`.
const TYPE_PREFIXES: [&str; 5] = [
    "pub struct ",
    "struct ",
    "class ",
    "export class ",
    "export default class ",
];
const FN_PREFIXES: [&str; 9] = [
    "pub fn ",
    "fn ",
    "def ",
    "function ",
    "async function ",
    "export function ",
    "export async function ",
    "export default function ",
    "export default async function ",
];

impl AstExtractor {
    pub fn extract_content(path_str: &str, file_name: &str, bytes: &[u8]) -> CodeGraph {
        let content = String::from_utf8_lossy(bytes);

        let mut graph = CodeGraph::new();
        let file_node_id = format!("file:{}", path_str);
        graph.add_node(NodeV1 {
            id: file_node_id.clone(),
            path: path_str.to_string(),
            name: file_name.to_string(),
            kind: NodeKind::File,
            span: None,
            search_body: content.chars().take(4000).collect(),
            file_residual: content.chars().take(8000).collect(),
        });

        let mut current_class: Option<String> = None;

        for line in content.lines() {
            let trimmed = line.trim();

            if TYPE_PREFIXES.iter().any(|p| trimmed.starts_with(p)) {
                if let Some(name) = Self::extract_type_name(trimmed, &TYPE_PREFIXES) {
                    let class_id = format!("{}:{}", path_str, name);
                    current_class = Some(class_id.clone());

                    graph.add_node(NodeV1 {
                        id: class_id.clone(),
                        path: path_str.to_string(),
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

            if trimmed.starts_with("pub enum ") || trimmed.starts_with("enum ") {
                if let Some(name) = Self::extract_type_name(trimmed, &["pub enum ", "enum "]) {
                    let enum_id = format!("{}:{}", path_str, name);

                    graph.add_node(NodeV1 {
                        id: enum_id.clone(),
                        path: path_str.to_string(),
                        name: name.to_string(),
                        kind: NodeKind::Enum,
                        span: None,
                        search_body: trimmed.to_string(),
                        file_residual: String::new(),
                    });

                    graph.add_edge(EdgeV1 {
                        source: file_node_id.clone(),
                        target: enum_id,
                        relation: EdgeRelation::Contains,
                        confidence: 1.0,
                    });
                }
            }

            if trimmed.starts_with("pub trait ")
                || trimmed.starts_with("trait ")
                || trimmed.starts_with("interface ")
            {
                if let Some(name) =
                    Self::extract_type_name(trimmed, &["pub trait ", "trait ", "interface "])
                {
                    let iface_id = format!("{}:{}", path_str, name);

                    graph.add_node(NodeV1 {
                        id: iface_id.clone(),
                        path: path_str.to_string(),
                        name: name.to_string(),
                        kind: NodeKind::Interface,
                        span: None,
                        search_body: trimmed.to_string(),
                        file_residual: String::new(),
                    });

                    graph.add_edge(EdgeV1 {
                        source: file_node_id.clone(),
                        target: iface_id,
                        relation: EdgeRelation::Contains,
                        confidence: 1.0,
                    });
                }
            }

            if FN_PREFIXES.iter().any(|p| trimmed.starts_with(p)) {
                if let Some(fn_name) = Self::extract_identifier_after_keyword(trimmed) {
                    let fn_id = format!("{}:{}", path_str, fn_name);

                    graph.add_node(NodeV1 {
                        id: fn_id.clone(),
                        path: path_str.to_string(),
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
                        target: fn_id,
                        relation: EdgeRelation::Contains,
                        confidence: 1.0,
                    });
                }
            }
        }

        let specs = crate::imports::extract(path_str, &content);
        if !specs.is_empty() {
            graph.imports.insert(file_node_id, specs);
        }

        graph
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

    /// struct/enum/trait/class/interface bildirimlerinden isim çıkarır.
    ///
    /// ESKİ HATA (düzeltildi): `split_whitespace().last()` kullanılıyordu —
    /// `"pub struct Foo {"` gibi açan parantez BOŞLUKLA ayrıysa (standart
    /// biçim) son öğe `"{"` olur, `trim_end_matches('{')` boş string üretir.
    /// Sonuç: aynı dosyada birden çok düğüm `id = "yol:"` üzerinde çakışır
    /// (`CodeGraph::add_node` id çakışmasını kontrol etmez, `node_index_map`
    /// sessizce son ekleneni tutar). Bunun yerine anahtar kelimeden sonraki
    /// metni İLK sınırlayıcıya (boşluk, `{`, `(`, `<`, `;`, `:`) kadar alıyoruz
    /// — generic'ler (`Foo<T>`), tuple struct'lar (`Foo(i32)`) ve tek satır
    /// bildirimler (`struct Foo;`) hepsi doğru ayrışır.
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

    /// `export function`/`export async function`/`export class` are how most real
    /// `.mjs`/`.ts` modules declare their public surface. Before `FN_PREFIXES`/
    /// `TYPE_PREFIXES` covered them, only bare `function`/`class` were seen — on
    /// Archify's own source about a quarter of its declarations use these forms.
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
        let src = "pub fn alpha() {}\ndef beta():\n    pass\nstruct Foo { x: i32 }\n";
        let graph = AstExtractor::extract_content("a.rs", "a.rs", src.as_bytes());
        let found = names(&graph);
        for n in ["alpha", "beta", "Foo"] {
            assert!(found.contains(&n), "missing {n} in {found:?}");
        }
    }
}
