//! archify-route: orthogonal edge routing for box diagrams.
//!
//! Given boxes, keep-out rectangles (region titles) and edge requests, [`route_all`] returns
//! for every edge an axis-aligned polyline that leaves the source through the middle of a
//! side, never passes through a box, keeps a margin to the boxes, fans out when several edges
//! share a side or a corridor, and carries a label plate and an arrowhead position.
//!
//! Pipeline: side choice and port spreading ([`sides`]) -> sparse grid ([`grid`]) -> A* with
//! bend/overlap/crossing costs ([`astar`]) -> lane separation ([`nudge`]) -> label plates
//! ([`label`]). Rounded corners are a drawing concern: see [`poly::rounded_path`].

pub mod astar;
pub mod geom;
pub mod grid;
pub mod label;
pub mod nudge;
pub mod poly;
pub mod sides;

pub use astar::Costs;
pub use geom::{Pt, Rect, Side};
pub use poly::{flatten_rounded, rounded_path, PathCmd};

use astar::{dir_index, Searcher};
use grid::Grid;

/// Arrowhead length and half-width (the original 10x7 marker).
pub const ARROW_LEN: f32 = 10.0;
pub const ARROW_HALF_W: f32 = 3.5;
/// Corner radius of the original routes.
pub const CORNER_RADIUS: f32 = 8.0;

#[derive(Debug, Clone, Copy)]
pub struct EdgeRequest {
    pub from: usize,
    pub to: usize,
    /// Width of the label plate, `None` for an unlabeled edge.
    pub label_width: Option<f32>,
}

#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// Clearance kept to every box while searching.
    pub margin: f32,
    /// Longest straight piece leaving a port before the first bend is allowed.
    pub max_stub: f32,
    pub costs: Costs,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            margin: 4.0,
            max_stub: 12.0,
            costs: Costs::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arrow {
    /// Point on the target box side.
    pub tip: Pt,
    /// Unit vector of travel at the tip.
    pub dir: Pt,
}

#[derive(Debug, Clone)]
pub struct RoutedEdge {
    /// Orthogonal polyline from the source port to the target port.
    pub points: Vec<Pt>,
    pub label: Option<Rect>,
    pub arrow: Arrow,
    pub sides: [Side; 2],
    /// False when the search failed and a blind fallback path is returned.
    pub clear: bool,
}

/// Routes every edge. `boxes` are obstacles and edge endpoints; `keepouts` are obstacles only.
pub fn route_all(
    boxes: &[Rect],
    keepouts: &[Rect],
    edges: &[EdgeRequest],
    opts: &Options,
) -> Vec<RoutedEdge> {
    let pairs: Vec<(usize, usize)> = edges.iter().map(|e| (e.from, e.to)).collect();
    let (sides, mut ports) = sides::assign(boxes, &pairs);
    let mut stubs: Vec<[Pt; 2]> = Vec::with_capacity(edges.len());
    for (e, &(f, t)) in pairs.iter().enumerate() {
        stubs.push([
            stub_point(boxes, f, sides[e][0], ports[e][0], opts.max_stub),
            stub_point(boxes, t, sides[e][1], ports[e][1], opts.max_stub),
        ]);
    }
    let mut obstacles: Vec<Rect> = boxes.iter().map(|b| b.inflate(opts.margin)).collect();
    obstacles.extend(keepouts.iter().map(|k| k.inflate(2.0)));
    let extra_x: Vec<f32> = stubs.iter().flatten().map(|p| p[0]).collect();
    let extra_y: Vec<f32> = stubs.iter().flatten().map(|p| p[1]).collect();
    let mut grid = Grid::build(&obstacles, &extra_x, &extra_y);
    // nearby port lines were merged: move stubs and ports onto the merged lines
    for e in 0..edges.len() {
        for w in 0..2 {
            let s = stubs[e][w];
            let snapped = [grid.xs[grid.xi(s[0])], grid.ys[grid.yi(s[1])]];
            stubs[e][w] = snapped;
            let axis = usize::from(sides[e][w].is_horizontal());
            ports[e][w][axis] = snapped[axis];
        }
    }
    let mut searcher = Searcher::new(grid.nx * grid.ny, opts.costs);

    let mut order: Vec<usize> = (0..edges.len()).collect();
    order.sort_by(|&a, &b| {
        let len = |e: usize| {
            (stubs[e][0][0] - stubs[e][1][0]).abs() + (stubs[e][0][1] - stubs[e][1][1]).abs()
        };
        len(a).total_cmp(&len(b)).then(a.cmp(&b))
    });

    let mut routes: Vec<Vec<Pt>> = vec![Vec::new(); edges.len()];
    let mut clear = vec![true; edges.len()];
    for e in order {
        let (f, t) = pairs[e];
        if f == t {
            routes[e] = self_loop(&boxes[f]);
            continue;
        }
        let [s, g] = stubs[e];
        let sdir = dir_index(sides[e][0].outward());
        let tdir = dir_index(sides[e][1].opposite().outward());
        let start = (grid.xi(s[0]), grid.yi(s[1]));
        let goal = (grid.xi(g[0]), grid.yi(g[1]));
        searcher.net = f as u32;
        let found = searcher.search(&grid, start, sdir, goal, tdir);
        let mut pts = vec![ports[e][0]];
        match found {
            Some(path) => {
                pts.extend(path.iter().map(|&(i, j)| [grid.xs[i], grid.ys[j]]));
                grid.occupy(&path, f as u32);
            }
            None => {
                clear[e] = false;
                pts.extend(blind_path(s, g, sides[e][0]));
            }
        }
        pts.push(ports[e][1]);
        poly::snap_axes(&mut pts);
        routes[e] = poly::simplify(&pts);
    }

    let mut solid: Vec<Rect> = boxes.to_vec();
    solid.extend_from_slice(keepouts);
    let nets: Vec<usize> = pairs.iter().map(|p| p.0).collect();
    nudge::nudge(&mut routes, &solid, &nets);

    let mut blockers: Vec<Rect> = boxes.iter().map(|b| b.inflate(2.0)).collect();
    blockers.extend_from_slice(keepouts);
    let mut placed: Vec<Rect> = Vec::new();
    let mut labels: Vec<Option<Rect>> = vec![None; edges.len()];
    for (e, req) in edges.iter().enumerate() {
        if let Some(w) = req.label_width {
            let r = label::place(&routes[e], w, e, &routes, &nets, &blockers, &placed)
                .unwrap_or_else(|| label::fallback(&routes[e], w));
            placed.push(r);
            labels[e] = Some(r);
        }
    }

    routes
        .into_iter()
        .zip(labels)
        .enumerate()
        .map(|(e, (points, label))| {
            let arrow = arrow_of(&points, sides[e][1]);
            RoutedEdge {
                points,
                label,
                arrow,
                sides: sides[e],
                clear: clear[e],
            }
        })
        .collect()
}

fn arrow_of(points: &[Pt], target_side: Side) -> Arrow {
    let n = points.len();
    if n < 2 {
        return Arrow {
            tip: points.first().copied().unwrap_or([0.0, 0.0]),
            dir: target_side.opposite().outward(),
        };
    }
    let (a, b) = (points[n - 2], points[n - 1]);
    let l = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2))
        .sqrt()
        .max(1e-6);
    Arrow {
        tip: b,
        dir: [(b[0] - a[0]) / l, (b[1] - a[1]) / l],
    }
}

/// Point `stub` px out of the port, never closer than halfway to a box in the way.
fn stub_point(boxes: &[Rect], own: usize, side: Side, port: Pt, max_stub: f32) -> Pt {
    let out = side.outward();
    let mut free = f32::INFINITY;
    for (i, b) in boxes.iter().enumerate() {
        if i == own {
            continue;
        }
        let d = if side.is_horizontal() {
            if port[1] <= b.y || port[1] >= b.bottom() {
                continue;
            }
            if out[0] > 0.0 {
                b.x - port[0]
            } else {
                port[0] - b.right()
            }
        } else {
            if port[0] <= b.x || port[0] >= b.right() {
                continue;
            }
            if out[1] > 0.0 {
                b.y - port[1]
            } else {
                port[1] - b.bottom()
            }
        };
        if d >= 0.0 {
            free = free.min(d);
        }
    }
    let len = (free * 0.5).clamp(3.0, max_stub);
    [port[0] + out[0] * len, port[1] + out[1] * len]
}

fn self_loop(r: &Rect) -> Vec<Pt> {
    let (x, y) = (r.right(), r.cy());
    vec![
        [x, y - 8.0],
        [x + 18.0, y - 8.0],
        [x + 18.0, y + 8.0],
        [x, y + 8.0],
    ]
}

/// Elbow path ignoring obstacles; continues straight out of the first stub, then turns.
fn blind_path(s: Pt, g: Pt, side: Side) -> Vec<Pt> {
    let elbow = if side.is_horizontal() {
        [g[0], s[1]]
    } else {
        [s[0], g[1]]
    };
    vec![s, elbow, g]
}

#[cfg(test)]
mod tests;
