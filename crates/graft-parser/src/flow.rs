//! Data flow: which functions read or write which data files (`envanter.bin`).
//! Facts come from `graft-rust` (`a:<file>` literal in the body, `io:r` / `io:w` verbs);
//! here they become `Artifact` nodes and `Reads` / `Writes` edges. A function that does
//! the I/O but gets its path from a helper (`fn yol() -> PathBuf { .. "x.bin" }`)
//! inherits the file names of the helpers it calls.

use graft_model::{CodeGraph, EdgeRelation, EdgeV1, NodeKind, NodeV1};
use std::collections::{BTreeMap, BTreeSet, HashMap};

fn arts(facts: &[String]) -> Vec<&str> {
    facts.iter().filter_map(|f| f.strip_prefix("a:")).collect()
}

/// Rebuilds every `Artifact` node and `Reads` / `Writes` edge. Idempotent.
pub fn link(graph: &mut CodeGraph) {
    graph.nodes.retain(|n| n.kind != NodeKind::Artifact);
    graph
        .edges
        .retain(|e| !matches!(e.relation, EdgeRelation::Reads | EdgeRelation::Writes));

    let mut callees: HashMap<&str, Vec<&str>> = HashMap::new();
    for e in graph
        .edges
        .iter()
        .filter(|e| e.relation == EdgeRelation::Calls)
    {
        callees
            .entry(e.source.as_str())
            .or_default()
            .push(e.target.as_str());
    }
    let mut found: BTreeMap<String, BTreeSet<(String, bool)>> = BTreeMap::new();
    for n in graph
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Function | NodeKind::Method))
    {
        let Some(facts) = graph.facts.get(&n.id) else {
            continue;
        };
        let has = |tag: &str| facts.iter().any(|f| f == tag);
        let (reads, writes) = (has("io:r"), has("io:w"));
        if !reads && !writes {
            continue;
        }
        let mut names: Vec<&str> = arts(facts);
        if names.is_empty() {
            for callee in callees.get(n.id.as_str()).into_iter().flatten() {
                if let Some(f) = graph.facts.get(*callee) {
                    names.extend(arts(f));
                }
            }
        }
        for a in names {
            let set = found.entry(a.to_string()).or_default();
            if writes {
                set.insert((n.id.clone(), true));
            }
            if reads {
                set.insert((n.id.clone(), false));
            }
        }
    }
    for (name, users) in found {
        let id = format!("artifact:{name}");
        graph.nodes.push(NodeV1 {
            id: id.clone(),
            path: String::new(),
            name: name.clone(),
            kind: NodeKind::Artifact,
            span: None,
            search_body: name,
            file_residual: String::new(),
        });
        for (fn_id, write) in users {
            graph.edges.push(EdgeV1 {
                source: fn_id,
                target: id.clone(),
                relation: if write {
                    EdgeRelation::Writes
                } else {
                    EdgeRelation::Reads
                },
                confidence: 0.9,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn func(id: &str) -> NodeV1 {
        NodeV1 {
            id: id.into(),
            path: "a.rs".into(),
            name: id.rsplit(':').next().unwrap().into(),
            kind: NodeKind::Function,
            span: None,
            search_body: String::new(),
            file_residual: String::new(),
        }
    }

    #[test]
    fn writers_readers_and_path_helpers_meet_at_one_artifact() {
        let mut g = CodeGraph::new();
        for id in ["a.rs:uret", "a.rs:oku", "a.rs:yol", "a.rs:diger"] {
            g.add_node(func(id));
        }
        g.facts.insert(
            "a.rs:uret".into(),
            vec!["a:envanter.bin".into(), "io:w".into()],
        );
        g.facts.insert("a.rs:oku".into(), vec!["io:r".into()]);
        g.facts
            .insert("a.rs:yol".into(), vec!["a:envanter.bin".into()]);
        g.facts.insert("a.rs:diger".into(), vec!["a:x.json".into()]);
        g.add_edge(EdgeV1 {
            source: "a.rs:oku".into(),
            target: "a.rs:yol".into(),
            relation: EdgeRelation::Calls,
            confidence: 0.9,
        });
        link(&mut g);
        link(&mut g);
        let mut e: Vec<String> = g
            .edges
            .iter()
            .filter(|e| e.relation != EdgeRelation::Calls)
            .map(|e| format!("{}-{:?}->{}", e.source, e.relation, e.target))
            .collect();
        e.sort();
        assert_eq!(
            e,
            [
                "a.rs:oku-Reads->artifact:envanter.bin",
                "a.rs:uret-Writes->artifact:envanter.bin"
            ]
        );
    }
}
