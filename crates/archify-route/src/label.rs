//! Edge-label plate placement: on a long segment, clear of boxes, region titles and other
//! labels, preferring spots where no other edge runs underneath.

use crate::geom::{Pt, Rect};

pub const LABEL_H: f32 = 14.0;
/// Gap between a horizontal edge and the plate sitting on it.
const LIFT: f32 = 4.0;
const FRACTIONS: [f32; 7] = [0.5, 0.35, 0.65, 0.2, 0.8, 0.1, 0.9];

/// Places one plate of width `w`. `others` are all routes (the edge's own included, skipped
/// via `own`). Returns `None` when every candidate collides with a box.
pub fn place(
    route: &[Pt],
    w: f32,
    own: usize,
    routes: &[Vec<Pt>],
    nets: &[usize],
    blockers: &[Rect],
    placed: &[Rect],
) -> Option<Rect> {
    let mut best: Option<(f32, Rect)> = None;
    for (k, seg) in route.windows(2).enumerate() {
        let (a, b) = (seg[0], seg[1]);
        let horizontal = (a[1] - b[1]).abs() < 0.01;
        let len = (b[0] - a[0]).abs() + (b[1] - a[1]).abs();
        let need = if horizontal { w + 10.0 } else { LABEL_H + 12.0 };
        if len < need {
            continue;
        }
        let stub = k == 0 || k + 2 == route.len();
        for (fi, t) in FRACTIONS.iter().enumerate() {
            // keep the plate inside the segment
            let margin = need * 0.5;
            let frac = t.clamp(margin / len, 1.0 - margin / len);
            let p = [a[0] + (b[0] - a[0]) * frac, a[1] + (b[1] - a[1]) * frac];
            let cands: [(Rect, f32); 3] = if horizontal {
                [
                    (
                        Rect::new(p[0] - w * 0.5, p[1] - LIFT - LABEL_H, w, LABEL_H),
                        0.0,
                    ),
                    (Rect::new(p[0] - w * 0.5, p[1] + LIFT, w, LABEL_H), 1.5),
                    (
                        Rect::new(p[0] - w * 0.5, p[1] - LABEL_H * 0.5, w, LABEL_H),
                        6.0,
                    ),
                ]
            } else {
                [
                    (
                        Rect::new(p[0] - w * 0.5, p[1] - LABEL_H * 0.5, w, LABEL_H),
                        0.0,
                    ),
                    (
                        Rect::new(p[0] + LIFT, p[1] - LABEL_H * 0.5, w, LABEL_H),
                        2.0,
                    ),
                    (
                        Rect::new(p[0] - LIFT - w, p[1] - LABEL_H * 0.5, w, LABEL_H),
                        2.5,
                    ),
                ]
            };
            for (r, base) in cands {
                if blockers.iter().any(|o| o.overlaps(&r))
                    || placed.iter().any(|o| o.inflate(2.0).overlaps(&r))
                {
                    continue;
                }
                let under = routes
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != own && nets[*i] != nets[own])
                    .map(|(_, other)| {
                        other
                            .windows(2)
                            .filter(|s| segment_in(&r, s[0], s[1]))
                            .count()
                    })
                    .sum::<usize>() as f32;
                let score = base + under * 12.0 + fi as f32 * 0.8 + if stub { 3.0 } else { 0.0 }
                    - len.min(240.0) * 0.02;
                if best.as_ref().is_none_or(|(s, _)| score < *s) {
                    best = Some((score, r));
                }
            }
        }
    }
    best.map(|(_, r)| r)
}

/// Does the segment run through the plate?
fn segment_in(r: &Rect, a: Pt, b: Pt) -> bool {
    r.inflate(1.0).hit_by_segment(a, b)
}

/// Last resort: a plate centred on the middle of the longest segment.
pub fn fallback(route: &[Pt], w: f32) -> Rect {
    let mut best = (0.0, [0.0, 0.0]);
    for s in route.windows(2) {
        let len = (s[1][0] - s[0][0]).abs() + (s[1][1] - s[0][1]).abs();
        if len >= best.0 {
            best = (len, [(s[0][0] + s[1][0]) * 0.5, (s[0][1] + s[1][1]) * 0.5]);
        }
    }
    Rect::new(best.1[0] - w * 0.5, best.1[1] - LABEL_H * 0.5, w, LABEL_H)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plate_sits_above_a_long_horizontal_segment() {
        let route = vec![[0.0, 100.0], [300.0, 100.0]];
        let r = place(
            &route,
            60.0,
            0,
            std::slice::from_ref(&route),
            &[0],
            &[],
            &[],
        )
        .unwrap();
        assert!((r.cx() - 150.0).abs() < 0.1);
        assert!(r.bottom() <= 100.0 - LIFT + 0.01);
    }

    #[test]
    fn plate_avoids_blockers_and_other_plates() {
        let route = vec![[0.0, 100.0], [300.0, 100.0]];
        let blocker = Rect::new(120.0, 70.0, 60.0, 40.0);
        let r = place(
            &route,
            60.0,
            0,
            std::slice::from_ref(&route),
            &[0],
            &[blocker],
            &[],
        )
        .unwrap();
        assert!(!blocker.overlaps(&r));
        let again = place(
            &route,
            60.0,
            0,
            std::slice::from_ref(&route),
            &[0],
            &[blocker],
            &[r],
        )
        .unwrap();
        assert!(!again.inflate(2.0).overlaps(&r));
    }
}
