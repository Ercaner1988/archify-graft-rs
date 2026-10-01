//! Level 1: the crate diagram. Boxes are crates, arrows are Cargo dependencies weighted
//! by how many `use` paths back them, regions are the repositories / workspaces.

use crate::crates::{summarize, CrateBox, CrateEdge, CrateSummary};
use crate::labels::{crate_sublabel, crates_subtitle, group_sublabel, uses_label};
use crate::layered::{self, Edge, NODE_H, NODE_W};
use crate::tour;
use archify_ir::{ArchitectureDiagram, Component, Connection, DiagramMeta, Region, VisualPreset};
use graft_model::CodeGraph;
use std::collections::{BTreeMap, HashMap};

const MAX_CRATES: usize = 48;
/// Arrows with at least this many `use` paths flow (animated light beam).
const STRONG_USES: u32 = 20;

/// Keeps the `MAX_CRATES` best connected boxes (and the edges between them).
fn trim(s: CrateSummary) -> (CrateSummary, usize) {
    let total = s.boxes.len();
    if total <= MAX_CRATES {
        return (s, total);
    }
    let mut score: Vec<(usize, u32)> = (0..total)
        .map(|i| {
            let deg: u32 = s
                .edges
                .iter()
                .filter(|e| e.from == i || e.to == i)
                .map(|e| e.uses + 1)
                .sum();
            (i, deg * 2 + (s.boxes[i].lines / 400) as u32)
        })
        .collect();
    score.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    score.truncate(MAX_CRATES);
    let mut keep: Vec<usize> = score.into_iter().map(|(i, _)| i).collect();
    keep.sort_unstable();
    let new_ix: HashMap<usize, usize> = keep.iter().enumerate().map(|(n, &o)| (o, n)).collect();
    let mut boxes = Vec::new();
    let mut old: Vec<Option<CrateBox>> = s.boxes.into_iter().map(Some).collect();
    for &i in &keep {
        if let Some(b) = old[i].take() {
            boxes.push(b);
        }
    }
    let edges = s
        .edges
        .into_iter()
        .filter_map(|e| {
            Some(CrateEdge {
                from: *new_ix.get(&e.from)?,
                to: *new_ix.get(&e.to)?,
                ..e
            })
        })
        .collect();
    (CrateSummary { boxes, edges }, total)
}

/// `None` when the graph has no crates.
pub fn diagram(graph: &CodeGraph, title: &str, locale: &str) -> Option<ArchitectureDiagram> {
    let (s, total) = trim(summarize(graph)?);
    let mut group_ix: BTreeMap<&str, usize> = BTreeMap::new();
    // Locals first so a workspace gets the lower index; externals follow by name.
    for b in s.boxes.iter().filter(|b| !b.external) {
        let next = group_ix.len();
        group_ix.entry(b.group.as_str()).or_insert(next);
    }
    for b in s.boxes.iter().filter(|b| b.external) {
        let next = group_ix.len();
        group_ix.entry(b.group.as_str()).or_insert(next);
    }
    let group: Vec<usize> = s.boxes.iter().map(|b| group_ix[b.group.as_str()]).collect();
    let edges: Vec<Edge> = s
        .edges
        .iter()
        .map(|e| Edge {
            from: e.from,
            to: e.to,
            weight: e.uses + 1,
        })
        .collect();
    let placed = layered::place(&group, &edges, group_ix.len());
    let right = placed
        .groups
        .iter()
        .map(|r| r.x + r.w)
        .fold(0.0f32, f32::max);
    let rtl = locale == "ar";
    let fx = |x: f32, w: f32| if rtl { layered::mirror(x, w, right) } else { x };

    let mut ordered: Vec<(&&str, &usize)> = group_ix.iter().collect();
    ordered.sort_by_key(|(_, g)| **g);
    let regions: Vec<Region> = ordered
        .into_iter()
        .map(|(name, &g)| {
            let members: Vec<&CrateBox> = s.boxes.iter().filter(|b| b.group == *name).collect();
            let r = placed.groups[g];
            Region {
                id: (*name).to_string(),
                label: (*name).to_string(),
                sublabel: Some(group_sublabel(
                    members.len(),
                    members.iter().all(|b| b.external),
                    locale,
                )),
                x: fx(r.x, r.w),
                y: r.y,
                width: r.w,
                height: r.h,
            }
        })
        .collect();
    let components: Vec<Component> = s
        .boxes
        .iter()
        .enumerate()
        .map(|(i, b)| Component {
            id: b.id.clone(),
            label: b.label.clone(),
            sublabel: Some(crate_sublabel(b.lib, b.bin, b.external, b.lines, locale)),
            role: b.role,
            x: fx(placed.pos[i].0, NODE_W),
            y: placed.pos[i].1,
            width: NODE_W,
            height: NODE_H,
        })
        .collect();
    let connections: Vec<Connection> = s
        .edges
        .iter()
        .map(|e| Connection {
            from: s.boxes[e.from].id.clone(),
            to: s.boxes[e.to].id.clone(),
            label: Some(uses_label(e.uses, locale)),
            line_style: if e.optional {
                "dashed"
            } else if s.boxes[e.to].external {
                "dotted"
            } else if e.uses >= STRONG_USES {
                "animated"
            } else {
                ""
            }
            .to_string(),
        })
        .collect();
    let story_beats = tour::crate_tour(&s, locale);
    Some(ArchitectureDiagram {
        meta: DiagramMeta {
            title: title.to_string(),
            subtitle: Some(crates_subtitle(
                s.boxes.len(),
                total,
                connections.len(),
                locale,
            )),
            locale: locale.to_string(),
            visual_preset: VisualPreset::Editorial,
        },
        components,
        connections,
        regions,
        story_beats,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_cargo::Manifest;
    use graft_model::{NodeKind, NodeV1};

    fn add(g: &mut CodeGraph, path: &str, name: &str, kind: NodeKind) {
        g.add_node(NodeV1 {
            id: format!("file:{path}"),
            path: path.into(),
            name: name.into(),
            kind,
            span: None,
            search_body: String::new(),
            file_residual: String::new(),
        });
    }

    /// cli -> core -> store, cli -> core (50 uses), core -> katla (sibling repo).
    fn graph() -> CodeGraph {
        let mut g = CodeGraph::new();
        let crates = [
            ("/w/r/cli", "cli", "core={path=\"../core\"}\nclap=\"4\"\n"),
            (
                "/w/r/core",
                "core",
                "store={path=\"../store\"}\nkatla={path=\"../../katla\"}\n",
            ),
            ("/w/r/store", "store", ""),
        ];
        add(&mut g, "/w/r/Cargo.toml", "Cargo.toml", NodeKind::File);
        g.imports.insert(
            "file:/w/r/Cargo.toml".into(),
            Manifest::parse("[workspace]\n").unwrap().to_specs(),
        );
        for (dir, name, deps) in crates {
            let p = format!("{dir}/Cargo.toml");
            add(&mut g, &p, "Cargo.toml", NodeKind::File);
            let toml = format!("[package]\nname=\"{name}\"\n[dependencies]\n{deps}");
            g.imports.insert(
                format!("file:{p}"),
                Manifest::parse(&toml).unwrap().to_specs(),
            );
            let src = if name == "cli" { "main.rs" } else { "lib.rs" };
            add(&mut g, &format!("{dir}/src/{src}"), src, NodeKind::File);
        }
        let specs: Vec<String> = (0..50).map(|i| format!("rs:core::f{i}")).collect();
        g.imports.insert("file:/w/r/cli/src/main.rs".into(), specs);
        g
    }

    #[test]
    fn crates_become_boxes_in_repo_regions_with_weighted_labelled_arrows() {
        let d = diagram(&graph(), "t", "tr").unwrap();
        let ids: Vec<&str> = d.components.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["cli", "core", "store", "ext:katla/katla"]);
        let regions: Vec<&str> = d.regions.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(regions, ["r", "katla"]);
        let c = d
            .connections
            .iter()
            .find(|c| c.from == "cli" && c.to == "core")
            .unwrap();
        assert_eq!(c.label.as_deref(), Some("kullanır ×50"));
        assert_eq!(c.line_style, "animated");
        let k = d
            .connections
            .iter()
            .find(|c| c.to == "ext:katla/katla")
            .unwrap();
        assert_eq!(k.line_style, "dotted");
        assert_eq!(k.label.as_deref(), Some("bağımlı"));
        assert_eq!(d.meta.visual_preset, VisualPreset::Editorial);
    }

    #[test]
    fn users_are_left_of_what_they_use_and_nothing_overlaps() {
        let d = diagram(&graph(), "t", "en").unwrap();
        let x = |id: &str| d.components.iter().find(|c| c.id == id).unwrap().x;
        assert!(x("cli") < x("core") && x("core") < x("store"));
        assert!(x("core") < x("ext:katla/katla"));
        for a in &d.components {
            for b in &d.components {
                if a.id != b.id {
                    assert!((a.x - b.x).abs() >= a.width || (a.y - b.y).abs() >= a.height);
                }
            }
        }
    }

    #[test]
    fn a_tree_without_manifests_has_no_crate_diagram() {
        assert!(diagram(&CodeGraph::new(), "t", "en").is_none());
    }
}
