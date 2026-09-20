//! graft-model: Core data models for Graft code intelligence.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

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
    /// Raw `use`/`import` specs per file node id. `Imports` edges are derived from
    /// this map from scratch whenever the file set changes, so an incremental update
    /// never loses the links of files it did not re-read. BTreeMap: stable cache bytes.
    pub imports: BTreeMap<String, Vec<String>>,
    #[serde(skip)]
    node_index_map: HashMap<String, usize>,
}

/// `graft/.graph/wiring.bin`: the small versioned record other tools read to learn
/// that a repository was indexed. bincode is not self-describing, so FIELD ORDER IS
/// THE FORMAT: readers mirror this struct exactly and `version` bumps on any change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WiringMeta {
    pub version: u32,
    pub node_count: u64,
    pub edge_count: u64,
    pub languages: Vec<String>,
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
