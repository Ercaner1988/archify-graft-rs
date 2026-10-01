//! Crate nodes -> `DependsOn` edges (and `ExternalCrate` nodes for sibling repositories)
//! from the `Cargo.toml` specs kept in `CodeGraph::imports`.

use graft_cargo::{resolve, Manifest, Target};
use graft_model::{CodeGraph, EdgeRelation, EdgeV1, NodeKind, NodeV1};
use std::collections::HashMap;

/// Rebuilds every `DependsOn` edge and `ExternalCrate` node. Idempotent.
pub fn link(graph: &mut CodeGraph) {
    graph.nodes.retain(|n| n.kind != NodeKind::ExternalCrate);
    graph
        .edges
        .retain(|e| e.relation != EdgeRelation::DependsOn);

    // Every manifest, also a `[workspace]`-only root: `name.workspace = true` needs it.
    let manifests: Vec<(String, Manifest)> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::File && n.name == "Cargo.toml")
        .filter_map(|n| {
            let specs = graph.imports.get(&n.id)?;
            Some((n.path.clone(), Manifest::from_specs(specs)))
        })
        .collect();
    let node_of: HashMap<String, String> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Crate)
        .map(|n| (n.path.replace('\\', "/"), n.id.clone()))
        .collect();
    let resolved = resolve(manifests);
    let crate_ids: Vec<String> = resolved
        .crates
        .iter()
        .filter_map(|c| node_of.get(&c.manifest_path).cloned())
        .collect();
    if resolved.crates.len() != crate_ids.len() {
        return;
    }

    let mut external: HashMap<String, String> = HashMap::new();
    let mut edges: HashMap<(String, String), f32> = HashMap::new();
    for link in &resolved.links {
        let from = crate_ids[link.from].clone();
        let to = match &link.to {
            Target::Local(i) => crate_ids[*i].clone(),
            Target::External { name, dir, repo } => external
                .entry(format!("ext:{repo}/{name}"))
                .or_insert_with(|| {
                    let id = format!("ext:{repo}/{name}");
                    graph.nodes.push(NodeV1 {
                        id: id.clone(),
                        path: dir.clone(),
                        name: name.clone(),
                        kind: NodeKind::ExternalCrate,
                        span: None,
                        search_body: repo.clone(),
                        file_residual: String::new(),
                    });
                    id
                })
                .clone(),
        };
        // Optional (feature-gated) dependencies are drawn lighter than required ones.
        let weight = if link.optional { 0.5 } else { 1.0 };
        let slot = edges.entry((from, to)).or_insert(weight);
        *slot = slot.max(weight);
    }
    let mut edges: Vec<_> = edges.into_iter().collect();
    edges.sort_by(|a, b| a.0.cmp(&b.0));
    for ((source, target), confidence) in edges {
        graph.edges.push(EdgeV1 {
            source,
            target,
            relation: EdgeRelation::DependsOn,
            confidence,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add_crate(g: &mut CodeGraph, path: &str, toml: &str) {
        let m = Manifest::parse(toml).unwrap();
        g.imports.insert(format!("file:{path}"), m.to_specs());
        g.add_node(NodeV1 {
            id: format!("file:{path}"),
            path: path.into(),
            name: "Cargo.toml".into(),
            kind: NodeKind::File,
            span: None,
            search_body: String::new(),
            file_residual: String::new(),
        });
        g.add_node(NodeV1 {
            id: format!("{path}:{}", m.package.clone().unwrap()),
            path: path.into(),
            name: m.package.unwrap(),
            kind: NodeKind::Crate,
            span: None,
            search_body: String::new(),
            file_residual: String::new(),
        });
    }

    #[test]
    fn path_dependencies_become_depends_on_edges_and_sibling_repos_external_nodes() {
        let mut g = CodeGraph::new();
        add_crate(
            &mut g,
            "/r/app/core/Cargo.toml",
            "[package]\nname=\"core\"\n",
        );
        add_crate(
            &mut g,
            "/r/app/cli/Cargo.toml",
            "[package]\nname=\"cli\"\n[dependencies]\ncore={path=\"../core\"}\nkatla={path=\"../../katla\", optional=true}\nserde=\"1\"\n",
        );
        link(&mut g);
        link(&mut g);
        let deps: Vec<(&str, &str, f32)> = g
            .edges
            .iter()
            .map(|e| (e.source.as_str(), e.target.as_str(), e.confidence))
            .collect();
        assert_eq!(
            deps,
            [
                (
                    "/r/app/cli/Cargo.toml:cli",
                    "/r/app/core/Cargo.toml:core",
                    1.0
                ),
                ("/r/app/cli/Cargo.toml:cli", "ext:katla/katla", 0.5),
            ]
        );
        assert_eq!(
            g.nodes
                .iter()
                .filter(|n| n.kind == NodeKind::ExternalCrate)
                .count(),
            1
        );
    }
}
