//! Scene construction: text fitting, region plates, routing and content bounds.

use crate::layered::{layered, LayerOptions};
use crate::{legend_of, tour, Line, RouteStats, Scene, SceneEdge, SceneNode, SceneRegion};
use archify_ir::{
    ArchitectureDiagram, DataflowDiagram, DeltaDiagram, DiffStatus, Region, SemanticRole,
    StoryBeat, VisualPreset,
};
use archify_route::{route_all, EdgeRequest, Options, Rect};
use archify_style::{font, text, EdgeKind};

struct NodeSpec {
    id: String,
    label: String,
    sublabel: Option<String>,
    tag: Option<String>,
    role: SemanticRole,
    rect: Rect,
    status: Option<DiffStatus>,
}

struct EdgeSpec {
    from: String,
    to: String,
    label: Option<String>,
    line_style: String,
    status: Option<DiffStatus>,
}

struct Meta<'a> {
    title: &'a str,
    subtitle: Option<&'a str>,
    locale: &'a str,
    preset: VisualPreset,
}

pub(crate) fn architecture_scene(d: &ArchitectureDiagram) -> Scene {
    let nodes = d
        .components
        .iter()
        .map(|c| NodeSpec {
            id: c.id.clone(),
            label: c.label.clone(),
            sublabel: c.sublabel.clone(),
            tag: None,
            role: c.role,
            rect: Rect::new(c.x, c.y, c.width.max(40.0), c.height.max(28.0)),
            status: None,
        })
        .collect();
    let edges = d
        .connections
        .iter()
        .map(|c| EdgeSpec {
            from: c.from.clone(),
            to: c.to.clone(),
            label: c.label.clone(),
            line_style: c.line_style.clone(),
            status: None,
        })
        .collect();
    assemble(
        Meta {
            title: &d.meta.title,
            subtitle: d.meta.subtitle.as_deref(),
            locale: &d.meta.locale,
            preset: d.meta.visual_preset,
        },
        nodes,
        edges,
        region_scenes(&d.regions),
        &d.story_beats,
    )
}

pub fn dataflow_scene(d: &DataflowDiagram) -> Scene {
    let sizes: Vec<[f32; 2]> = d
        .nodes
        .iter()
        .map(|n| {
            let w = (text::units(&n.label) * font::NODE_LABEL * font::MONO_ADVANCE + 28.0)
                .clamp(140.0, 230.0);
            [w, if n.stream_rate.is_some() { 64.0 } else { 48.0 }]
        })
        .collect();
    let index = |id: &str| d.nodes.iter().position(|n| n.id == id);
    let pairs: Vec<(usize, usize)> = d
        .pipelines
        .iter()
        .filter_map(|p| Some((index(&p.from)?, index(&p.to)?)))
        .collect();
    let rects = layered(&sizes, &pairs, &LayerOptions::default());
    let nodes = d
        .nodes
        .iter()
        .zip(rects)
        .map(|(n, rect)| NodeSpec {
            id: n.id.clone(),
            label: n.label.clone(),
            sublabel: n.stream_rate.clone(),
            tag: None,
            role: n.role,
            rect,
            status: None,
        })
        .collect();
    let edges = d
        .pipelines
        .iter()
        .map(|p| EdgeSpec {
            from: p.from.clone(),
            to: p.to.clone(),
            label: match (&p.schema, &p.throughput) {
                (Some(s), Some(t)) => Some(format!("{s} · {t}")),
                (Some(s), None) => Some(s.clone()),
                (None, t) => t.clone(),
            },
            line_style: String::new(),
            status: None,
        })
        .collect();
    assemble(
        Meta {
            title: &d.meta.title,
            subtitle: d.meta.subtitle.as_deref(),
            locale: &d.meta.locale,
            preset: d.meta.visual_preset,
        },
        nodes,
        edges,
        Vec::new(),
        &[],
    )
}

pub fn delta_scene(d: &DeltaDiagram) -> Scene {
    let nodes = d
        .components
        .iter()
        .map(|c| NodeSpec {
            id: c.component.id.clone(),
            label: c.component.label.clone(),
            sublabel: c.component.sublabel.clone(),
            tag: None,
            role: c.component.role,
            rect: Rect::new(
                c.component.x,
                c.component.y,
                c.component.width.max(40.0),
                c.component.height.max(28.0),
            ),
            status: Some(c.status),
        })
        .collect();
    let edges = d
        .connections
        .iter()
        .map(|c| EdgeSpec {
            from: c.connection.from.clone(),
            to: c.connection.to.clone(),
            label: c.connection.label.clone(),
            line_style: c.connection.line_style.clone(),
            status: Some(c.status),
        })
        .collect();
    assemble(
        Meta {
            title: &d.meta.title,
            subtitle: d.meta.subtitle.as_deref(),
            locale: &d.meta.locale,
            preset: d.meta.visual_preset,
        },
        nodes,
        edges,
        Vec::new(),
        &[],
    )
}

fn line(t: &str, avail: f32, font_px: f32, min: f32, y: f32) -> Line {
    let f = text::fit(t, avail, font_px, min);
    Line {
        text: f.text,
        font: f.font,
        y,
        truncated: f.truncated,
    }
}

/// Text block of a box: proportions follow the 64px reference (label 30, sublabel 46, tag 56).
fn node_text(spec: &NodeSpec) -> (Line, Option<Line>, Option<Line>) {
    let (w, h) = (spec.rect.w, spec.rect.h);
    let avail = (w - 16.0).max(20.0);
    let sub = spec.sublabel.as_deref().filter(|s| !s.is_empty());
    let tag = spec.tag.as_deref().filter(|s| !s.is_empty());
    let (ly, sy, ty) = match (sub.is_some(), tag.is_some()) {
        (false, false) => (h * 0.5, 0.0, 0.0),
        (true, false) => (h * 0.42, h * 0.70, 0.0),
        (false, true) => (h * 0.42, 0.0, h * 0.78),
        (true, true) => (h * 0.47, h * 0.72, h * 0.875),
    };
    (
        line(&spec.label, avail, font::NODE_LABEL, 8.0, ly),
        sub.map(|s| line(s, avail, font::NODE_SUBLABEL, 7.0, sy)),
        tag.map(|s| line(s, avail, font::NODE_TAG, 6.0, ty)),
    )
}

fn region_scenes(regions: &[Region]) -> Vec<SceneRegion> {
    regions
        .iter()
        .map(|r| {
            let rect = Rect::new(r.x, r.y, r.width, r.height);
            let room = (rect.w - 28.0).max(30.0);
            let title = line(&r.label, room, font::REGION_TITLE, 7.0, 15.0);
            let tw = text::width(&title.text, title.font) + 12.0;
            let title_plate = Rect::new(rect.x + 10.0, rect.y + 7.0, tw, 16.0);
            let (sub, sub_plate) = match r.sublabel.as_deref().filter(|s| !s.is_empty()) {
                Some(s) => {
                    let left = room - tw - 16.0;
                    let l = line(s, left, 8.0, 7.0, 15.0);
                    let w = text::width(&l.text, l.font) + 12.0;
                    if left >= 36.0 {
                        let plate = Rect::new(rect.right() - 10.0 - w, rect.y + 7.0, w, 16.0);
                        (Some(l), Some(plate))
                    } else {
                        (None, None)
                    }
                }
                None => (None, None),
            };
            SceneRegion {
                id: r.id.clone(),
                rect,
                title,
                title_plate,
                sub,
                sub_plate,
            }
        })
        .collect()
}

fn assemble(
    meta: Meta<'_>,
    specs: Vec<NodeSpec>,
    edge_specs: Vec<EdgeSpec>,
    regions: Vec<SceneRegion>,
    story: &[StoryBeat],
) -> Scene {
    let nodes: Vec<SceneNode> = specs
        .iter()
        .map(|s| {
            let (label, sublabel, tag) = node_text(s);
            let context = regions
                .iter()
                .filter(|r| r.rect.contains_strict([s.rect.cx(), s.rect.cy()]))
                .min_by(|a, b| (a.rect.w * a.rect.h).total_cmp(&(b.rect.w * b.rect.h)))
                .map(|r| r.title.text.clone());
            let mut tip = s.label.clone();
            for extra in [s.sublabel.as_deref(), context.as_deref(), s.tag.as_deref()]
                .into_iter()
                .flatten()
            {
                tip.push_str(" · ");
                tip.push_str(extra);
            }
            SceneNode {
                id: s.id.clone(),
                rect: s.rect,
                role: s.role,
                label,
                sublabel,
                tag,
                tooltip: tip,
                context,
                status: s.status,
            }
        })
        .collect();

    let index = |id: &str| nodes.iter().position(|n| n.id == id);
    let mut kept: Vec<(usize, usize, &EdgeSpec, Option<text::Fitted>)> = Vec::new();
    for e in &edge_specs {
        if let (Some(a), Some(b)) = (index(&e.from), index(&e.to)) {
            let fitted = e
                .label
                .as_deref()
                .filter(|l| !l.is_empty())
                .map(|l| text::fit(l, 150.0, font::EDGE_LABEL, 7.0));
            kept.push((a, b, e, fitted));
        }
    }
    let requests: Vec<EdgeRequest> = kept
        .iter()
        .map(|(a, b, _, f)| EdgeRequest {
            from: *a,
            to: *b,
            label_width: f.as_ref().map(|f| text::edge_label_width(&f.text)),
        })
        .collect();
    let boxes: Vec<Rect> = nodes.iter().map(|n| n.rect).collect();
    let keepouts: Vec<Rect> = regions
        .iter()
        .flat_map(|r| [Some(r.title_plate), r.sub_plate])
        .flatten()
        .collect();
    let routed = route_all(&boxes, &keepouts, &requests, &Options::default());

    let edges: Vec<SceneEdge> = kept
        .into_iter()
        .zip(routed)
        .map(|((from, to, spec, fitted), route)| SceneEdge {
            from,
            to,
            label: fitted.map(|f| f.text),
            full_label: spec.label.clone().filter(|l| !l.is_empty()),
            kind: EdgeKind::from_line_style(&spec.line_style),
            animated: EdgeKind::is_animated(&spec.line_style),
            route,
            status: spec.status,
        })
        .collect();

    let mut content: Option<Rect> = None;
    let mut grow = |r: Rect| {
        content = Some(content.map_or(r, |c| c.union(&r)));
    };
    for n in &nodes {
        grow(n.rect);
    }
    for r in &regions {
        grow(r.rect);
    }
    for e in &edges {
        for p in &e.route.points {
            grow(Rect::new(p[0], p[1], 0.0, 0.0));
        }
        if let Some(l) = e.route.label {
            grow(l);
        }
    }
    let beats = tour::build(story, &nodes, &edges, meta.locale);
    let stats = RouteStats {
        edges: edges.len(),
        blind: edges.iter().filter(|e| !e.route.clear).count(),
        through_box: edges
            .iter()
            .filter(|e| {
                nodes
                    .iter()
                    .any(|n| archify_route::poly::hits_rect(&e.route.points, &n.rect))
            })
            .count(),
    };
    Scene {
        title: meta.title.to_string(),
        subtitle: meta.subtitle.map(str::to_string),
        locale: meta.locale.to_string(),
        preset: meta.preset,
        legend: legend_of(&nodes),
        content: content.unwrap_or(Rect::new(0.0, 0.0, 400.0, 240.0)),
        nodes,
        regions,
        edges,
        beats,
        stats,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archify_ir::{ArchitectureDiagram, Component, Connection, DiagramMeta};

    fn comp(id: &str, label: &str, x: f32, y: f32) -> Component {
        Component {
            id: id.into(),
            label: label.into(),
            sublabel: Some("alt metin".into()),
            role: SemanticRole::Backend,
            x,
            y,
            width: 150.0,
            height: 56.0,
        }
    }

    fn diagram() -> ArchitectureDiagram {
        ArchitectureDiagram {
            meta: DiagramMeta {
                title: "t".into(),
                subtitle: None,
                locale: "tr".into(),
                visual_preset: VisualPreset::Editorial,
            },
            components: vec![
                comp(
                    "a",
                    "00ebbd47387f9a1c2e3d4b5a69788796a5b4c3d2e1f00112233445566778899aabbc.json",
                    40.0,
                    80.0,
                ),
                comp("b", "çekirdek", 340.0, 80.0),
                comp("c", "depo", 340.0, 240.0),
            ],
            connections: vec![
                Connection {
                    from: "a".into(),
                    to: "b".into(),
                    label: Some("kullanır ×4".into()),
                    line_style: String::new(),
                },
                Connection {
                    from: "b".into(),
                    to: "c".into(),
                    label: None,
                    line_style: "dashed".into(),
                },
                Connection {
                    from: "zz".into(),
                    to: "c".into(),
                    label: None,
                    line_style: String::new(),
                },
            ],
            regions: vec![],
            story_beats: vec![],
        }
    }

    #[test]
    fn long_names_are_fitted_inside_the_box_and_tooltip_keeps_the_full_name() {
        let s = Scene::from_architecture(&diagram());
        let n = &s.nodes[0];
        assert!(n.label.truncated);
        assert!(text::width(&n.label.text, n.label.font) <= n.rect.w - 16.0 + 0.01);
        assert!(n.tooltip.contains("aabbc.json"));
        assert!(n.label.text.ends_with(".json"));
    }

    #[test]
    fn edges_to_unknown_nodes_are_dropped_and_routes_are_clear() {
        let s = Scene::from_architecture(&diagram());
        assert_eq!(s.edges.len(), 2);
        assert!(s.edges.iter().all(|e| e.route.clear));
        assert_eq!(s.edges[1].kind, EdgeKind::Dashed);
        assert!(s.edges[0].route.label.is_some());
    }

    #[test]
    fn content_box_covers_everything_and_legend_counts_roles() {
        let s = Scene::from_architecture(&diagram());
        for n in &s.nodes {
            assert!(s.content.x <= n.rect.x && s.content.right() >= n.rect.right());
        }
        assert_eq!(s.legend, vec![(SemanticRole::Backend, 3)]);
    }

    #[test]
    fn routing_stats_report_clean_routes() {
        let s = Scene::from_architecture(&diagram());
        assert_eq!(s.stats.edges, 2);
        assert_eq!(s.stats.blind, 0);
        assert_eq!(s.stats.through_box, 0);
    }

    #[test]
    fn a_tour_is_derived_when_the_diagram_has_none() {
        let s = Scene::from_architecture(&diagram());
        assert!(!s.beats.is_empty());
        assert!(s.beats[0].nodes.len() >= 2);
    }
}
