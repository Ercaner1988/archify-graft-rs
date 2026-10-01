//! `EXT=js SAMPLE=30 cargo run -p graft-parser --example langs_check -- <dir>`: Calls edges
//! of the files with one extension: count, top targets, and sampled edges with the source
//! line that holds the call, for checking them by eye.

use graft_model::{EdgeRelation, NodeKind};
use graft_parser::CodeExtractor;
use std::collections::{BTreeMap, HashMap};

fn main() {
    let root = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let ext = std::env::var("EXT").unwrap_or_default();
    let sample: usize = std::env::var("SAMPLE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let t = std::time::Instant::now();
    let g = CodeExtractor::index_directory(&root).expect("index");
    let by_id: HashMap<&str, &graft_model::NodeV1> =
        g.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let in_ext = |p: &str| ext.is_empty() || p.ends_with(&format!(".{ext}"));
    let files = g
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::File && in_ext(&n.path))
        .count();
    let fns = g
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Function | NodeKind::Method) && in_ext(&n.path))
        .count();
    let calls: Vec<_> = g
        .edges
        .iter()
        .filter(|e| {
            e.relation == EdgeRelation::Calls
                && by_id
                    .get(e.source.as_str())
                    .is_some_and(|n| in_ext(&n.path))
        })
        .collect();
    let imports = g
        .edges
        .iter()
        .filter(|e| {
            e.relation == EdgeRelation::Imports
                && by_id
                    .get(e.source.as_str())
                    .is_some_and(|n| in_ext(&n.path))
        })
        .count();
    let with_facts = g
        .nodes
        .iter()
        .filter(|n| {
            in_ext(&n.path)
                && g.facts
                    .get(&n.id)
                    .is_some_and(|f| f.iter().any(|x| x.starts_with("c:")))
        })
        .count();
    println!("indexed in {:?}: {files} files, {fns} fns ({with_facts} with call sites), {} Calls, {imports} Imports", t.elapsed(), calls.len());
    let mut hubs: BTreeMap<&str, usize> = BTreeMap::new();
    for e in &calls {
        *hubs.entry(e.target.as_str()).or_default() += 1;
    }
    let mut hubs: Vec<_> = hubs.into_iter().collect();
    hubs.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    for (id, c) in hubs.iter().take(10) {
        let n = by_id[id];
        println!(
            "  hub {c:3} <- {} ({})",
            n.name,
            n.path
                .rsplit(|c| c == '/' || c == char::from(92u8))
                .next()
                .unwrap_or("")
        );
    }
    #[allow(clippy::manual_checked_ops)]
    if sample > 0 {
        for e in calls
            .iter()
            .step_by((calls.len() / sample).max(1))
            .take(sample)
        {
            let (s, d) = (by_id[e.source.as_str()], by_id[e.target.as_str()]);
            let line = std::fs::read_to_string(&s.path).ok().and_then(|txt| {
                let sp = s.span.as_ref()?;
                txt.lines()
                    .skip(sp.start_line.saturating_sub(1))
                    .take(sp.end_line.saturating_sub(sp.start_line) + 2)
                    .find(|l| {
                        l.contains(&format!("{}(", d.name))
                            || l.contains(&format!("new {}", d.name))
                            || l.contains(&format!(".{}", d.name))
                    })
                    .map(|l| l.trim().chars().take(90).collect::<String>())
            });
            let f = |p: &str| {
                p.rsplit(|c| c == '/' || c == char::from(92u8))
                    .next()
                    .unwrap_or("")
                    .to_string()
            };
            println!(
                "  {}:{} -> {}:{} ({:.2}) | {}",
                f(&s.path),
                s.name,
                f(&d.path),
                d.name,
                e.confidence,
                line.unwrap_or_default()
            );
        }
    }
}
