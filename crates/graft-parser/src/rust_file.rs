//! Turns one Rust file (via `graft-rust`) or one `Cargo.toml` into graph nodes, edges
//! and facts. Only per-file knowledge lives here; cross-file links are made by the
//! resolvers once the whole file set is known.

use crate::extractor::contains;
use graft_cargo::Manifest;
use graft_model::{CodeGraph, NodeKind, NodeV1, Span};
use graft_rust::{mask, scan, ItemKind};
use std::collections::HashMap;

/// `tests/`, `benches/`, `examples/` folders and `tests.rs` files are not the product.
fn is_test_file(path: &str) -> bool {
    let p = path.replace('\\', "/");
    p.split('/')
        .any(|s| matches!(s, "tests" | "benches" | "examples"))
        || p.ends_with("/tests.rs")
}

pub fn manifest(graph: &mut CodeGraph, path: &str, file_id: &str, content: &str) {
    let Some(m) = Manifest::parse(content) else {
        return;
    };
    graph.imports.insert(file_id.to_string(), m.to_specs());
    if let Some(pkg) = &m.package {
        let id = format!("{path}:{pkg}");
        graph.add_node(NodeV1 {
            id: id.clone(),
            path: path.to_string(),
            name: pkg.clone(),
            kind: NodeKind::Crate,
            span: None,
            search_body: pkg.clone(),
            file_residual: String::new(),
        });
        graph.add_edge(contains(file_id, &id));
    }
}

pub fn extract(graph: &mut CodeGraph, path: &str, file_id: &str, content: &str) {
    let masked = mask(content);
    let test_file = is_test_file(path);
    let scanned = scan(content, &masked, test_file);
    graph.facts.insert(
        file_id.to_string(),
        vec![format!("loc:{}", scanned.code_lines)],
    );

    let specs = crate::imports::extract(path, content, Some(&masked.text));
    if !specs.is_empty() {
        graph.imports.insert(file_id.to_string(), specs);
    }

    // Node id of each type in this file, so a method hangs below its `impl` type.
    let mut types: HashMap<String, String> = HashMap::new();
    let mut used: HashMap<String, usize> = HashMap::new();
    for item in &scanned.items {
        let base = match (&item.kind, &item.owner) {
            (ItemKind::Fn, Some(owner)) => format!("{path}:{owner}::{}", item.name),
            _ => format!("{path}:{}", item.name),
        };
        let n = used.entry(base.clone()).or_insert(0);
        *n += 1;
        let id = if *n == 1 { base } else { format!("{base}#{n}") };
        let kind = match (item.in_test, &item.kind, &item.owner) {
            (true, _, _) => NodeKind::Test,
            (_, ItemKind::Fn, Some(_)) => NodeKind::Method,
            (_, ItemKind::Fn, None) => NodeKind::Function,
            (_, ItemKind::Struct, _) => NodeKind::Class,
            (_, ItemKind::Enum, _) => NodeKind::Enum,
            (_, ItemKind::Trait, _) => NodeKind::Interface,
        };
        let parent = match (&item.kind, &item.owner) {
            (ItemKind::Fn, Some(owner)) => types.get(owner).map_or(file_id, String::as_str),
            _ => file_id,
        }
        .to_string();
        if item.kind != ItemKind::Fn {
            types.entry(item.name.clone()).or_insert_with(|| id.clone());
        }
        graph.add_node(NodeV1 {
            id: id.clone(),
            path: path.to_string(),
            name: item.name.clone(),
            kind,
            span: Some(Span {
                start_line: item.line,
                start_col: 0,
                end_line: item.end_line,
                end_col: 0,
            }),
            search_body: item.sig.clone(),
            file_residual: String::new(),
        });
        graph.add_edge(contains(&parent, &id));

        let f = &item.facts;
        if item.kind == ItemKind::Fn && !item.in_test {
            let mut facts: Vec<String> = f.calls.iter().map(|c| format!("c:{c}")).collect();
            facts.extend(f.artifacts.iter().map(|a| format!("a:{a}")));
            if f.reads {
                facts.push("io:r".into());
            }
            if f.writes {
                facts.push("io:w".into());
            }
            if item.is_pub {
                facts.push("pub".into());
            }
            if !facts.is_empty() {
                graph.facts.insert(id, facts);
            }
        }
    }
}
