//! Picks the modules worth drawing and lays regions and components out on a canvas.

use crate::labels::{counts_label, modules_label, relation_label, role_for};
use crate::modules::ModuleSummary;
use archify_ir::{Component, Connection, Region};
use graft_model::EdgeRelation;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};

const MAX_COMPONENTS: usize = 30;
const MAX_CONNECTIONS: usize = 45;
const CELL_W: f32 = 150.0;
const CELL_H: f32 = 56.0;
const GAP_X: f32 = 24.0;
const GAP_Y: f32 = 26.0;
const PAD: f32 = 18.0;
const HEADER: f32 = 34.0;
const LEFT: f32 = 40.0;
const TOP: f32 = 96.0;
const REGION_GAP: f32 = 36.0;
const MAX_ROW_W: f32 = 1700.0;
/// Edges lighter than this carry no label (keeps the picture quiet).
const LABEL_MIN: u32 = 3;
const STRONG_MIN: u32 = 10;

pub struct Layout {
    pub regions: Vec<Region>,
    pub components: Vec<Component>,
    pub connections: Vec<Connection>,
    pub shown: usize,
    pub total: usize,
}

fn region_of<'a>(summary: &'a ModuleSummary, key: &str) -> &'a str {
    summary.modules[key].region.as_str()
}

pub fn layout(summary: &ModuleSummary, locale: &str) -> Layout {
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
    let keep: HashSet<&str> = ranked.iter().map(|&(k, _)| k).collect();

    let mut groups: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for &(k, _) in &ranked {
        groups.entry(region_of(summary, k)).or_default().push(k);
    }

    // Regions that call others go left, the ones being called go right.
    let mut flow: HashMap<&str, i64> = HashMap::new();
    for d in &summary.dependencies {
        if keep.contains(d.from.as_str()) && keep.contains(d.to.as_str()) {
            let (a, b) = (region_of(summary, &d.from), region_of(summary, &d.to));
            if a != b {
                *flow.entry(a).or_default() += i64::from(d.count);
                *flow.entry(b).or_default() -= i64::from(d.count);
            }
        }
    }
    let mut order: Vec<&str> = groups.keys().copied().collect();
    order.sort_by(|a, b| {
        let (fa, fb) = (flow.get(a).unwrap_or(&0), flow.get(b).unwrap_or(&0));
        fb.cmp(fa).then(a.cmp(b))
    });

    // A module that talks to regions on its right belongs in its region's right column
    // (and vice versa), so cross-region links stay short instead of crossing boxes.
    let pos: HashMap<&str, usize> = order.iter().enumerate().map(|(i, &r)| (r, i)).collect();
    let pull = |key: &str, own: usize| -> i64 {
        (summary.dependencies.iter())
            .filter(|d| keep.contains(d.from.as_str()) && keep.contains(d.to.as_str()))
            .filter_map(|d| {
                let other = if d.from == key {
                    &d.to
                } else if d.to == key {
                    &d.from
                } else {
                    return None;
                };
                Some(match pos[region_of(summary, other)].cmp(&own) {
                    Ordering::Greater => i64::from(d.count),
                    Ordering::Less => -i64::from(d.count),
                    Ordering::Equal => 0,
                })
            })
            .sum()
    };

    let mut regions = Vec::new();
    let mut components = Vec::new();
    let (mut x, mut y, mut row_h) = (LEFT, TOP, 0.0f32);
    for (ri, &name) in order.iter().enumerate() {
        let mods = &groups[name];
        let n = mods.len();
        let cols = ((n as f32).sqrt().ceil() as usize).clamp(1, 3);
        let rows = n.div_ceil(cols);
        let w = 2.0 * PAD + cols as f32 * CELL_W + (cols as f32 - 1.0) * GAP_X;
        let h = HEADER + PAD + rows as f32 * CELL_H + (rows as f32 - 1.0) * GAP_Y + PAD;
        if x > LEFT && x + w > LEFT + MAX_ROW_W {
            x = LEFT;
            y += row_h + REGION_GAP;
            row_h = 0.0;
        }
        let total_in_region = summary
            .modules
            .values()
            .filter(|m| m.region == name)
            .count();
        regions.push(Region {
            id: name.to_string(),
            label: name.to_string(),
            sublabel: Some(modules_label(n, total_in_region, locale)),
            x,
            y,
            width: w,
            height: h,
        });
        // Column-major by pull (stable: equal pull keeps the degree ranking).
        let mut by_pull: Vec<(&str, i64)> = mods.iter().map(|&k| (k, pull(k, ri))).collect();
        by_pull.sort_by_key(|&(_, p)| p);
        for (i, &(key, _)) in by_pull.iter().enumerate() {
            let m = &summary.modules[key];
            components.push(Component {
                id: key.to_string(),
                label: m.name.clone(),
                sublabel: Some(counts_label(m.files, m.symbols, locale)),
                role: role_for(&m.name, &m.region),
                x: x + PAD + (i / rows) as f32 * (CELL_W + GAP_X),
                y: y + HEADER + (i % rows) as f32 * (CELL_H + GAP_Y),
                width: CELL_W,
                height: CELL_H,
            });
        }
        x += w + REGION_GAP;
        row_h = row_h.max(h);
    }

    if locale == "ar" {
        let right = regions.iter().map(|r| r.x + r.width).fold(LEFT, f32::max);
        for r in &mut regions {
            r.x = LEFT + right - r.x - r.width;
        }
        for c in &mut components {
            c.x = LEFT + right - c.x - c.width;
        }
    }

    let connections = summary
        .dependencies
        .iter()
        .filter(|d| keep.contains(d.from.as_str()) && keep.contains(d.to.as_str()))
        .take(MAX_CONNECTIONS)
        .map(|d| Connection {
            from: d.from.clone(),
            to: d.to.clone(),
            label: (d.count >= LABEL_MIN)
                .then(|| format!("{} ×{}", relation_label(&d.dominant, locale), d.count)),
            line_style: if d.count >= STRONG_MIN {
                "strong"
            } else if d.dominant == EdgeRelation::References {
                "dashed"
            } else {
                "default"
            }
            .to_string(),
        })
        .collect();

    Layout {
        regions,
        components,
        connections,
        shown: keep.len(),
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
        assert_eq!((l.shown, l.total, l.components.len()), (30, 60, 30));
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
        assert_eq!(ltr.regions[0].id, "b");
        assert!(ltr.regions[0].x < ltr.regions[1].x);
        let rtl = layout(&summary(4, 4), "ar");
        let (b, a) = (
            rtl.regions.iter().find(|r| r.id == "b").unwrap(),
            rtl.regions.iter().find(|r| r.id == "a").unwrap(),
        );
        assert!(b.x > a.x, "in RTL the caller sits on the right");
        assert!(rtl.regions.iter().all(|r| r.x >= LEFT));
    }

    #[test]
    fn modules_that_talk_across_regions_sit_on_the_facing_edge() {
        let l = layout(&summary(4, 4), "en");
        let comp = |id: &str| l.components.iter().find(|c| c.id == id).unwrap();
        let (b, a) = (&l.regions[0], &l.regions[1]);
        assert_eq!((b.id.as_str(), a.id.as_str()), ("b", "a"));
        // b (left) calls a (right): its heaviest link points right, so m0 takes the right column.
        assert_eq!(comp("b/m0").x, b.x + b.width - PAD - CELL_W);
        // ...and the callee takes the left column of its region.
        assert_eq!(comp("a/m0").x, a.x + PAD);
    }

    #[test]
    fn connections_are_labelled_and_styled_by_weight() {
        let l = layout(&summary(4, 4), "en");
        let by = |from: &str| l.connections.iter().find(|c| c.from == from).unwrap();
        assert_eq!(by("b/m0").label.as_deref(), Some("calls ×12"));
        assert_eq!(by("b/m0").line_style, "strong");
        assert_eq!(by("b/m1").label.as_deref(), Some("imports ×3"));
        assert_eq!(by("b/m2").label, None);
        assert_eq!(by("b/m2").line_style, "dashed");
    }

    #[test]
    fn connections_to_cut_modules_are_dropped() {
        let mut s = summary(40, 2);
        s.dependencies.push(crate::modules::Dependency {
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
        let ids = |l: &Layout| {
            l.components
                .iter()
                .map(|c| (c.id.clone(), c.x as i32, c.y as i32))
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(&a), ids(&b));
    }
}
