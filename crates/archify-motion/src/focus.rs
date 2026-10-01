//! Hover / focus dimming: per-node and per-edge opacity transitions plus the
//! 90 ms hover-intent delay.

use crate::transition::{Transition, HOVER_SECS};

/// Which items stay fully visible. Everything else dims.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FocusSet {
    pub nodes: Vec<bool>,
    pub edges: Vec<bool>,
    pub selected: Option<usize>,
}

/// Node + its neighbours + connecting edges (Intent Trace / click focus).
pub fn adjacent_focus(edges: &[(usize, usize)], node: usize, n_nodes: usize) -> FocusSet {
    let mut set = FocusSet {
        nodes: vec![false; n_nodes],
        edges: vec![false; edges.len()],
        selected: Some(node),
    };
    if node < n_nodes {
        set.nodes[node] = true;
    }
    for (i, &(a, b)) in edges.iter().enumerate() {
        if a == node || b == node {
            set.edges[i] = true;
            for n in [a, b] {
                if n < n_nodes {
                    set.nodes[n] = true;
                }
            }
        }
    }
    set
}

#[derive(Debug, Clone, Default)]
pub struct FocusAnim {
    nodes: Vec<Transition>,
    edges: Vec<Transition>,
    glow: Vec<Transition>,
    /// Snap instead of easing.
    pub reduced_motion: bool,
}

impl FocusAnim {
    pub fn new(n_nodes: usize, n_edges: usize) -> Self {
        let mut s = Self::default();
        s.resize(n_nodes, n_edges);
        s
    }

    /// Match a new graph size (new slots start fully visible, no glow).
    pub fn resize(&mut self, n_nodes: usize, n_edges: usize) {
        self.nodes.resize(n_nodes, Transition::new(1.0));
        self.glow.resize(n_nodes, Transition::new(0.0));
        self.edges.resize(n_edges, Transition::new(1.0));
    }

    /// Retarget every item: matched -> 1.0, others -> `dim`; `None` -> all 1.0.
    pub fn set_focus(&mut self, focus: Option<&FocusSet>, dim: f32) {
        let secs = if self.reduced_motion { 0.0 } else { HOVER_SECS };
        let pick = |set: Option<&Vec<bool>>, i: usize| match (focus, set) {
            (None, _) => 1.0,
            (Some(_), Some(v)) if v.get(i).copied().unwrap_or(false) => 1.0,
            _ => dim,
        };
        for (i, t) in self.nodes.iter_mut().enumerate() {
            t.set_target(pick(focus.map(|f| &f.nodes), i), secs);
        }
        for (i, t) in self.edges.iter_mut().enumerate() {
            t.set_target(pick(focus.map(|f| &f.edges), i), secs);
        }
        let sel = focus.and_then(|f| f.selected);
        for (i, t) in self.glow.iter_mut().enumerate() {
            t.set_target(if sel == Some(i) { 1.0 } else { 0.0 }, secs);
        }
    }

    /// Advance all transitions; `true` while any is still moving.
    pub fn update(&mut self, dt: f32) -> bool {
        let mut any = false;
        for t in self
            .nodes
            .iter_mut()
            .chain(self.edges.iter_mut())
            .chain(self.glow.iter_mut())
        {
            any |= t.update(dt);
        }
        any
    }

    pub fn node_alpha(&self, i: usize) -> f32 {
        self.nodes.get(i).map_or(1.0, Transition::value)
    }

    pub fn edge_alpha(&self, i: usize) -> f32 {
        self.edges.get(i).map_or(1.0, Transition::value)
    }

    pub fn node_glow(&self, i: usize) -> f32 {
        self.glow.get(i).map_or(0.0, Transition::value)
    }
}

/// Delays activation of a hovered item by `delay` seconds (entering only).
/// Leaving is instant. While moving from active item A to B, A stays active
/// until B passes the delay.
#[derive(Debug, Clone)]
pub struct HoverIntent {
    pub delay: f32,
    pub reduced_motion: bool,
    candidate: Option<usize>,
    elapsed: f32,
    active: Option<usize>,
}

impl Default for HoverIntent {
    fn default() -> Self {
        Self {
            delay: 0.09,
            reduced_motion: false,
            candidate: None,
            elapsed: 0.0,
            active: None,
        }
    }
}

impl HoverIntent {
    pub fn update(&mut self, dt: f32, hovered: Option<usize>) -> Option<usize> {
        let Some(h) = hovered else {
            self.candidate = None;
            self.active = None;
            return None;
        };
        if self.candidate != Some(h) {
            self.candidate = Some(h);
            self.elapsed = 0.0;
        }
        if self.active != Some(h) {
            self.elapsed += dt.max(0.0);
            if self.reduced_motion || self.elapsed >= self.delay {
                self.active = Some(h);
            }
        }
        self.active
    }

    pub fn active(&self) -> Option<usize> {
        self.active
    }

    /// A hover is waiting out its delay (caller must keep repainting).
    pub fn is_pending(&self) -> bool {
        self.candidate.is_some() && self.candidate != self.active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjacent_focus_marks_neighbours_and_edges() {
        let edges = [(0, 1), (1, 2), (3, 4)];
        let f = adjacent_focus(&edges, 1, 5);
        assert_eq!(f.nodes, vec![true, true, true, false, false]);
        assert_eq!(f.edges, vec![true, true, false]);
        assert_eq!(f.selected, Some(1));
    }

    #[test]
    fn dims_then_restores_smoothly() {
        let edges = [(0, 1), (2, 3)];
        let mut a = FocusAnim::new(4, 2);
        a.set_focus(Some(&adjacent_focus(&edges, 0, 4)), 0.2);
        assert!(a.update(1.0 / 60.0));
        let mid = a.node_alpha(3);
        assert!(mid < 1.0 && mid > 0.2);
        while a.update(1.0 / 60.0) {}
        assert!((a.node_alpha(3) - 0.2).abs() < 1e-6);
        assert_eq!(a.node_alpha(1), 1.0);
        assert_eq!(a.edge_alpha(1), 0.2);
        assert_eq!(a.node_glow(0), 1.0);
        a.set_focus(None, 0.2);
        while a.update(1.0 / 60.0) {}
        assert_eq!(a.node_alpha(3), 1.0);
        assert_eq!(a.node_glow(0), 0.0);
    }

    #[test]
    fn reduced_motion_snaps() {
        let mut a = FocusAnim::new(2, 0);
        a.reduced_motion = true;
        a.set_focus(Some(&adjacent_focus(&[], 0, 2)), 0.13);
        assert_eq!(a.node_alpha(1), 0.13);
        assert!(!a.update(0.016));
    }

    #[test]
    fn hover_intent_delays_entry_and_leaves_instantly() {
        let mut h = HoverIntent::default();
        assert_eq!(h.update(0.05, Some(3)), None);
        assert!(h.is_pending());
        assert_eq!(h.update(0.05, Some(3)), Some(3));
        assert!(!h.is_pending());
        assert_eq!(h.update(0.0, None), None);
        let mut r = HoverIntent {
            reduced_motion: true,
            ..Default::default()
        };
        assert_eq!(r.update(0.0, Some(1)), Some(1));
    }
}
