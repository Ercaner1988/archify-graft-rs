//! Flowing light beams along routed (orthogonal) polylines.
//!
//! Colour convention (chosen by the caller): `Out` = frontend-stroke (cyan),
//! `In` = database-stroke (violet), `Loop` = security-stroke (rose). All beams
//! travel source -> target; direction only changes the colour.

pub type Pt = [f32; 2];

/// Total length of a polyline.
pub fn polyline_len(points: &[Pt]) -> f32 {
    points
        .windows(2)
        .map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]))
        .sum()
}

/// Position and unit direction at arc length `s` (clamped to the path).
pub fn point_at(points: &[Pt], s: f32) -> (Pt, Pt) {
    let mut left = s.max(0.0);
    let mut dir = [1.0, 0.0];
    for w in points.windows(2) {
        let (dx, dy) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
        let len = dx.hypot(dy);
        if len <= f32::EPSILON {
            continue;
        }
        dir = [dx / len, dy / len];
        if left <= len {
            return ([w[0][0] + dir[0] * left, w[0][1] + dir[1] * left], dir);
        }
        left -= len;
    }
    (points.last().copied().unwrap_or([0.0, 0.0]), dir)
}

/// Sub-polyline between arc lengths `s0 <= s1` (empty if degenerate).
pub fn slice(points: &[Pt], s0: f32, s1: f32) -> Vec<Pt> {
    let total = polyline_len(points);
    let (s0, s1) = (s0.max(0.0), s1.min(total));
    if s1 <= s0 || points.len() < 2 {
        return Vec::new();
    }
    let mut out = vec![point_at(points, s0).0];
    let mut acc = 0.0;
    for w in points.windows(2) {
        acc += (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]);
        if acc > s0 && acc < s1 {
            out.push(w[1]);
        }
    }
    out.push(point_at(points, s1).0);
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeDir {
    In,
    Out,
    Loop,
}

/// Classify every edge touching `node`.
pub fn edge_dirs(edges: &[(usize, usize)], node: usize) -> Vec<(usize, EdgeDir)> {
    edges
        .iter()
        .enumerate()
        .filter_map(|(i, &(a, b))| match (a == node, b == node) {
            (true, true) => Some((i, EdgeDir::Loop)),
            (true, false) => Some((i, EdgeDir::Out)),
            (false, true) => Some((i, EdgeDir::In)),
            _ => None,
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
pub struct BeamCycle {
    /// Seconds per pass (default 1.4).
    pub period: f32,
    /// Beam length as a fraction of the path (default 0.22) ...
    pub len_frac: f32,
    /// ... clamped to this range in world px.
    pub len_min: f32,
    pub len_max: f32,
}

impl Default for BeamCycle {
    fn default() -> Self {
        Self {
            period: 1.4,
            len_frac: 0.22,
            len_min: 24.0,
            len_max: 90.0,
        }
    }
}

impl BeamCycle {
    /// Thin, slow ambient variant (period 3.2 s).
    pub fn ambient() -> Self {
        Self {
            period: 3.2,
            len_frac: 0.12,
            len_min: 12.0,
            len_max: 40.0,
        }
    }

    fn progress(&self, t: f32, phase: f32) -> f32 {
        (t / self.period + phase).rem_euclid(1.0)
    }

    /// Visibility of the beam this cycle: `sin(pi * progress)`.
    pub fn intensity(&self, t: f32, phase: f32) -> f32 {
        (std::f32::consts::PI * self.progress(t, phase)).sin()
    }

    /// `(head_s, tail_s)` arc-length window inside `0..=total_len`.
    /// The head enters at the source, the tail leaves at the target; clipped to the path.
    pub fn window(&self, total_len: f32, t: f32, phase: f32) -> (f32, f32) {
        if total_len <= 0.0 {
            return (0.0, 0.0);
        }
        let len = (self.len_frac * total_len)
            .clamp(self.len_min, self.len_max)
            .min(total_len);
        let head = self.progress(t, phase) * (total_len + len);
        (head.min(total_len), (head - len).max(0.0))
    }
}

/// Hover beam window with the default 1.4 s cycle.
pub fn beam_window(total_len: f32, t: f32, phase: f32) -> (f32, f32) {
    BeamCycle::default().window(total_len, t, phase)
}

/// Visibility `sin(pi * progress)` of the default hover beam.
pub fn beam_intensity(t: f32, phase: f32) -> f32 {
    BeamCycle::default().intensity(t, phase)
}

/// Thin slow ambient window; each edge is offset by its index.
pub fn ambient_window(total_len: f32, t: f32, edge_index: usize) -> (f32, f32) {
    let phase = (edge_index as f32 * 0.37).fract();
    BeamCycle::ambient().window(total_len, t, phase)
}

/// Split `tail..head` into `n` pieces whose alpha rises from transparent (tail)
/// to bright (head): `(s0, s1, alpha)`.
pub fn gradient_pieces(head: f32, tail: f32, n: usize) -> Vec<(f32, f32, f32)> {
    if n == 0 || head <= tail {
        return Vec::new();
    }
    let step = (head - tail) / n as f32;
    (0..n)
        .map(|k| {
            let s0 = tail + step * k as f32;
            let s1 = if k + 1 == n { head } else { s0 + step };
            (s0, s1, (k + 1) as f32 / n as f32)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const L: [Pt; 3] = [[0.0, 0.0], [100.0, 0.0], [100.0, 50.0]];

    #[test]
    fn polyline_helpers() {
        assert_eq!(polyline_len(&L), 150.0);
        let (p, d) = point_at(&L, 120.0);
        assert_eq!((p, d), ([100.0, 20.0], [0.0, 1.0]));
        assert_eq!(point_at(&L, 999.0).0, [100.0, 50.0]);
        let s = slice(&L, 80.0, 120.0);
        assert_eq!(s, vec![[80.0, 0.0], [100.0, 0.0], [100.0, 20.0]]);
        assert!((polyline_len(&s) - 40.0).abs() < 1e-4);
        assert!(slice(&L, 10.0, 10.0).is_empty());
    }

    #[test]
    fn edge_dirs_classify() {
        let e = [(0, 1), (2, 0), (0, 0), (3, 4)];
        assert_eq!(
            edge_dirs(&e, 0),
            vec![(0, EdgeDir::Out), (1, EdgeDir::In), (2, EdgeDir::Loop)]
        );
    }

    #[test]
    fn window_stays_inside_path_and_travels_forward() {
        let total = 300.0;
        let mut last_head = -1.0;
        for i in 0..140 {
            let t = i as f32 * 0.01;
            let (head, tail) = beam_window(total, t, 0.0);
            assert!(0.0 <= tail && tail <= head && head <= total);
            assert!(head >= last_head || t > 1.3);
            last_head = head;
            assert!((0.0..=1.0).contains(&beam_intensity(t, 0.0)));
        }
        assert_eq!(beam_window(0.0, 1.0, 0.0), (0.0, 0.0));
        // short paths: beam never longer than the path
        let (h, t) = beam_window(10.0, 0.7, 0.0);
        assert!(h - t <= 10.0);
    }

    #[test]
    fn phase_shifts_edges_apart() {
        assert_ne!(beam_window(300.0, 0.5, 0.0), beam_window(300.0, 0.5, 0.5));
        assert_ne!(ambient_window(300.0, 1.0, 0), ambient_window(300.0, 1.0, 1));
    }

    #[test]
    fn gradient_pieces_cover_the_window() {
        let p = gradient_pieces(90.0, 30.0, 6);
        assert_eq!(p.len(), 6);
        let total: f32 = p.iter().map(|&(a, b, _)| b - a).sum();
        assert!((total - 60.0).abs() < 1e-4);
        assert_eq!(p[0].0, 30.0);
        assert_eq!(p[5].1, 90.0);
        assert!(p.windows(2).all(|w| w[0].2 < w[1].2));
        assert_eq!(p[5].2, 1.0);
        assert!(gradient_pieces(10.0, 10.0, 4).is_empty());
    }
}
