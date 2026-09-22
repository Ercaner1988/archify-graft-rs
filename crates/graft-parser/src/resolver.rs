//! Inter-symbol call resolver linking callers to function/method definitions.

use graft_model::{CodeGraph, EdgeRelation, EdgeV1, NodeKind};
use std::collections::{HashMap, HashSet};

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// A function's name without a generic suffix: `baslat<F>` is called as `baslat(`.
fn base_name(name: &str) -> &str {
    name.split('<').next().unwrap_or(name)
}

/// Identifiers of `body` used like a call: `name(`, `.name(` or `::name`. A token is
/// a maximal identifier run, so `envanter_oku(` never yields `oku` (a plain
/// `contains("oku(")` linked unrelated functions across crates). One pass over the
/// text: resolving a whole graph is linear in its size, where trying every known
/// function name against every body took minutes on a 30k-node repository.
fn called_names(body: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = None;
    // The sentinel closes a token that ends the text.
    for (i, c) in body
        .char_indices()
        .chain(std::iter::once((body.len(), ' ')))
    {
        match (is_ident(c), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                if body[i..].starts_with('(') || body[..s].ends_with("::") {
                    out.push(&body[s..i]);
                }
                start = None;
            }
            _ => {}
        }
    }
    out
}

pub struct CallResolver;

impl CallResolver {
    /// Discovers call references inside function bodies and links them to targets
    pub fn resolve_calls(graph: &mut CodeGraph) {
        let is_fn =
            |n: &graft_model::NodeV1| n.kind == NodeKind::Function || n.kind == NodeKind::Method;
        let mut name_to_ids: HashMap<&str, Vec<&str>> = HashMap::new();
        for node in graph.nodes.iter().filter(|n| is_fn(n)) {
            name_to_ids
                .entry(base_name(&node.name))
                .or_default()
                .push(&node.id);
        }

        let mut existing_pairs: HashSet<(&str, &str)> = graph
            .edges
            .iter()
            .map(|e| (e.source.as_str(), e.target.as_str()))
            .collect();

        let mut new_edges = Vec::new();
        for node in graph.nodes.iter().filter(|n| is_fn(n)) {
            let own = base_name(&node.name);
            for called in called_names(&node.search_body) {
                if called == own {
                    continue;
                }
                let Some(target_ids) = name_to_ids.get(called) else {
                    continue;
                };
                for &target_id in target_ids {
                    if existing_pairs.insert((&node.id, target_id)) {
                        new_edges.push(EdgeV1 {
                            source: node.id.clone(),
                            target: target_id.to_string(),
                            relation: EdgeRelation::Calls,
                            confidence: 0.85,
                        });
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
    use graft_model::NodeV1;

    fn calls(body: &str, name: &str) -> bool {
        called_names(body).contains(&name)
    }

    fn function(id: &str, name: &str, body: &str) -> NodeV1 {
        NodeV1 {
            id: id.to_string(),
            path: id.to_string(),
            name: name.to_string(),
            kind: NodeKind::Function,
            span: None,
            search_body: body.to_string(),
            file_residual: String::new(),
        }
    }

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

    #[test]
    fn a_generic_function_is_called_by_its_bare_name() {
        let mut g = CodeGraph::new();
        g.add_node(function("a.rs:baslat<F>", "baslat<F>", "fn baslat<F>() {"));
        g.add_node(function("b.rs:run", "run", "fn run() { baslat(1) }"));
        CallResolver::resolve_calls(&mut g);
        assert_eq!(g.edges.len(), 1);
        assert_eq!(g.edges[0].target, "a.rs:baslat<F>");
    }

    /// The old loop tried every known name against every body: 20k functions meant
    /// 400M searches (minutes). One pass over the text finishes in milliseconds.
    #[test]
    fn resolving_a_large_graph_is_linear() {
        let mut g = CodeGraph::new();
        for i in 0..20_000 {
            let next = (i + 1) % 20_000;
            g.add_node(function(
                &format!("f{i}.rs:fn_{i}"),
                &format!("fn_{i}"),
                &format!("fn fn_{i}() {{ fn_{next}(); }}"),
            ));
        }
        let start = std::time::Instant::now();
        CallResolver::resolve_calls(&mut g);
        assert_eq!(g.edges.len(), 20_000);
        assert!(start.elapsed().as_secs() < 10, "resolver is not linear");
    }
}
