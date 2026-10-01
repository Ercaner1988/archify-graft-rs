//! Side selection and port spreading.

use crate::geom::{Pt, Rect, Side};
use std::collections::HashMap;

/// Tightest port pitch we accept before edges spill onto another side.
const MIN_PITCH: f32 = 6.0;
/// Widest port pitch (original `PORT_MAX_SPACING`).
const MAX_PITCH: f32 = 14.0;

/// Sides an edge leaves `a` and enters `b` through: the facing pair along the axis where the
/// boxes are separated, the dominant axis when they are diagonal.
pub fn default_sides(a: &Rect, b: &Rect) -> (Side, Side) {
    let gap_x = (b.x - a.right()).max(a.x - b.right());
    let gap_y = (b.y - a.bottom()).max(a.y - b.bottom());
    let (dx, dy) = (b.cx() - a.cx(), b.cy() - a.cy());
    let horizontal = if gap_x > 0.0 && gap_y <= 0.0 {
        true
    } else if gap_y > 0.0 && gap_x <= 0.0 {
        false
    } else {
        dx.abs() >= dy.abs()
    };
    if horizontal {
        if dx >= 0.0 {
            (Side::Right, Side::Left)
        } else {
            (Side::Left, Side::Right)
        }
    } else if dy >= 0.0 {
        (Side::Bottom, Side::Top)
    } else {
        (Side::Top, Side::Bottom)
    }
}

/// One end of an edge attached to a box side.
#[derive(Clone, Copy)]
struct End {
    edge: usize,
    /// 0 = source end, 1 = target end.
    which: usize,
    key: f32,
}

fn gutter(extent: f32) -> f32 {
    (extent * 0.14).clamp(3.0, 10.0)
}

fn capacity(extent: f32) -> usize {
    let usable = (extent - 2.0 * gutter(extent)).max(0.0);
    1 + (usable / MIN_PITCH) as usize
}

fn group_ends(
    boxes: &[Rect],
    pairs: &[(usize, usize)],
    sides: &[[Side; 2]],
) -> HashMap<(usize, Side), Vec<End>> {
    let mut groups: HashMap<(usize, Side), Vec<End>> = HashMap::new();
    for (e, &(from, to)) in pairs.iter().enumerate() {
        for (which, (me, other)) in [(from, to), (to, from)].into_iter().enumerate() {
            let side = sides[e][which];
            let c = &boxes[other];
            let key = if side.is_horizontal() { c.cy() } else { c.cx() };
            groups.entry((me, side)).or_default().push(End {
                edge: e,
                which,
                key,
            });
        }
    }
    groups
}

/// Moves edges that do not fit on a crowded side onto the perpendicular side facing their
/// counterpart.
fn spill_overflow(boxes: &[Rect], pairs: &[(usize, usize)], sides: &mut [[Side; 2]]) {
    let groups = group_ends(boxes, pairs, sides);
    let mut keys: Vec<_> = groups.keys().copied().collect();
    keys.sort_by_key(|&(b, s)| (b, s as u8));
    for (b, side) in keys {
        let ends = &groups[&(b, side)];
        let r = &boxes[b];
        let cap = capacity(side.extent(r));
        let outs = ends.iter().filter(|e| e.which == 0).count();
        if ends.len() - outs + outs.min(1) <= cap {
            continue;
        }
        let mid = if side.is_horizontal() { r.cy() } else { r.cx() };
        let mut order: Vec<&End> = ends.iter().collect();
        order.sort_by(|a, c| {
            (a.key - mid)
                .abs()
                .partial_cmp(&(c.key - mid).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.edge.cmp(&c.edge))
        });
        for end in order.into_iter().skip(cap) {
            let other = if end.which == 0 {
                pairs[end.edge].1
            } else {
                pairs[end.edge].0
            };
            let o = &boxes[other];
            sides[end.edge][end.which] = if side.is_horizontal() {
                if o.cy() < r.cy() {
                    Side::Top
                } else {
                    Side::Bottom
                }
            } else if o.cx() < r.cx() {
                Side::Left
            } else {
                Side::Right
            };
        }
    }
}

/// Chooses sides for every edge and spreads the ports on crowded sides.
/// Returns `(sides, ports)` where `ports[e] = [source port, target port]`.
pub fn assign(boxes: &[Rect], pairs: &[(usize, usize)]) -> (Vec<[Side; 2]>, Vec<[Pt; 2]>) {
    let mut sides: Vec<[Side; 2]> = pairs
        .iter()
        .map(|&(f, t)| {
            let (a, b) = default_sides(&boxes[f], &boxes[t]);
            [a, b]
        })
        .collect();
    spill_overflow(boxes, pairs, &mut sides);
    let groups = group_ends(boxes, pairs, &sides);
    let mut ports = vec![[[0.0; 2]; 2]; pairs.len()];
    for ((b, side), mut ends) in groups {
        let r = &boxes[b];
        ends.sort_by(|a, c| {
            a.key
                .partial_cmp(&c.key)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.edge.cmp(&c.edge))
                .then(a.which.cmp(&c.which))
        });
        // edges leaving through the same side share one port (a bus); entering edges keep theirs
        let mut slots: Vec<(f32, Vec<End>)> = Vec::new();
        let outs: Vec<End> = ends.iter().copied().filter(|e| e.which == 0).collect();
        if !outs.is_empty() {
            let mean = outs.iter().map(|e| e.key).sum::<f32>() / outs.len() as f32;
            slots.push((mean, outs));
        }
        for e in ends.iter().filter(|e| e.which == 1) {
            slots.push((e.key, vec![*e]));
        }
        let first_edge = |s: &(f32, Vec<End>)| s.1.iter().map(|e| e.edge).min().unwrap_or(0);
        slots.sort_by(|a, c| a.0.total_cmp(&c.0).then(first_edge(a).cmp(&first_edge(c))));
        let n = slots.len();
        let extent = side.extent(r);
        let usable = (extent - 2.0 * gutter(extent)).max(0.0);
        let pitch = if n > 1 {
            (usable / (n - 1) as f32).min(MAX_PITCH)
        } else {
            0.0
        };
        for (i, (_, members)) in slots.iter().enumerate() {
            let offset = (i as f32 - (n as f32 - 1.0) * 0.5) * pitch;
            for end in members {
                ports[end.edge][end.which] = side.port(r, offset);
            }
        }
    }
    (sides, ports)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facing_sides_follow_the_separated_axis() {
        let a = Rect::new(0.0, 0.0, 100.0, 50.0);
        let b = Rect::new(200.0, 0.0, 100.0, 50.0);
        assert_eq!(default_sides(&a, &b), (Side::Right, Side::Left));
        let c = Rect::new(0.0, 200.0, 100.0, 50.0);
        assert_eq!(default_sides(&a, &c), (Side::Bottom, Side::Top));
        assert_eq!(default_sides(&c, &a), (Side::Top, Side::Bottom));
    }

    #[test]
    fn leaving_edges_share_one_bus_port_and_entering_edges_spread_symmetrically() {
        let boxes = [
            Rect::new(0.0, 100.0, 170.0, 64.0),
            Rect::new(300.0, 0.0, 170.0, 64.0),
            Rect::new(300.0, 100.0, 170.0, 64.0),
            Rect::new(300.0, 200.0, 170.0, 64.0),
        ];
        let (_, bus) = assign(&boxes, &[(0, 1), (0, 2), (0, 3)]);
        assert!(
            bus.iter().all(|p| p[0] == bus[0][0]),
            "one port for the whole bus"
        );
        assert!((bus[0][0][1] - boxes[0].cy()).abs() < 0.01);
        let (_, ports) = assign(&boxes, &[(1, 0), (2, 0), (3, 0)]);
        let ys: Vec<f32> = ports.iter().map(|p| p[1][1]).collect();
        assert!(ys[0] < ys[1] && ys[1] < ys[2]);
        assert!((ys[1] - boxes[0].cy()).abs() < 0.01);
        assert!((ys[1] - ys[0] - (ys[2] - ys[1])).abs() < 0.01);
        assert!(ys[1] - ys[0] <= MAX_PITCH + 0.01);
    }

    #[test]
    fn crowded_side_spills_to_the_perpendicular_side() {
        let hub = Rect::new(0.0, 100.0, 100.0, 40.0);
        let mut boxes = vec![hub];
        let mut pairs = Vec::new();
        for i in 0..12 {
            boxes.push(Rect::new(300.0, i as f32 * 60.0, 80.0, 30.0));
            pairs.push((i + 1, 0));
        }
        let (sides, _) = assign(&boxes, &pairs);
        let on_right = sides.iter().filter(|s| s[1] == Side::Right).count();
        assert!(on_right <= capacity(40.0));
        assert!(sides.iter().any(|s| s[1] != Side::Right));
    }
}
