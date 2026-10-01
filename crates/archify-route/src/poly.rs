//! Polyline helpers: simplification, rounded corners and validation.

use crate::geom::{Pt, Rect};

/// Drops repeated points and middle points of straight runs.
pub fn simplify(points: &[Pt]) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::with_capacity(points.len());
    for &p in points {
        if let Some(&last) = out.last() {
            if (last[0] - p[0]).abs() < 0.01 && (last[1] - p[1]).abs() < 0.01 {
                continue;
            }
        }
        while out.len() >= 2 {
            let a = out[out.len() - 2];
            let b = out[out.len() - 1];
            let straight_x = (a[0] - b[0]).abs() < 0.01 && (b[0] - p[0]).abs() < 0.01;
            let straight_y = (a[1] - b[1]).abs() < 0.01 && (b[1] - p[1]).abs() < 0.01;
            if straight_x || straight_y {
                out.pop();
            } else {
                break;
            }
        }
        out.push(p);
    }
    out
}

/// Grid lines that merged within tolerance leave hair-width offsets; make those segments
/// exactly axis-aligned by snapping each point to its predecessor's coordinate.
pub fn snap_axes(points: &mut [Pt]) {
    for i in 1..points.len() {
        let prev = points[i - 1];
        for (v, p) in points[i].iter_mut().zip(prev) {
            if (*v - p).abs() < 0.1 {
                *v = p;
            }
        }
    }
}

pub fn length(points: &[Pt]) -> f32 {
    points
        .windows(2)
        .map(|w| ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt())
        .sum()
}

/// One step of a rounded path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathCmd {
    MoveTo(Pt),
    LineTo(Pt),
    /// Quadratic bezier: control point, end point.
    QuadTo(Pt, Pt),
}

/// Path with every inner corner replaced by a quadratic of radius `r` (clamped to half of
/// the adjacent segments), exactly how the original draws `M L Q L Q L`.
pub fn rounded_path(points: &[Pt], r: f32) -> Vec<PathCmd> {
    let mut cmds = Vec::with_capacity(points.len() * 2);
    let Some(&first) = points.first() else {
        return cmds;
    };
    cmds.push(PathCmd::MoveTo(first));
    for i in 1..points.len() {
        let cur = points[i];
        if i + 1 == points.len() {
            cmds.push(PathCmd::LineTo(cur));
            break;
        }
        let prev = points[i - 1];
        let next = points[i + 1];
        let l_in = seg_len(prev, cur);
        let l_out = seg_len(cur, next);
        let rr = r.min(l_in * 0.5).min(l_out * 0.5);
        if rr < 1.0 {
            cmds.push(PathCmd::LineTo(cur));
            continue;
        }
        let before = toward(cur, prev, rr);
        let after = toward(cur, next, rr);
        cmds.push(PathCmd::LineTo(before));
        cmds.push(PathCmd::QuadTo(cur, after));
    }
    cmds
}

/// The rounded path flattened to a polyline (`steps` points per corner).
pub fn flatten_rounded(points: &[Pt], r: f32, steps: usize) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::with_capacity(points.len() + steps * 4);
    let mut cur = [0.0, 0.0];
    for cmd in rounded_path(points, r) {
        match cmd {
            PathCmd::MoveTo(p) | PathCmd::LineTo(p) => {
                out.push(p);
                cur = p;
            }
            PathCmd::QuadTo(c, e) => {
                for s in 1..=steps {
                    let t = s as f32 / steps as f32;
                    let u = 1.0 - t;
                    out.push([
                        u * u * cur[0] + 2.0 * u * t * c[0] + t * t * e[0],
                        u * u * cur[1] + 2.0 * u * t * c[1] + t * t * e[1],
                    ]);
                }
                cur = e;
            }
        }
    }
    out
}

fn seg_len(a: Pt, b: Pt) -> f32 {
    ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt()
}

/// Point `d` px from `from` towards `to`.
fn toward(from: Pt, to: Pt, d: f32) -> Pt {
    let l = seg_len(from, to).max(1e-6);
    [
        from[0] + (to[0] - from[0]) / l * d,
        from[1] + (to[1] - from[1]) / l * d,
    ]
}

/// True when any segment passes through the interior of `r`.
pub fn hits_rect(points: &[Pt], r: &Rect) -> bool {
    points.windows(2).any(|w| r.hit_by_segment(w[0], w[1]))
}

/// Length (px) of collinear overlap between two polylines (axis-aligned segments only).
pub fn overlap_length(a: &[Pt], b: &[Pt]) -> f32 {
    let mut total = 0.0;
    for sa in a.windows(2) {
        for sb in b.windows(2) {
            total += seg_overlap(sa[0], sa[1], sb[0], sb[1]);
        }
    }
    total
}

fn seg_overlap(a0: Pt, a1: Pt, b0: Pt, b1: Pt) -> f32 {
    const EPS: f32 = 0.6;
    let a_vert = (a0[0] - a1[0]).abs() < 0.01;
    let b_vert = (b0[0] - b1[0]).abs() < 0.01;
    if a_vert != b_vert {
        return 0.0;
    }
    let (ca, cb, (alo, ahi), (blo, bhi)) = if a_vert {
        (a0[0], b0[0], minmax(a0[1], a1[1]), minmax(b0[1], b1[1]))
    } else {
        (a0[1], b0[1], minmax(a0[0], a1[0]), minmax(b0[0], b1[0]))
    };
    if (ca - cb).abs() > EPS {
        return 0.0;
    }
    (ahi.min(bhi) - alo.max(blo)).max(0.0)
}

fn minmax(a: f32, b: f32) -> (f32, f32) {
    (a.min(b), a.max(b))
}

/// Proper crossings between two orthogonal polylines (touching ends do not count).
pub fn crossings(a: &[Pt], b: &[Pt]) -> usize {
    let mut n = 0;
    for sa in a.windows(2) {
        for sb in b.windows(2) {
            let a_vert = (sa[0][0] - sa[1][0]).abs() < 0.01;
            let b_vert = (sb[0][0] - sb[1][0]).abs() < 0.01;
            if a_vert == b_vert {
                continue;
            }
            let (v0, v1, h0, h1) = if a_vert {
                (sa[0], sa[1], sb[0], sb[1])
            } else {
                (sb[0], sb[1], sa[0], sa[1])
            };
            let x = v0[0];
            let y = h0[1];
            let (vlo, vhi) = minmax(v0[1], v1[1]);
            let (hlo, hhi) = minmax(h0[0], h1[0]);
            if x > hlo + 0.5 && x < hhi - 0.5 && y > vlo + 0.5 && y < vhi - 0.5 {
                n += 1;
            }
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simplify_removes_collinear_points() {
        let s = simplify(&[
            [0.0, 0.0],
            [5.0, 0.0],
            [10.0, 0.0],
            [10.0, 0.0],
            [10.0, 8.0],
        ]);
        assert_eq!(s, vec![[0.0, 0.0], [10.0, 0.0], [10.0, 8.0]]);
    }

    #[test]
    fn rounded_corner_uses_radius_eight() {
        let cmds = rounded_path(&[[0.0, 0.0], [100.0, 0.0], [100.0, 100.0]], 8.0);
        assert_eq!(cmds[1], PathCmd::LineTo([92.0, 0.0]));
        assert_eq!(cmds[2], PathCmd::QuadTo([100.0, 0.0], [100.0, 8.0]));
    }

    #[test]
    fn radius_is_clamped_on_short_segments() {
        let cmds = rounded_path(&[[0.0, 0.0], [10.0, 0.0], [10.0, 100.0]], 8.0);
        assert_eq!(cmds[1], PathCmd::LineTo([5.0, 0.0]));
    }

    #[test]
    fn overlap_and_crossing_detection() {
        let a = [[0.0, 0.0], [100.0, 0.0]];
        let b = [[50.0, 0.0], [150.0, 0.0]];
        assert!((overlap_length(&a, &b) - 50.0).abs() < 0.1);
        let c = [[50.0, -10.0], [50.0, 10.0]];
        assert_eq!(crossings(&a, &c), 1);
        assert_eq!(crossings(&a, &b), 0);
    }
}
