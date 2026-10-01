//! archify-scene: the resolved picture of a diagram, shared by the egui canvas and the SVG
//! renderer.
//!
//! A [`Scene`] holds boxes with fitted text, regions with their title plates, routed edges
//! (orthogonal polylines, label plates, arrowheads), the legend and the tour beats. Building it
//! is the only place where layout, routing and text fitting happen, so both outputs agree.

mod build;
pub mod layered;
mod tour;

pub use archify_route::{Arrow, Pt, Rect, RoutedEdge};
pub use build::{dataflow_scene, delta_scene};
pub use tour::{BeatStep, Relation, SceneBeat};

use archify_ir::{ArchitectureDiagram, DiffStatus, SemanticRole, VisualPreset};
use archify_style::{EdgeKind, ROLE_ORDER};

/// One line of text placed inside a box (`y` is the line centre below the box top).
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub text: String,
    pub font: f32,
    pub y: f32,
    pub truncated: bool,
}

#[derive(Debug, Clone)]
pub struct SceneNode {
    pub id: String,
    pub rect: Rect,
    pub role: SemanticRole,
    pub label: Line,
    pub sublabel: Option<Line>,
    pub tag: Option<Line>,
    /// Full, untruncated text for tooltips: label, sublabel, region, tag.
    pub tooltip: String,
    /// Title of the smallest region holding the box.
    pub context: Option<String>,
    pub status: Option<DiffStatus>,
}

#[derive(Debug, Clone)]
pub struct SceneRegion {
    pub id: String,
    pub rect: Rect,
    pub title: Line,
    /// Plate behind the title (also an obstacle for edges).
    pub title_plate: Rect,
    pub sub: Option<Line>,
    pub sub_plate: Option<Rect>,
}

#[derive(Debug, Clone)]
pub struct SceneEdge {
    pub from: usize,
    pub to: usize,
    pub label: Option<String>,
    pub full_label: Option<String>,
    pub kind: EdgeKind,
    /// `line_style == "animated"`: the edge flows even at rest.
    pub animated: bool,
    pub route: RoutedEdge,
    pub status: Option<DiffStatus>,
}

#[derive(Debug, Clone)]
pub struct Scene {
    pub title: String,
    pub subtitle: Option<String>,
    pub locale: String,
    pub preset: VisualPreset,
    pub nodes: Vec<SceneNode>,
    pub regions: Vec<SceneRegion>,
    pub edges: Vec<SceneEdge>,
    /// Bounding box of everything drawn (boxes, regions, edges, plates), no margin.
    pub content: Rect,
    /// Roles present with their box counts, in legend order.
    pub legend: Vec<(SemanticRole, usize)>,
    pub beats: Vec<SceneBeat>,
    pub stats: RouteStats,
}

/// How well the routing came out (acceptance numbers).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RouteStats {
    pub edges: usize,
    /// Edges the search could not route (blind fallback).
    pub blind: usize,
    /// Edges whose path passes through the inside of a box.
    pub through_box: usize,
}

/// Margin around the content in exported pictures.
pub const MARGIN: f32 = 40.0;

impl Scene {
    pub fn from_architecture(d: &ArchitectureDiagram) -> Scene {
        build::architecture_scene(d)
    }

    /// Index of the node with this id.
    pub fn node_index(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    /// `(from, to)` pairs of all edges.
    pub fn edge_pairs(&self) -> Vec<(usize, usize)> {
        self.edges.iter().map(|e| (e.from, e.to)).collect()
    }

    /// Picture size with margin, title band and legend row: `(x, y, w, h)` of the viewBox.
    pub fn view_box(&self, title_band: f32, legend_band: f32) -> Rect {
        let c = self.content;
        Rect::new(
            c.x - MARGIN,
            c.y - MARGIN - title_band,
            c.w + 2.0 * MARGIN,
            c.h + 2.0 * MARGIN + title_band + legend_band,
        )
    }
}

pub(crate) fn legend_of(nodes: &[SceneNode]) -> Vec<(SemanticRole, usize)> {
    ROLE_ORDER
        .iter()
        .filter_map(|&r| {
            let n = nodes.iter().filter(|x| x.role == r).count();
            (n > 0).then_some((r, n))
        })
        .collect()
}
