//! Post-processing of routed polylines: tiny jogs are straightened, and routes that share a
//! corridor are fanned out into parallel lanes ordered so that they cross as little as possible.

use crate::geom::{Pt, Rect};
use crate::poly::simplify;

/// Lane pitches tried from widest to tightest.
const PITCHES: [f32; 5] = [8.0, 6.0, 5.0, 4.0, 3.0];
/// Minimal clearance kept to every obstacle while moving segments.
const CLEAR: f32 = 1.5;
const MIN_STUB: f32 = 3.0;
/// Perpendicular jogs shorter than this are collapsed.
const JOG: f32 = 10.0;
/// Segments whose lines are closer than this compete for the same corridor.
const CORRIDOR: f32 = 6.0;
/// How far a segment may slide while looking for room (px each way).
const MAX_SLIDE: i32 = 40;

#[derive(Clone, Copy)]
struct Seg {
    route: usize,
    net: usize,
    k: usize,
    coord: f32,
    lo: f32,
    hi: f32,
}

/// Straightens jogs, then fans out overlapping segments. Horizontal segments move
/// vertically first, then vertical segments move horizontally.
pub fn nudge(routes: &mut [Vec<Pt>], obstacles: &[Rect], nets: &[usize]) {
    for r in routes.iter_mut() {
        remove_jogs(r, obstacles);
    }
    for vertical in [false, true] {
        pass(routes, obstacles, nets, vertical);
        for r in routes.iter_mut() {
            *r = simplify(r);
        }
    }
}

fn is_vertical(a: Pt, b: Pt) -> bool {
    (a[0] - b[0]).abs() < 0.01 && (a[1] - b[1]).abs() >= 0.01
}

fn is_horizontal(a: Pt, b: Pt) -> bool {
    (a[1] - b[1]).abs() < 0.01 && (a[0] - b[0]).abs() >= 0.01
}

fn seg_len(a: Pt, b: Pt) -> f32 {
    (a[0] - b[0]).abs() + (a[1] - b[1]).abs()
}

/// Collapses `A - short B - C` staircases where A and C are parallel.
fn remove_jogs(route: &mut Vec<Pt>, obstacles: &[Rect]) {
    *route = simplify(route);
    for _ in 0..8 {
        let mut changed = false;
        let n = route.len();
        let mut i = 0;
        while i + 3 < n {
            let (p0, p1, p2, p3) = (route[i], route[i + 1], route[i + 2], route[i + 3]);
            let horizontal_ac = is_horizontal(p0, p1) && is_horizontal(p2, p3);
            let vertical_ac = is_vertical(p0, p1) && is_vertical(p2, p3);
            if (horizontal_ac || vertical_ac) && seg_len(p1, p2) < JOG {
                // axis of the coordinate that differs between A and C
                let axis = usize::from(horizontal_ac);
                let (len_a, len_c) = (seg_len(p0, p1), seg_len(p2, p3));
                let try_a = i >= 1;
                let try_c = i + 4 < n;
                let order: [bool; 2] = if len_a <= len_c {
                    [true, false]
                } else {
                    [false, true]
                };
                for move_a in order {
                    let ok = if move_a {
                        try_a && can_move(route, i, axis, p2[axis], obstacles)
                    } else {
                        try_c && can_move(route, i + 2, axis, p1[axis], obstacles)
                    };
                    if ok {
                        let (k, to) = if move_a {
                            (i, p2[axis])
                        } else {
                            (i + 2, p1[axis])
                        };
                        route[k][axis] = to;
                        route[k + 1][axis] = to;
                        changed = true;
                        break;
                    }
                }
                if changed {
                    break;
                }
            }
            i += 1;
        }
        *route = simplify(route);
        if !changed {
            break;
        }
    }
}

fn pass(routes: &mut [Vec<Pt>], obstacles: &[Rect], nets: &[usize], vertical: bool) {
    let axis = usize::from(!vertical); // coordinate that moves
    let other = 1 - axis;
    let mut segs: Vec<Seg> = Vec::new();
    for (r, pts) in routes.iter().enumerate() {
        if pts.len() < 4 {
            continue;
        }
        for k in 1..pts.len() - 2 {
            let (a, b) = (pts[k], pts[k + 1]);
            let ok = if vertical {
                is_vertical(a, b)
            } else {
                is_horizontal(a, b)
            };
            if ok {
                segs.push(Seg {
                    route: r,
                    net: nets[r],
                    k,
                    coord: a[axis],
                    lo: a[other].min(b[other]),
                    hi: a[other].max(b[other]),
                });
            }
        }
    }
    segs.sort_by(|a, b| a.coord.total_cmp(&b.coord).then(a.lo.total_cmp(&b.lo)));
    let mut i = 0;
    while i < segs.len() {
        let mut j = i + 1;
        while j < segs.len()
            && segs[j].coord - segs[j - 1].coord <= CORRIDOR * 0.5
            && segs[j].coord - segs[i].coord <= CORRIDOR
        {
            j += 1;
        }
        let mut cluster = segs[i..j].to_vec();
        cluster.sort_by(|a, b| a.lo.total_cmp(&b.lo));
        for comp in components(&cluster) {
            if comp.iter().any(|s| s.net != comp[0].net) {
                fan_out(routes, obstacles, vertical, comp);
            }
        }
        i = j;
    }
}

/// Connected components (by interval overlap) of segments in one corridor.
fn components(sorted_by_lo: &[Seg]) -> Vec<Vec<Seg>> {
    let mut out: Vec<Vec<Seg>> = Vec::new();
    let mut hi = f32::NEG_INFINITY;
    for s in sorted_by_lo {
        match out.last_mut() {
            Some(last) if s.lo < hi - 1.0 => {
                last.push(*s);
                hi = hi.max(s.hi);
            }
            _ => {
                out.push(vec![*s]);
                hi = s.hi;
            }
        }
    }
    out
}

/// Sign of the direction the route leaves the segment at its two ends (-1 towards smaller
/// coordinate, +1 towards larger); the lane order follows it.
fn lane_key(routes: &[Vec<Pt>], s: &Seg, axis: usize) -> f32 {
    let pts = &routes[s.route];
    let sign = |from: Pt, to: Pt| (to[axis] - from[axis]).signum();
    sign(pts[s.k], pts[s.k - 1]) + sign(pts[s.k + 1], pts[s.k + 2])
}

/// Feasible slide interval `[lo, hi]` (px, relative) around the current position.
fn slide_range(pts: &[Pt], s: &Seg, axis: usize, obstacles: &[Rect]) -> Option<(f32, f32)> {
    if !can_move(pts, s.k, axis, s.coord, obstacles) {
        return None;
    }
    let reach = |dir: i32| {
        let mut last = 0;
        for d in 1..=MAX_SLIDE {
            if can_move(pts, s.k, axis, s.coord + (dir * d) as f32, obstacles) {
                last = d;
            } else {
                break;
            }
        }
        (dir * last) as f32
    };
    Some((reach(-1), reach(1)))
}

fn fan_out(routes: &mut [Vec<Pt>], obstacles: &[Rect], vertical: bool, mut comp: Vec<Seg>) {
    let axis = usize::from(!vertical);
    comp.sort_by(|a, b| {
        lane_key(routes, a, axis)
            .total_cmp(&lane_key(routes, b, axis))
            .then(a.coord.total_cmp(&b.coord))
            .then(a.route.cmp(&b.route))
    });
    // absolute interval every member can reach
    let (mut lo, mut hi) = (f32::NEG_INFINITY, f32::INFINITY);
    for s in &comp {
        let Some((l, h)) = slide_range(&routes[s.route], s, axis, obstacles) else {
            return;
        };
        lo = lo.max(s.coord + l);
        hi = hi.min(s.coord + h);
    }
    if hi < lo {
        return;
    }
    // one lane per net: edges leaving the same box share their trunk
    let mut lane_nets: Vec<usize> = Vec::new();
    for s in &comp {
        if !lane_nets.contains(&s.net) {
            lane_nets.push(s.net);
        }
    }
    let lane_of = |s: &Seg| lane_nets.iter().position(|&n| n == s.net).unwrap_or(0);
    let n = lane_nets.len() as f32;
    let mean = comp.iter().map(|s| s.coord).sum::<f32>() / comp.len() as f32;
    for pitch in PITCHES {
        let span = (n - 1.0) * pitch;
        if span > hi - lo + 0.01 {
            continue;
        }
        let centre = mean.clamp(lo + span * 0.5, hi - span * 0.5);
        let targets: Vec<f32> = (0..lane_nets.len())
            .map(|i| centre + (i as f32 - (n - 1.0) * 0.5) * pitch)
            .collect();
        for s in &comp {
            let t = targets[lane_of(s)];
            let pts = &mut routes[s.route];
            pts[s.k][axis] = t;
            pts[s.k + 1][axis] = t;
        }
        return;
    }
}

fn can_move(pts: &[Pt], k: usize, axis: usize, to: f32, obstacles: &[Rect]) -> bool {
    let last = pts.len() - 1;
    if k == 0 || k + 2 > last {
        return false;
    }
    let mut a = pts[k];
    let mut b = pts[k + 1];
    a[axis] = to;
    b[axis] = to;
    let hit = |p: Pt, q: Pt| {
        obstacles
            .iter()
            .any(|o| o.inflate(CLEAR).hit_by_segment(p, q))
    };
    if hit(a, b) {
        return false;
    }
    // segment before: pts[k-1] -> a
    if k - 1 == 0 {
        if !stub_ok(pts[0], pts[k], a) {
            return false;
        }
    } else if hit(pts[k - 1], a) {
        return false;
    }
    // segment after: b -> pts[k+2]
    if k + 2 == last {
        if !stub_ok(pts[last], pts[k + 1], b) {
            return false;
        }
    } else if hit(b, pts[k + 2]) {
        return false;
    }
    true
}

/// A port stub must keep its direction and a minimal length when its free end moves.
fn stub_ok(port: Pt, old_end: Pt, new_end: Pt) -> bool {
    let d_old = [old_end[0] - port[0], old_end[1] - port[1]];
    let d_new = [new_end[0] - port[0], new_end[1] - port[1]];
    let along = d_old[0] * d_new[0] + d_old[1] * d_new[1];
    let len = (d_new[0] * d_new[0] + d_new[1] * d_new[1]).sqrt();
    along > 0.0 && len >= MIN_STUB
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poly::overlap_length;

    #[test]
    fn two_overlapping_routes_get_separate_lanes() {
        let a = vec![
            [0.0, 0.0],
            [0.0, 20.0],
            [0.0, 100.0],
            [200.0, 100.0],
            [200.0, 180.0],
            [200.0, 200.0],
        ];
        let b = vec![
            [50.0, 0.0],
            [50.0, 20.0],
            [50.0, 100.0],
            [250.0, 100.0],
            [250.0, 180.0],
            [250.0, 200.0],
        ];
        let mut routes = vec![simplify(&a), simplify(&b)];
        assert!(overlap_length(&routes[0], &routes[1]) > 100.0);
        nudge(&mut routes, &[], &[0, 1]);
        assert!(overlap_length(&routes[0], &routes[1]) < 1.0);
        // endpoints (ports) did not move
        assert_eq!(routes[0][0], [0.0, 0.0]);
        assert_eq!(*routes[1].last().unwrap(), [250.0, 200.0]);
    }

    #[test]
    fn lanes_squeeze_to_one_side_when_the_other_is_walled() {
        // wall right above y=100: lanes must go downwards
        let a = vec![[0.0, 0.0], [0.0, 100.0], [200.0, 100.0], [200.0, 300.0]];
        let b = vec![[30.0, 0.0], [30.0, 100.0], [230.0, 100.0], [230.0, 300.0]];
        let wall = Rect::new(40.0, 80.0, 100.0, 10.0);
        let mut routes = vec![a, b];
        nudge(&mut routes, &[wall], &[0, 1]);
        assert!(overlap_length(&routes[0], &routes[1]) < 1.0);
        for r in &routes {
            for w in r.windows(2) {
                assert!(!wall.hit_by_segment(w[0], w[1]));
            }
        }
    }

    #[test]
    fn small_jogs_are_collapsed() {
        let mut r = vec![
            [0.0, 0.0],
            [0.0, 50.0],
            [100.0, 50.0],
            [100.0, 54.0],
            [200.0, 54.0],
            [200.0, 120.0],
        ];
        remove_jogs(&mut r, &[]);
        assert_eq!(r.len(), 4, "{r:?}");
        assert_eq!(r[1][1], r[2][1]);
    }
}
