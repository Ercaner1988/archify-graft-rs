//! Call sequence from one function: who calls whom, three calls deep.

use crate::labels::relation_label;
use archify_ir::{
    DiagramMeta, SemanticRole, SequenceDiagram, SequenceMessage, SequenceParticipant, VisualPreset,
};
use graft_model::{CodeGraph, EdgeRelation, NodeKind, NodeV1};
use std::collections::{HashMap, HashSet};

const MAX_DEPTH: usize = 3;
const MAX_MESSAGES: usize = 30;

fn role_of(n: &NodeV1) -> SemanticRole {
    crate::labels::role_for(&n.name, &n.path)
}

/// The function named `root` (name or id): tests are ignored, `main.rs` wins ties, then
/// the one that calls the most.
fn find_root<'a>(
    graph: &'a CodeGraph,
    root: &str,
    out: &HashMap<&str, Vec<&str>>,
) -> Option<&'a NodeV1> {
    graph
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Function | NodeKind::Method))
        .filter(|n| n.name == root || n.id == root || n.id.ends_with(&format!(":{root}")))
        .max_by_key(|n| {
            (
                n.path.ends_with("main.rs"),
                out.get(n.id.as_str()).map_or(0, Vec::len),
            )
        })
}

pub fn compile(graph: &CodeGraph, root_fn: &str, title: &str, locale: &str) -> SequenceDiagram {
    let by_id: HashMap<&str, &NodeV1> = graph.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut out: HashMap<&str, Vec<&str>> = HashMap::new();
    for e in graph
        .edges
        .iter()
        .filter(|e| e.relation == EdgeRelation::Calls)
    {
        out.entry(e.source.as_str())
            .or_default()
            .push(e.target.as_str());
    }
    let root = find_root(graph, root_fn, &out);
    let root_id = root.map_or_else(|| root_fn.to_string(), |n| n.id.clone());

    let mut participants = vec![SequenceParticipant {
        id: root_id.clone(),
        label: root.map_or_else(|| root_fn.to_string(), |n| n.name.clone()),
        role: SemanticRole::Frontend,
    }];
    let mut seen: HashSet<String> = HashSet::from([root_id.clone()]);
    let mut messages = Vec::new();
    // Depth-first, so the messages read in the order the calls happen.
    let mut stack = vec![(root_id.clone(), 0usize)];
    let mut expanded: HashSet<String> = HashSet::new();
    while let Some((caller, depth)) = stack.pop() {
        if depth >= MAX_DEPTH || !expanded.insert(caller.clone()) {
            continue;
        }
        let callees = out.get(caller.as_str()).cloned().unwrap_or_default();
        let mut next = Vec::new();
        for callee in callees {
            let Some(n) = by_id.get(callee) else { continue };
            if messages.len() >= MAX_MESSAGES {
                break;
            }
            if seen.insert(n.id.clone()) {
                participants.push(SequenceParticipant {
                    id: n.id.clone(),
                    label: n.name.clone(),
                    role: role_of(n),
                });
            }
            messages.push(SequenceMessage {
                order: messages.len() + 1,
                from: caller.clone(),
                to: n.id.clone(),
                action: format!(
                    "{} {}()",
                    relation_label(&EdgeRelation::Calls, locale),
                    n.name
                ),
                is_async: n.search_body.contains("async "),
            });
            next.push((n.id.clone(), depth + 1));
        }
        stack.extend(next.into_iter().rev());
    }
    SequenceDiagram {
        meta: DiagramMeta {
            title: title.to_string(),
            subtitle: Some(match locale {
                "tr" => format!("'{root_fn}' çağrı dizisi · {} mesaj", messages.len()),
                "ar" => format!("تسلسل استدعاءات '{root_fn}' · {} رسالة", messages.len()),
                _ => format!("Call sequence of '{root_fn}' · {} messages", messages.len()),
            }),
            locale: locale.to_string(),
            visual_preset: VisualPreset::Editorial,
        },
        participants,
        messages,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_model::EdgeV1;

    fn f(id: &str, path: &str, name: &str, sig: &str) -> NodeV1 {
        NodeV1 {
            id: id.into(),
            path: path.into(),
            name: name.into(),
            kind: NodeKind::Function,
            span: None,
            search_body: sig.into(),
            file_residual: String::new(),
        }
    }

    fn call(a: &str, b: &str) -> EdgeV1 {
        EdgeV1 {
            source: a.into(),
            target: b.into(),
            relation: EdgeRelation::Calls,
            confidence: 0.9,
        }
    }

    #[test]
    fn calls_are_followed_three_levels_deep_in_call_order() {
        let mut g = CodeGraph::new();
        for (id, sig) in [
            ("a.rs:main", ""),
            ("a.rs:b", ""),
            ("a.rs:c", "async fn c()"),
            ("a.rs:d", ""),
            ("a.rs:e", ""),
        ] {
            g.add_node(f(id, "a.rs", id.rsplit(':').next().unwrap(), sig));
        }
        for (a, b) in [
            ("a.rs:main", "a.rs:b"),
            ("a.rs:b", "a.rs:c"),
            ("a.rs:c", "a.rs:d"),
            ("a.rs:d", "a.rs:e"),
        ] {
            g.add_edge(call(a, b));
        }
        let s = compile(&g, "main", "t", "en");
        let order: Vec<&str> = s.messages.iter().map(|m| m.action.as_str()).collect();
        assert_eq!(order, ["calls b()", "calls c()", "calls d()"]);
        assert!(s.messages[1].is_async);
        assert_eq!(s.participants.len(), 4);
    }

    #[test]
    fn the_main_of_main_rs_beats_a_bench_main() {
        let mut g = CodeGraph::new();
        g.add_node(f("benches/x.rs:main", "benches/x.rs", "main", ""));
        g.add_node(f("src/main.rs:main", "src/main.rs", "main", ""));
        g.add_node(f("src/main.rs:run", "src/main.rs", "run", ""));
        g.add_edge(call("src/main.rs:main", "src/main.rs:run"));
        let s = compile(&g, "main", "t", "tr");
        assert_eq!(s.messages.len(), 1);
    }
}
