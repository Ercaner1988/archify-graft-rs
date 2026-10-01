//! Level 2 (and the fallback for trees without `Cargo.toml`): modules as boxes,
//! aggregated dependencies as arrows, laid out in layers like the crate level.

use crate::labels::{counts_label, modules_label, relation_label, role_for};
use crate::layered::{self, Edge, NODE_H, NODE_W};
use crate::modules::ModuleSummary;
use archify_ir::{Component, Connection, Region};
use graft_model::EdgeRelation;
use std::collections::{BTreeMap, HashMap, HashSet};

pub const MAX_COMPONENTS: usize = 36;
const MAX_CONNECTIONS: usize = 60;
/// Edges lighter than this carry no label (keeps the picture quiet).
const LABEL_MIN: u32 = 2;
const STRONG_MIN: u32 = 10;

pub struct Laid {
    pub regions: Vec<Region>,
    pub components: Vec<Component>,
    pub connections: Vec<Connection>,
    pub shown: usize,
    pub total: usize,
}

pub fn layout(summary: &ModuleSummary, locale: &str) -> Laid {
    // Rank by how connected a module is; the busiest ones survive the cut.
    let mut degree: HashMap<&str, u32> = HashMap::new();
    for d in &summary.dependencies {
        *degree.entry(d.from.as_str()).or_default() += d.count;
        *degree.entry(d.to.as_str()).or_default() += d.count;
    }
    let mut ranked: Vec<(&str, u32)> = summary
        .modules
        .iter()
        .map(|(k, m)| {
            let deg = degree.get(k.as_str()).copied().unwrap_or(0);
            (k.as_str(), deg * 2 + m.files as u32 + m.symbols as u32 / 4)
        })
        .collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    ranked.truncate(MAX_COMPONENTS);
    // Stable order for the placement: by key.
    let mut kept: Vec<&str> = ranked.iter().map(|&(k, _)| k).collect();
    kept.sort_unstable();
    let index: HashMap<&str, usize> = kept.iter().enumerate().map(|(i, &k)| (k, i)).collect();

    let mut group_names: BTreeMap<&str, usize> = BTreeMap::new();
    for &k in &kept {
        let next = group_names.len();
        group_names
            .entry(summary.modules[k].region.as_str())
            .or_insert(next);
    }
    let group: Vec<usize> = kept
        .iter()
        .map(|k| group_names[summary.modules[*k].region.as_str()])
        .collect();
    let kept_deps: Vec<_> = summary
        .dependencies
        .iter()
        .filter(|d| index.contains_key(d.from.as_str()) && index.contains_key(d.to.as_str()))
        .take(MAX_CONNECTIONS)
        .collect();
    let edges: Vec<Edge> = kept_deps
        .iter()
        .map(|d| Edge {
            from: index[d.from.as_str()],
            to: index[d.to.as_str()],
            weight: d.count,
        })
        .collect();
    let placed = layered::place(&group, &edges, group_names.len());

    let right = placed
        .groups
        .iter()
        .map(|r| r.x + r.w)
        .fold(0.0f32, f32::max);
    let rtl = locale == "ar";
    let fx = |x: f32, w: f32| if rtl { layered::mirror(x, w, right) } else { x };

    let mut regions = Vec::new();
    for (name, &g) in &group_names {
        let r = placed.groups[g];
        let shown = group.iter().filter(|&&x| x == g).count();
        let total = summary
            .modules
            .values()
            .filter(|m| m.region == *name)
            .count();
        regions.push(Region {
            id: (*name).to_string(),
            label: (*name).to_string(),
            sublabel: Some(modules_label(shown, total, locale)),
            x: fx(r.x, r.w),
            y: r.y,
            width: r.w,
            height: r.h,
        });
    }
    let components = kept
        .iter()
        .enumerate()
        .map(|(i, &key)| {
            let m = &summary.modules[key];
            Component {
                id: key.to_string(),
                label: m.name.clone(),
                sublabel: Some(counts_label(m.files, m.symbols, locale)),
                role: role_for(&m.name, &m.region),
                x: fx(placed.pos[i].0, NODE_W),
                y: placed.pos[i].1,
                width: NODE_W,
                height: NODE_H,
            }
        })
        .collect();
    let connections = kept_deps
        .iter()
        .map(|d| Connection {
            from: d.from.clone(),
            to: d.to.clone(),
            label: (d.count >= LABEL_MIN)
                .then(|| format!("{} ×{}", relation_label(&d.dominant, locale), d.count)),
            line_style: if d.count >= STRONG_MIN {
                "animated"
            } else if d.dominant == EdgeRelation::References {
                "dashed"
            } else {
                ""
            }
            .to_string(),
        })
        .collect();
    let shown: HashSet<&str> = kept.iter().copied().collect();
    Laid {
        regions,
        components,
        connections,
        shown: shown.len(),
        total: summary.modules.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::{Dependency, ModuleInfo};

    fn summary(n_a: usize, n_b: usize) -> ModuleSummary {
        let mut modules = BTreeMap::new();
        for (region, n) in [("a", n_a), ("b", n_b)] {
            for i in 0..n {
                modules.insert(
                    format!("{region}/m{i}"),
                    ModuleInfo {
                        region: region.into(),
                        name: format!("m{i}"),
                        files: 1,
                        symbols: i,
                    },
                );
            }
        }
        let dep = |from: &str, to: &str, count, dominant| Dependency {
            from: from.into(),
            to: to.into(),
            count,
            dominant,
        };
        ModuleSummary {
            modules,
            dependencies: vec![
                dep("b/m0", "a/m0", 12, EdgeRelation::Calls),
                dep("b/m1", "a/m1", 3, EdgeRelation::Imports),
                dep("b/m2", "a/m2", 1, EdgeRelation::References),
            ],
        }
    }

    fn inside(c: &Component, r: &Region) -> bool {
        c.x >= r.x
            && c.y >= r.y
            && c.x + c.width <= r.x + r.width
            && c.y + c.height <= r.y + r.height
    }

    #[test]
    fn caps_the_component_count_and_reports_how_many_were_cut() {
        let l = layout(&summary(30, 30), "en");
        assert_eq!(
            (l.shown, l.total, l.components.len()),
            (MAX_COMPONENTS, 60, MAX_COMPONENTS)
        );
    }

    #[test]
    fn every_component_sits_inside_its_region_and_regions_do_not_overlap() {
        let l = layout(&summary(9, 5), "en");
        for c in &l.components {
            let region = l
                .regions
                .iter()
                .find(|r| c.id.starts_with(&format!("{}/", r.id)));
            assert!(
                inside(c, region.expect("component has a region")),
                "{}",
                c.id
            );
        }
        let (a, b) = (&l.regions[0], &l.regions[1]);
        assert!(
            a.x + a.width <= b.x
                || b.x + b.width <= a.x
                || a.y + a.height <= b.y
                || b.y + b.height <= a.y
        );
    }

    #[test]
    fn callers_go_left_and_rtl_mirrors_that() {
        let ltr = layout(&summary(4, 4), "en");
        let x = |l: &Laid, id: &str| l.regions.iter().find(|r| r.id == id).unwrap().x;
        assert!(
            x(&ltr, "b") < x(&ltr, "a"),
            "b calls a, so b is on the left"
        );
        let rtl = layout(&summary(4, 4), "ar");
        assert!(
            x(&rtl, "b") > x(&rtl, "a"),
            "in RTL the caller sits on the right"
        );
    }

    #[test]
    fn connections_are_labelled_and_styled_by_weight() {
        let l = layout(&summary(4, 4), "en");
        let by = |from: &str| l.connections.iter().find(|c| c.from == from).unwrap();
        assert_eq!(by("b/m0").label.as_deref(), Some("calls ×12"));
        assert_eq!(by("b/m0").line_style, "animated");
        assert_eq!(by("b/m1").label.as_deref(), Some("imports ×3"));
        assert_eq!(by("b/m2").label, None);
        assert_eq!(by("b/m2").line_style, "dashed");
    }

    #[test]
    fn connections_to_cut_modules_are_dropped() {
        let mut s = summary(40, 2);
        s.dependencies.push(Dependency {
            from: "b/m0".into(),
            to: "a/m39".into(),
            count: 1,
            dominant: EdgeRelation::Calls,
        });
        let l = layout(&s, "en");
        let ids: HashSet<&str> = l.components.iter().map(|c| c.id.as_str()).collect();
        assert!(l
            .connections
            .iter()
            .all(|c| ids.contains(c.from.as_str()) && ids.contains(c.to.as_str())));
    }

    #[test]
    fn layout_is_deterministic() {
        let (a, b) = (layout(&summary(12, 7), "tr"), layout(&summary(12, 7), "tr"));
        let ids = |l: &Laid| {
            l.components
                .iter()
                .map(|c| (c.id.clone(), c.x as i32, c.y as i32))
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(&a), ids(&b));
    }
}
