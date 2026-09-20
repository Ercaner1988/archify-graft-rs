//! Inter-symbol call resolver linking callers to function/method definitions.

use graft_model::{CodeGraph, EdgeRelation, EdgeV1, NodeKind};
use std::collections::{HashMap, HashSet};

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// True when `body` calls or references `name`: `name(`, `.name(` or `::name`, with
/// no identifier character right before it. A plain `contains("oku(")` also matched
/// `envanter_oku(`, which linked unrelated functions across crates.
fn calls(body: &str, name: &str) -> bool {
    let mut from = 0;
    while let Some(i) = body[from..].find(name) {
        let (start, end) = (from + i, from + i + name.len());
        let left_ok = body[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !is_ident(c));
        let paren = body[end..].starts_with('(');
        let path_ref = body[..start].ends_with("::")
            && body[end..].chars().next().is_none_or(|c| !is_ident(c));
        if left_ok && (paren || path_ref) {
            return true;
        }
        from = end;
    }
    false
}

pub struct CallResolver;

impl CallResolver {
    /// Discovers call references inside function bodies and links them to targets
    pub fn resolve_calls(graph: &mut CodeGraph) {
        let mut name_to_ids: HashMap<String, Vec<String>> = HashMap::new();
        for node in &graph.nodes {
            if node.kind == NodeKind::Function || node.kind == NodeKind::Method {
                name_to_ids
                    .entry(node.name.clone())
                    .or_default()
                    .push(node.id.clone());
            }
        }

        let mut new_edges = Vec::new();
        let mut existing_pairs: HashSet<(String, String)> = HashSet::new();

        for edge in &graph.edges {
            existing_pairs.insert((edge.source.clone(), edge.target.clone()));
        }

        for node in &graph.nodes {
            if node.kind == NodeKind::Function || node.kind == NodeKind::Method {
                for (target_name, target_ids) in &name_to_ids {
                    if *target_name == node.name {
                        continue;
                    }

                    if calls(&node.search_body, target_name) {
                        for target_id in target_ids {
                            let pair = (node.id.clone(), target_id.clone());
                            if !existing_pairs.contains(&pair) {
                                existing_pairs.insert(pair);
                                new_edges.push(EdgeV1 {
                                    source: node.id.clone(),
                                    target: target_id.clone(),
                                    relation: EdgeRelation::Calls,
                                    confidence: 0.85,
                                });
                            }
                        }
                    }
                }
            }
        }

        for edge in new_edges {
            graph.add_edge(edge);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_inside_a_longer_identifier_is_not_a_call() {
        assert!(!calls("let x = envanter_oku(kok);", "oku"));
        assert!(!calls("dugum_ciz(ui)", "ciz"));
        assert!(!calls("oku_hepsi(x)", "oku"));
        assert!(!calls("let oku = 1;", "oku"));
    }

    #[test]
    fn plain_method_and_path_calls_are_found() {
        assert!(calls("oku(kok)", "oku"));
        assert!(calls("let x = self.oku(kok);", "oku"));
        assert!(calls("crate::modul::oku(kok)", "oku"));
        assert!(calls(".map(modul::oku)", "oku"));
        assert!(calls("if a { x } else { oku(y) }", "oku"));
    }

    #[test]
    fn a_later_occurrence_is_found_after_a_rejected_one() {
        assert!(calls("envanter_oku(a); oku(b)", "oku"));
        assert!(calls("çağır(); çağır_bunu(); x.çağır(1)", "çağır"));
    }
}
