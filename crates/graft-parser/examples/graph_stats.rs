//! `cargo run -p graft-parser --example graph_stats -- <repo dir>`: prints what the
//! indexer found (kinds, relations, crate edges, call hubs, artifacts) so graph quality
//! can be measured without a diagram in the way.

use graft_model::{EdgeRelation, NodeKind};
use graft_parser::CodeExtractor;
use std::collections::BTreeMap;

fn main() {
    let root = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let t = std::time::Instant::now();
    let g = CodeExtractor::index_directory(&root).expect("index");
    println!("indexed {root} in {:?}", t.elapsed());

    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    for n in &g.nodes {
        *kinds.entry(format!("{:?}", n.kind)).or_default() += 1;
    }
    let mut rels: BTreeMap<String, usize> = BTreeMap::new();
    for e in &g.edges {
        *rels.entry(format!("{:?}", e.relation)).or_default() += 1;
    }
    println!("nodes {kinds:?}");
    println!("edges {rels:?}");

    let name_of = |id: &str| {
        g.nodes
            .iter()
            .find(|n| n.id == id)
            .map_or(id.to_string(), |n| match n.kind {
                NodeKind::Crate | NodeKind::ExternalCrate | NodeKind::Artifact => n.name.clone(),
                _ => format!(
                    "{}:{}",
                    n.path.rsplit(['/', '\\']).next().unwrap_or(""),
                    n.name
                ),
            })
    };
    println!("-- DependsOn");
    for e in g
        .edges
        .iter()
        .filter(|e| e.relation == EdgeRelation::DependsOn)
    {
        println!(
            "  {} -> {} ({})",
            name_of(&e.source),
            name_of(&e.target),
            e.confidence
        );
    }
    let mut hubs: BTreeMap<&str, usize> = BTreeMap::new();
    for e in g.edges.iter().filter(|e| e.relation == EdgeRelation::Calls) {
        *hubs.entry(e.target.as_str()).or_default() += 1;
    }
    let mut hubs: Vec<_> = hubs.into_iter().collect();
    hubs.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    println!("-- top call targets");
    for (id, c) in hubs.iter().take(12) {
        println!("  {c:3} <- {}", name_of(id));
    }
    println!("-- artifacts");
    for a in g.nodes.iter().filter(|n| n.kind == NodeKind::Artifact) {
        let who = |rel: EdgeRelation| -> Vec<String> {
            g.edges
                .iter()
                .filter(|e| e.relation == rel && e.target == a.id)
                .map(|e| name_of(&e.source))
                .collect()
        };
        println!(
            "  {}: writers {:?} readers {:?}",
            a.name,
            who(EdgeRelation::Writes),
            who(EdgeRelation::Reads)
        );
    }
    if std::env::var("SAMPLE").is_ok() {
        println!("-- sample calls");
        let calls: Vec<_> = g
            .edges
            .iter()
            .filter(|e| e.relation == EdgeRelation::Calls)
            .collect();
        for e in calls.iter().step_by((calls.len() / 40).max(1)) {
            println!(
                "  {} -> {} ({})",
                name_of(&e.source),
                name_of(&e.target),
                e.confidence
            );
        }
    }
    let tests = g.nodes.iter().filter(|n| n.kind == NodeKind::Test).count();
    let loc: usize = g
        .facts
        .values()
        .flatten()
        .filter_map(|f| f.strip_prefix("loc:")?.parse::<usize>().ok())
        .sum();
    println!("test nodes {tests}, code lines {loc}");
}
