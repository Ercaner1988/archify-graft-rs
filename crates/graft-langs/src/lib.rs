//! graft-langs: reads JS/TS, Python, Go, Java and C/C++ source the way `graft-rust` reads
//! Rust: declarations with body spans, call sites, what each file imports, and (in
//! [`link`]) the graph-wide `Imports` and `Calls` edges built from them.

mod bindings;
mod calls;
mod items;
mod lang;

pub use calls::{is_std_method, is_std_receiver};
pub use lang::Lang;

use graft_model::{CodeGraph, EdgeRelation, EdgeV1, NodeKind, NodeV1, Span};
use items::Kind;
use std::collections::HashMap;

/// `tests/`, `__tests__/`, `*.test.js`, `*_test.go`, `test_*.py`, `FooTest.java` ...
fn is_test_file(path: &str) -> bool {
    let p = path.replace('\\', "/");
    let name = p.rsplit('/').next().unwrap_or("");
    p.split('/')
        .any(|s| matches!(s, "tests" | "test" | "__tests__" | "spec" | "specs" | "e2e"))
        || name.contains(".test.")
        || name.contains(".spec.")
        || name.ends_with("_test.go")
        || name.ends_with("_test.py")
        || name.starts_with("test_")
        || name.ends_with("Test.java")
        || name.ends_with("Tests.java")
}

/// Adds the symbols, call facts and import bindings of one file to `graph` (a small
/// per-file graph that already holds the file node). Returns false for other languages.
pub fn extract(graph: &mut CodeGraph, path: &str, file_id: &str, content: &str) -> bool {
    let Some(lang) = Lang::of_path(path) else {
        return false;
    };
    let masked = lang::mask(lang, content);
    let found = items::scan(lang, content, &masked);
    let test_file = is_test_file(path);

    let mut file_facts: Vec<String> = vec![format!(
        "loc:{}",
        masked.lines().filter(|l| !l.trim().is_empty()).count()
    )];
    file_facts.extend(bindings::extract(lang, content));
    if matches!(lang, Lang::Java | Lang::C | Lang::Go) {
        file_facts.extend(type_names(&masked).into_iter().map(|t| format!("ty:{t}")));
    }
    graph.facts.insert(file_id.to_string(), file_facts);

    let mut classes: HashMap<String, String> = HashMap::new();
    let mut used: HashMap<String, usize> = HashMap::new();
    for it in &found {
        let base = match (&it.kind, &it.owner) {
            (Kind::Fn, Some(o)) => format!("{path}:{o}::{}", it.name),
            _ => format!("{path}:{}", it.name),
        };
        let n = used.entry(base.clone()).or_insert(0);
        *n += 1;
        let id = if *n == 1 { base } else { format!("{base}#{n}") };
        let kind = match (&it.kind, &it.owner) {
            (_, _) if test_file && it.kind == Kind::Fn => NodeKind::Test,
            (Kind::Fn, Some(_)) => NodeKind::Method,
            (Kind::Fn, None) => NodeKind::Function,
            (Kind::Class, _) => NodeKind::Class,
            (Kind::Enum, _) => NodeKind::Enum,
            (Kind::Interface, _) => NodeKind::Interface,
        };
        let parent = it
            .owner
            .as_ref()
            .and_then(|o| classes.get(o))
            .map_or(file_id, String::as_str)
            .to_string();
        if it.kind != Kind::Fn {
            classes.entry(it.name.clone()).or_insert_with(|| id.clone());
        }
        graph.add_node(NodeV1 {
            id: id.clone(),
            path: path.to_string(),
            name: it.name.clone(),
            kind: kind.clone(),
            span: Some(Span {
                start_line: it.line,
                start_col: 0,
                end_line: it.end_line,
                end_col: 0,
            }),
            search_body: it.sig.clone(),
            file_residual: String::new(),
        });
        graph.add_edge(EdgeV1 {
            source: parent,
            target: id.clone(),
            relation: EdgeRelation::Contains,
            confidence: 1.0,
        });
        if it.kind != Kind::Fn && it.is_pub {
            graph.facts.insert(id.clone(), vec!["pub".into()]);
        }
        if it.kind == Kind::Fn && kind != NodeKind::Test {
            let mut facts: Vec<String> = it.calls.iter().map(|c| format!("c:{c}")).collect();
            if it.is_pub {
                facts.push("pub".into());
            }
            if !facts.is_empty() {
                graph.facts.insert(id, facts);
            }
        }
    }
    true
}

/// Capitalized identifiers of a file (types it can name), for the typed languages.
fn type_names(masked: &str) -> Vec<String> {
    let mut set = std::collections::BTreeSet::new();
    for w in masked.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
        if w.len() >= 3 && w.chars().next().is_some_and(char::is_uppercase) {
            set.insert(w.to_string());
        }
    }
    set.into_iter().take(400).collect()
}
