//! graft-model: Core data models for Graft code intelligence.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

pub mod ikili;
mod wiring;

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
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
    // Appended (the archive stores the variant index, so older caches still decode).
    /// A Cargo package of the indexed tree (`path` = its Cargo.toml).
    Crate,
    /// A path dependency that lives outside the indexed tree (sibling repository).
    ExternalCrate,
    /// A data file the code reads or writes (`envanter.bin`); `path` is empty.
    Artifact,
    /// A function or type that only exists for tests (`#[test]`, `#[cfg(test)]`, `tests/`).
    Test,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct Span {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct NodeV1 {
    pub id: String,
    pub path: String,
    pub name: String,
    pub kind: NodeKind,
    pub span: Option<Span>,
    pub search_body: String,
    pub file_residual: String,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub enum EdgeRelation {
    Calls,
    Extends,
    Implements,
    Contains,
    Imports,
    References,
    /// Crate -> crate (Cargo `[dependencies]`).
    DependsOn,
    /// Function -> artifact it reads.
    Reads,
    /// Function -> artifact it writes.
    Writes,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct EdgeV1 {
    pub source: String,
    pub target: String,
    pub relation: EdgeRelation,
    pub confidence: f32,
}

#[derive(
    Debug, Clone, Default, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct CodeGraph {
    pub nodes: Vec<NodeV1>,
    pub edges: Vec<EdgeV1>,
    /// Raw `use`/`import` specs per file node id. `Imports` edges are derived from
    /// this map from scratch whenever the file set changes, so an incremental update
    /// never loses the links of files it did not re-read. BTreeMap: stable cache bytes.
    pub imports: BTreeMap<String, Vec<String>>,
    /// Tagged facts per node id that the graph-wide resolvers consume: `c:<call token>`,
    /// `a:<artifact file name>`, `io:r`, `io:w` on functions; `loc:<n>` on files. Like
    /// `imports`, edges are derived from it from scratch, so nothing goes stale.
    pub facts: BTreeMap<String, Vec<String>>,
    #[serde(skip)]
    #[rkyv(with = rkyv::with::Skip)]
    node_index_map: HashMap<String, usize>,
}

/// `graft/.graph/wiring.bin`: the small versioned record other tools read to learn
/// that a repository was indexed. The layout is hand-specified (see `wiring.rs`), not
/// tied to a serialization crate: FIELD ORDER IS THE FORMAT, readers mirror this struct
/// exactly and `version` bumps on any change.
#[derive(Debug, Clone, PartialEq, Eq)]
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

    /// Drops everything that came from one file: its node, its symbols, their edges, raw
    /// imports and facts. The `path:` prefix (not a bare path prefix) keeps `a.rs` from
    /// swallowing `a.rs2`.
    pub fn forget_file(&mut self, path: &str) {
        let file_id = format!("file:{path}");
        let prefix = format!("{path}:");
        let mine = |id: &str| id == file_id || id.starts_with(&prefix);
        self.nodes.retain(|n| !mine(&n.id));
        self.edges.retain(|e| !mine(&e.source) && !mine(&e.target));
        self.imports.remove(&file_id);
        self.facts.retain(|k, _| !mine(k));
    }

    /// Adds everything another (single-file) graph carries.
    pub fn absorb(&mut self, other: CodeGraph) {
        for node in other.nodes {
            self.add_node(node);
        }
        self.edges.extend(other.edges);
        self.imports.extend(other.imports);
        self.facts.extend(other.facts);
    }

    pub fn rebuild_index(&mut self) {
        self.node_index_map.clear();
        for (idx, node) in self.nodes.iter().enumerate() {
            self.node_index_map.insert(node.id.clone(), idx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, path: &str, kind: NodeKind) -> NodeV1 {
        NodeV1 {
            id: id.into(),
            path: path.into(),
            name: id.into(),
            kind,
            span: None,
            search_body: String::new(),
            file_residual: String::new(),
        }
    }

    #[test]
    fn forget_file_removes_only_that_file_and_not_prefix_lookalikes() {
        let mut g = CodeGraph::new();
        g.add_node(node("file:a.rs", "a.rs", NodeKind::File));
        g.add_node(node("a.rs:f", "a.rs", NodeKind::Function));
        g.add_node(node("file:a.rs2", "a.rs2", NodeKind::File));
        g.add_edge(EdgeV1 {
            source: "file:a.rs".into(),
            target: "a.rs:f".into(),
            relation: EdgeRelation::Contains,
            confidence: 1.0,
        });
        g.facts.insert("a.rs:f".into(), vec!["c:g".into()]);
        g.facts.insert("file:a.rs2".into(), vec!["loc:3".into()]);
        g.forget_file("a.rs");
        assert_eq!(g.nodes.len(), 1);
        assert!(g.edges.is_empty());
        assert_eq!(g.facts.len(), 1);
    }
}
