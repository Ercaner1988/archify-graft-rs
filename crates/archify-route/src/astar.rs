//! A* over the routing grid with bend, hugging, overlap and crossing costs.

use crate::grid::Grid;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// Cost knobs (px-equivalent).
#[derive(Debug, Clone, Copy)]
pub struct Costs {
    pub bend: f32,
    /// Extra per-px factor for running along an obstacle side.
    pub hug: f32,
    /// Extra per-px factor per route already on the same unit segment.
    pub overlap: f32,
    pub crossing: f32,
    /// Heuristic weight (>1 trades optimality for speed).
    pub weight: f32,
}

impl Default for Costs {
    fn default() -> Self {
        Costs {
            bend: 22.0,
            hug: 0.35,
            overlap: 1.6,
            crossing: 28.0,
            weight: 1.4,
        }
    }
}

/// Direction indices: 0 = +x, 1 = +y, 2 = -x, 3 = -y.
const DX: [isize; 4] = [1, 0, -1, 0];
const DY: [isize; 4] = [0, 1, 0, -1];

struct Item {
    f: f32,
    state: u32,
}

impl PartialEq for Item {
    fn eq(&self, o: &Self) -> bool {
        self.f == o.f
    }
}
impl Eq for Item {}
impl PartialOrd for Item {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Item {
    fn cmp(&self, o: &Self) -> Ordering {
        // min-heap on f
        o.f.total_cmp(&self.f)
    }
}

/// Reusable search buffers.
pub struct Searcher {
    g: Vec<f32>,
    stamp: Vec<u32>,
    prev: Vec<u32>,
    generation: u32,
    heap: BinaryHeap<Item>,
    pub costs: Costs,
    pub max_pops: usize,
    /// Net of the route being searched (see `Grid::owner_h`).
    pub net: u32,
}

const NONE: u32 = u32::MAX;
const TERMINAL: u32 = 1 << 31;

impl Searcher {
    pub fn new(nodes: usize, costs: Costs) -> Searcher {
        Searcher {
            g: vec![0.0; nodes * 4],
            stamp: vec![0; nodes * 4],
            prev: vec![NONE; nodes * 4],
            generation: 0,
            heap: BinaryHeap::new(),
            costs,
            max_pops: 6_000_000,
            net: crate::grid::NO_NET,
        }
    }

    /// Finds the cheapest path from node `s` (leaving in direction `sdir`, never reversing) to
    /// node `t` (entering the target box in direction `tdir` afterwards). Returns the node
    /// sequence `(i, j)`.
    pub fn search(
        &mut self,
        grid: &Grid,
        s: (usize, usize),
        sdir: usize,
        t: (usize, usize),
        tdir: usize,
    ) -> Option<Vec<(usize, usize)>> {
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.stamp.fill(0);
            self.generation = 1;
        }
        self.heap.clear();
        let nx = grid.nx;
        let (tx, ty) = (grid.xs[t.0], grid.ys[t.1]);
        let w = self.costs.weight;
        let bend = self.costs.bend;
        let h = |i: usize, j: usize| {
            let (dx, dy) = ((grid.xs[i] - tx).abs(), (grid.ys[j] - ty).abs());
            w * (dx + dy + if dx > 0.5 && dy > 0.5 { bend } else { 0.0 })
        };
        let start = ((s.1 * nx + s.0) * 4 + sdir) as u32;
        self.set(start as usize, 0.0, NONE);
        self.heap.push(Item {
            f: h(s.0, s.1),
            state: start,
        });
        let mut pops = 0usize;
        while let Some(Item { f: _, state }) = self.heap.pop() {
            pops += 1;
            if pops > self.max_pops {
                return None;
            }
            if state & TERMINAL != 0 {
                return Some(self.rebuild(grid, state & !TERMINAL));
            }
            let st = state as usize;
            let (node, dir) = (st / 4, st % 4);
            let (i, j) = (node % nx, node / nx);
            let g = self.g[st];
            if (i, j) == t {
                if dir != (tdir + 2) % 4 {
                    let end = g + if dir == tdir { 0.0 } else { self.costs.bend };
                    self.heap.push(Item {
                        f: end,
                        state: state | TERMINAL,
                    });
                }
                continue;
            }
            for nd in 0..4 {
                if nd == (dir + 2) % 4 {
                    continue;
                }
                let (ni, nj) = (i as isize + DX[nd], j as isize + DY[nd]);
                if ni < 0 || nj < 0 || ni as usize >= nx || nj as usize >= grid.ny {
                    continue;
                }
                let (ni, nj) = (ni as usize, nj as usize);
                let nnode = nj * nx + ni;
                if grid.inside[nnode] {
                    continue;
                }
                let step = self.step_cost(grid, (i, j), (ni, nj), nd, dir);
                let ng = g + step;
                let ns = nnode * 4 + nd;
                if self.stamp[ns] == self.generation && self.g[ns] <= ng {
                    continue;
                }
                self.set(ns, ng, state);
                self.heap.push(Item {
                    f: ng + h(ni, nj),
                    state: ns as u32,
                });
            }
        }
        None
    }

    fn set(&mut self, state: usize, g: f32, prev: u32) {
        self.g[state] = g;
        self.stamp[state] = self.generation;
        self.prev[state] = prev;
    }

    fn step_cost(
        &self,
        grid: &Grid,
        (i, j): (usize, usize),
        (ni, nj): (usize, usize),
        nd: usize,
        dir: usize,
    ) -> f32 {
        let nx = grid.nx;
        let horizontal = nd.is_multiple_of(2);
        let len = if horizontal {
            (grid.xs[ni] - grid.xs[i]).abs()
        } else {
            (grid.ys[nj] - grid.ys[j]).abs()
        };
        let hull = if horizontal {
            grid.y_hull[j]
        } else {
            grid.x_hull[i]
        };
        let (si, sj) = (i.min(ni), j.min(nj));
        let idx = sj * nx + si;
        let (count, owner) = if horizontal {
            (grid.occ_h[idx], grid.owner_h[idx])
        } else {
            (grid.occ_v[idx], grid.owner_v[idx])
        };
        let occ = if owner == self.net { 0.0 } else { count as f32 };
        let mut cost =
            len * (1.0 + if hull { self.costs.hug } else { 0.0 } + occ * self.costs.overlap);
        if nd != dir {
            cost += self.costs.bend;
        } else {
            // passing straight through the node: crossing a route that does the same
            let crossed = if horizontal {
                j > 0 && grid.occ_v[(j - 1) * nx + i] > 0 && grid.occ_v[j * nx + i] > 0
            } else {
                i > 0 && grid.occ_h[j * nx + i - 1] > 0 && grid.occ_h[j * nx + i] > 0
            };
            if crossed {
                cost += self.costs.crossing;
            }
        }
        cost
    }

    fn rebuild(&self, grid: &Grid, end_state: u32) -> Vec<(usize, usize)> {
        let mut path = Vec::new();
        let mut cur = end_state;
        while cur != NONE {
            let node = cur as usize / 4;
            path.push((node % grid.nx, node / grid.nx));
            cur = self.prev[cur as usize];
        }
        path.reverse();
        path
    }
}

/// Direction index of a unit vector.
pub fn dir_index(v: [f32; 2]) -> usize {
    if v[0] > 0.5 {
        0
    } else if v[1] > 0.5 {
        1
    } else if v[0] < -0.5 {
        2
    } else {
        3
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Rect;

    #[test]
    fn routes_around_a_wall() {
        let wall = Rect::new(100.0, -100.0, 20.0, 200.0).inflate(4.0);
        let grid = Grid::build(&[wall], &[0.0, 300.0], &[0.0]);
        let mut s = Searcher::new(grid.nx * grid.ny, Costs::default());
        let (si, sj) = (grid.xi(0.0), grid.yi(0.0));
        let (ti, tj) = (grid.xi(300.0), grid.yi(0.0));
        let path = s.search(&grid, (si, sj), 0, (ti, tj), 0).expect("path");
        assert!(path.len() >= 4, "must detour: {path:?}");
        for &(i, j) in &path {
            assert!(!grid.inside[grid.node(i, j)]);
        }
    }
}
