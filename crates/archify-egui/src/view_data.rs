//! Per-diagram view state: scene, cached edge geometry, camera, focus animation and tour.

use crate::search;
use archify_egui_paint::paint::EdgeGeom;
use archify_ir::SemanticRole;
use archify_motion::{
    adjacent_focus, edge_dirs, EdgeDir, FocusAnim, FocusSet, HoverIntent, IntroTrace, Tour,
    TourStep, ViewState,
};
use archify_scene::{Rect, Scene};

/// What currently decides the dimming; a change restarts the focus transition.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum FocusKey {
    #[default]
    None,
    Hover(usize),
    Edge(usize),
    Selected(usize),
    Tour(usize),
    Legend(SemanticRole),
    Search(String),
    Route,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BeamColor {
    Out,
    In,
    Loop,
    Accent,
}

#[derive(Debug, Clone, Copy)]
pub struct BeamSpec {
    pub edge: usize,
    pub color: BeamColor,
    pub ambient: bool,
    pub phase: f32,
}

pub struct ViewData {
    pub scene: Scene,
    pub geoms: Vec<EdgeGeom>,
    pub pairs: Vec<(usize, usize)>,
    pub vs: ViewState,
    pub focus: FocusAnim,
    pub hover: HoverIntent,
    pub intro: IntroTrace,
    pub intro_t: f32,
    pub tour: Tour,
    /// `(beat, step)` of every tour step.
    pub tour_map: Vec<(usize, usize)>,
    pub selected: Option<usize>,
    pub legend_role: Option<SemanticRole>,
    pub hovered_edge: Option<usize>,
    pub route: Vec<usize>,
    pub route_start: Option<usize>,
    pub fitted: bool,
    /// The user moved the camera; a window resize no longer refits.
    pub touched: bool,
    pub last_viewport: [f32; 2],
    pub key: FocusKey,
    pub beam_t0: f32,
}

impl ViewData {
    pub fn new(scene: Scene, reduced_motion: bool) -> ViewData {
        let geoms = scene.edges.iter().map(EdgeGeom::of).collect();
        let pairs = scene.edge_pairs();
        let (steps, tour_map) = tour_steps(&scene);
        let mut vs = ViewState::default();
        vs.min_zoom = 0.04;
        vs.max_zoom = 5.0;
        let mut hover = HoverIntent::default();
        hover.reduced_motion = reduced_motion;
        let mut focus = FocusAnim::new(scene.nodes.len(), scene.edges.len());
        focus.reduced_motion = reduced_motion;
        ViewData {
            geoms,
            pairs,
            vs,
            focus,
            hover,
            intro: if reduced_motion {
                IntroTrace::skipped()
            } else {
                IntroTrace::new()
            },
            intro_t: 0.0,
            tour: Tour::new(steps),
            tour_map,
            selected: None,
            legend_role: None,
            hovered_edge: None,
            route: Vec::new(),
            route_start: None,
            fitted: false,
            touched: false,
            last_viewport: [0.0, 0.0],
            key: FocusKey::None,
            beam_t0: 0.0,
            scene,
        }
    }

    pub fn content_rect(&self) -> [f32; 4] {
        let c = self.scene.content;
        [c.x, c.y, c.w, c.h]
    }

    /// Union of the boxes at `idx` as `[x, y, w, h]`.
    pub fn nodes_rect(&self, idx: &[usize]) -> Option<[f32; 4]> {
        let mut acc: Option<Rect> = None;
        for &i in idx {
            let r = self.scene.nodes.get(i)?.rect;
            acc = Some(acc.map_or(r, |a| a.union(&r)));
        }
        acc.map(|r| [r.x, r.y, r.w, r.h])
    }

    /// Nodes of the tour step currently shown: the story so far inside its beat.
    pub fn tour_nodes(&self) -> Option<(Vec<usize>, usize)> {
        let i = self.tour.current()?;
        let (b, s) = *self.tour_map.get(i)?;
        let beat = self.scene.beats.get(b)?;
        Some((beat.nodes[..=s].to_vec(), beat.nodes[s]))
    }

    /// Framing for a tour step: previous, current and next box of the beat.
    pub fn tour_frame(&self) -> Option<[f32; 4]> {
        let i = self.tour.current()?;
        let (b, s) = *self.tour_map.get(i)?;
        let beat = self.scene.beats.get(b)?;
        let lo = s.saturating_sub(1);
        let hi = (s + 1).min(beat.nodes.len() - 1);
        self.nodes_rect(&beat.nodes[lo..=hi])
    }

    /// Dim level and focus set for the current key.
    pub fn focus_for(&self, key: &FocusKey, query: &str) -> Option<(FocusSet, f32)> {
        let n = self.scene.nodes.len();
        let e = self.scene.edges.len();
        let by_nodes = |set: &[usize], selected: Option<usize>| {
            let mut f = FocusSet {
                nodes: vec![false; n],
                edges: vec![false; e],
                selected,
            };
            for &i in set {
                f.nodes[i] = true;
            }
            for (k, &(a, b)) in self.pairs.iter().enumerate() {
                f.edges[k] = f.nodes[a] && f.nodes[b];
            }
            f
        };
        match key {
            FocusKey::None => None,
            FocusKey::Hover(i) => Some((
                adjacent_focus(&self.pairs, *i, n),
                archify_motion::DIM_HOVER,
            )),
            FocusKey::Selected(i) => Some((
                adjacent_focus(&self.pairs, *i, n),
                archify_motion::DIM_FOCUS,
            )),
            FocusKey::Edge(k) => {
                let (a, b) = self.pairs[*k];
                let mut f = by_nodes(&[a, b], None);
                f.edges[*k] = true;
                Some((f, archify_motion::DIM_HOVER))
            }
            FocusKey::Tour(_) => {
                let (nodes, current) = self.tour_nodes()?;
                Some((by_nodes(&nodes, Some(current)), archify_motion::DIM_FOCUS))
            }
            FocusKey::Legend(role) => {
                let set: Vec<usize> = (0..n)
                    .filter(|&i| self.scene.nodes[i].role == *role)
                    .collect();
                Some((by_nodes(&set, None), archify_motion::DIM_FOCUS))
            }
            FocusKey::Search(_) => {
                let set = self.search_hits(query);
                Some((by_nodes(&set, None), archify_motion::DIM_FOCUS))
            }
            FocusKey::Route => Some((by_nodes(&self.route, None), archify_motion::DIM_FOCUS)),
        }
    }

    pub fn search_hits(&self, query: &str) -> Vec<usize> {
        self.scene
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| search::matches(&n.tooltip, query) || search::matches(&n.id, query))
            .map(|(i, _)| i)
            .collect()
    }

    /// Beams to draw this frame.
    pub fn beams(&self, active: Option<usize>, ambient: bool) -> Vec<BeamSpec> {
        let mut out = Vec::new();
        if let Some(node) = active {
            for (edge, dir) in edge_dirs(&self.pairs, node) {
                out.push(BeamSpec {
                    edge,
                    color: match dir {
                        EdgeDir::Out => BeamColor::Out,
                        EdgeDir::In => BeamColor::In,
                        EdgeDir::Loop => BeamColor::Loop,
                    },
                    ambient: false,
                    phase: 0.0,
                });
            }
        } else if let Some((nodes, _)) = self.tour_nodes() {
            for w in nodes.windows(2) {
                for (k, &(a, b)) in self.pairs.iter().enumerate() {
                    if (a == w[0] && b == w[1]) || (a == w[1] && b == w[0]) {
                        out.push(BeamSpec {
                            edge: k,
                            color: BeamColor::Accent,
                            ambient: false,
                            phase: 0.0,
                        });
                    }
                }
            }
        } else if self.route.len() > 1 {
            for w in self.route.windows(2) {
                for (k, &(a, b)) in self.pairs.iter().enumerate() {
                    if a == w[0] && b == w[1] {
                        out.push(BeamSpec {
                            edge: k,
                            color: BeamColor::Accent,
                            ambient: false,
                            phase: 0.0,
                        });
                    }
                }
            }
        }
        for (k, e) in self.scene.edges.iter().enumerate() {
            if (ambient || e.animated) && !out.iter().any(|b| b.edge == k) {
                out.push(BeamSpec {
                    edge: k,
                    color: BeamColor::Accent,
                    ambient: true,
                    phase: (k as f32 * 0.37).fract(),
                });
            }
        }
        out
    }
}

/// Flattens the scene's beats into tour steps: one per box, so the story builds up node by
/// node like the original ("02/05 A → B").
fn tour_steps(scene: &Scene) -> (Vec<TourStep>, Vec<(usize, usize)>) {
    let mut steps = Vec::new();
    let mut map = Vec::new();
    for (b, beat) in scene.beats.iter().enumerate() {
        for (s, &node) in beat.nodes.iter().enumerate() {
            steps.push(TourStep {
                nodes: vec![node],
                title: beat.title.clone(),
                desc: String::new(),
            });
            map.push((b, s));
        }
    }
    (steps, map)
}

impl ViewData {
    /// Shortest directed path between two boxes (BFS over the edges); includes both ends.
    pub fn shortest_path(&self, from: usize, to: usize) -> Option<Vec<usize>> {
        let n = self.scene.nodes.len();
        let mut prev: Vec<Option<usize>> = vec![None; n];
        let mut seen = vec![false; n];
        let mut queue = std::collections::VecDeque::from([from]);
        seen[from] = true;
        while let Some(u) = queue.pop_front() {
            if u == to {
                let mut path = vec![to];
                let mut cur = to;
                while let Some(p) = prev[cur] {
                    path.push(p);
                    cur = p;
                }
                path.reverse();
                return Some(path);
            }
            for &(a, b) in &self.pairs {
                if a == u && !seen[b] {
                    seen[b] = true;
                    prev[b] = Some(u);
                    queue.push_back(b);
                }
            }
        }
        None
    }

    pub fn set_reduced_motion(&mut self, on: bool) {
        self.hover.reduced_motion = on;
        self.focus.reduced_motion = on;
        self.intro = if on {
            IntroTrace::skipped()
        } else {
            IntroTrace::new()
        };
    }
}

/// Below this zoom box labels stop being readable; a bigger diagram opens on its main area.
pub const MIN_READABLE_ZOOM: f32 = 0.6;

impl ViewData {
    /// Camera for the first view: the whole diagram when it stays readable, otherwise the
    /// region (or neighbourhood) of the busiest box at a readable scale. "Fit all" stays one
    /// click away.
    pub fn initial_view(&self, viewport: [f32; 2]) -> (f32, [f32; 2]) {
        let pad = 56.0;
        let all = ViewState::fit_rect(self.content_rect(), viewport, pad, 1.25);
        let n = self.scene.nodes.len();
        if all.0 >= MIN_READABLE_ZOOM || n < 2 {
            return all;
        }
        let mut degree = vec![0usize; n];
        for &(a, b) in &self.pairs {
            degree[a] += 1;
            degree[b] += 1;
        }
        let hub = (0..n)
            .max_by_key(|&i| (degree[i], std::cmp::Reverse(i)))
            .unwrap_or(0);
        let c = self.scene.nodes[hub].rect;
        let centre = [c.cx(), c.cy()];
        let region = self
            .scene
            .regions
            .iter()
            .filter(|r| r.rect.contains_strict(centre))
            .min_by(|a, b| (a.rect.w * a.rect.h).total_cmp(&(b.rect.w * b.rect.h)))
            .map(|r| [r.rect.x, r.rect.y, r.rect.w, r.rect.h]);
        let target = region.or_else(|| {
            let mut idx = vec![hub];
            for &(a, b) in &self.pairs {
                if a == hub {
                    idx.push(b);
                } else if b == hub {
                    idx.push(a);
                }
            }
            self.nodes_rect(&idx)
        });
        if let Some(t) = target {
            let fit = ViewState::fit_rect(t, viewport, pad, 1.25);
            if fit.0 >= MIN_READABLE_ZOOM {
                return fit;
            }
        }
        let z = MIN_READABLE_ZOOM;
        (
            z,
            [
                viewport[0] / 2.0 - centre[0] * z,
                viewport[1] / 2.0 - centre[1] * z,
            ],
        )
    }
}
