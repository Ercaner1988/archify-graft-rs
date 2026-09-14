//! Inter-symbol call resolver linking callers to function/method definitions.

use graft_model::{CodeGraph, EdgeRelation, EdgeV1, NodeKind};
use std::collections::{HashMap, HashSet};

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

                    let call_pattern_1 = format!("{}(", target_name);
                    let call_pattern_2 = format!("::{}", target_name);
                    let call_pattern_3 = format!(".{}(", target_name);

                    if node.search_body.contains(&call_pattern_1)
                        || node.search_body.contains(&call_pattern_2)
                        || node.search_body.contains(&call_pattern_3)
                    {
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
