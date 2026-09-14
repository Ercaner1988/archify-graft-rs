//! graft-model: Core data models for Graft code intelligence.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeKind {
    File,
    Function,
    Method,
    Class,
    Interface,
    TypeAlias,
    Enum,
    Module,
    Constant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeV1 {
    pub id: String,
    pub path: String,
    pub name: String,
    pub kind: NodeKind,
    pub span: Option<Span>,
    pub search_body: String,
    pub file_residual: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeRelation {
    Calls,
    Extends,
    Implements,
    Contains,
    Imports,
    References,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeV1 {
    pub source: String,
    pub target: String,
    pub relation: EdgeRelation,
    pub confidence: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodeGraph {
    pub nodes: Vec<NodeV1>,
    pub edges: Vec<EdgeV1>,
    #[serde(skip)]
    node_index_map: HashMap<String, usize>,
}

impl CodeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: NodeV1) -> usize {
        let id = node.id.clone();
        let idx = self.nodes.len();
        self.nodes.push(node);
        self.node_index_map.insert(id, idx);
        idx
    }

    pub fn add_edge(&mut self, edge: EdgeV1) {
        self.edges.push(edge);
    }

    pub fn get_node_by_id(&self, id: &str) -> Option<&NodeV1> {
        self.node_index_map
            .get(id)
            .and_then(|&idx| self.nodes.get(idx))
    }

    pub fn rebuild_index(&mut self) {
        self.node_index_map.clear();
        for (idx, node) in self.nodes.iter().enumerate() {
            self.node_index_map.insert(node.id.clone(), idx);
        }
    }
}
