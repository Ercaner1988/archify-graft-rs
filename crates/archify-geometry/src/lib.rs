//! archify-geometry: Orthogonal layout geometry, port spread, and topologic routing.

use archify_ir::ArchitectureDiagram;
use petgraph::algo::astar;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

pub struct ReachabilityEngine;

impl ReachabilityEngine {
    /// Finds the shortest directed path between two components, returning the ordered node IDs
    pub fn find_route(
        diagram: &ArchitectureDiagram,
        start_id: &str,
        target_id: &str,
    ) -> Option<Vec<String>> {
        let mut graph = DiGraph::<String, ()>::new();
        let mut id_to_index: HashMap<&str, NodeIndex> = HashMap::new();
        let mut index_to_id: HashMap<NodeIndex, String> = HashMap::new();

        for comp in &diagram.components {
            let idx = graph.add_node(comp.id.clone());
            id_to_index.insert(&comp.id, idx);
            index_to_id.insert(idx, comp.id.clone());
        }

        for conn in &diagram.connections {
            if let (Some(&from_idx), Some(&to_idx)) = (
                id_to_index.get(conn.from.as_str()),
                id_to_index.get(conn.to.as_str()),
            ) {
                graph.add_edge(from_idx, to_idx, ());
            }
        }

        let start_idx = *id_to_index.get(start_id)?;
        let target_idx = *id_to_index.get(target_id)?;

        if let Some((_cost, path)) = astar(
            &graph,
            start_idx,
            |finish| finish == target_idx,
            |_| 1,
            |_| 0,
        ) {
            let result: Vec<String> = path
                .into_iter()
                .filter_map(|idx| index_to_id.get(&idx).cloned())
                .collect();
            Some(result)
        } else {
            None
        }
    }
}
