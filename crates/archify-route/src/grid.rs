//! Sparse routing grid: vertical and horizontal lines through every obstacle side, through
//! the middle of every free strip between them, and through every port/stub coordinate.
//! Every orthogonal route that avoids the obstacles can be slid onto these lines, so a search
//! over their crossings is complete while staying small.

use crate::geom::Rect;

const TOL: f32 = 0.05;
/// Port and corridor lines closer than this share one line (keeps routes free of hair jogs).
const MERGE: f32 = 1.0;

pub struct Grid {
    pub xs: Vec<f32>,
    pub ys: Vec<f32>,
    /// Line lies on an obstacle side (hugging it is discouraged).
    pub x_hull: Vec<bool>,
    pub y_hull: Vec<bool>,
    pub nx: usize,
    pub ny: usize,
    /// Node strictly inside an obstacle.
    pub inside: Vec<bool>,
    /// Routes already using the unit segment (i,j)-(i+1,j).
    pub occ_h: Vec<u16>,
    /// Routes already using the unit segment (i,j)-(i,j+1).
    pub occ_v: Vec<u16>,
    /// Net (source box) that first used each unit segment; shared trunks of one net are free.
    pub owner_h: Vec<u32>,
    pub owner_v: Vec<u32>,
}

pub const NO_NET: u32 = u32::MAX;

fn lines(hull: &[f32], extra: &[f32]) -> (Vec<f32>, Vec<bool>) {
    let mut h: Vec<f32> = hull.to_vec();
    h.sort_by(f32::total_cmp);
    h.dedup_by(|a, b| (*a - *b).abs() < TOL);
    let mut all: Vec<(f32, bool)> = Vec::with_capacity(h.len() * 2 + extra.len() + 2);
    if let (Some(&lo), Some(&hi)) = (h.first(), h.last()) {
        all.push((lo - 24.0, false));
        all.push((hi + 24.0, false));
    }
    for w in h.windows(2) {
        if w[1] - w[0] > 0.5 {
            all.push(((w[0] + w[1]) * 0.5, false));
        }
    }
    all.extend(h.iter().map(|&v| (v, true)));
    all.extend(extra.iter().map(|&v| (v, false)));
    all.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut xs: Vec<f32> = Vec::with_capacity(all.len());
    let mut hulls: Vec<bool> = Vec::with_capacity(all.len());
    for (v, is_hull) in all {
        let merge = match (xs.last(), hulls.last()) {
            (Some(&last), Some(&last_hull)) => {
                let tol = if is_hull && last_hull { TOL } else { MERGE };
                (v - last).abs() < tol
            }
            _ => false,
        };
        if merge {
            if is_hull && !hulls[hulls.len() - 1] {
                let n = xs.len() - 1;
                xs[n] = v;
                hulls[n] = true;
            }
        } else {
            xs.push(v);
            hulls.push(is_hull);
        }
    }
    (xs, hulls)
}

impl Grid {
    /// `obstacles` are already inflated by the routing margin.
    pub fn build(obstacles: &[Rect], extra_x: &[f32], extra_y: &[f32]) -> Grid {
        let hx: Vec<f32> = obstacles.iter().flat_map(|r| [r.x, r.right()]).collect();
        let hy: Vec<f32> = obstacles.iter().flat_map(|r| [r.y, r.bottom()]).collect();
        let (xs, x_hull) = lines(&hx, extra_x);
        let (ys, y_hull) = lines(&hy, extra_y);
        let (nx, ny) = (xs.len(), ys.len());
        let mut inside = vec![false; nx * ny];
        for r in obstacles {
            let i0 = xs.partition_point(|&v| v <= r.x + TOL);
            let i1 = xs.partition_point(|&v| v < r.right() - TOL);
            let j0 = ys.partition_point(|&v| v <= r.y + TOL);
            let j1 = ys.partition_point(|&v| v < r.bottom() - TOL);
            for j in j0..j1 {
                for i in i0..i1 {
                    inside[j * nx + i] = true;
                }
            }
        }
        Grid {
            xs,
            ys,
            x_hull,
            y_hull,
            nx,
            ny,
            inside,
            occ_h: vec![0; nx * ny],
            occ_v: vec![0; nx * ny],
            owner_h: vec![NO_NET; nx * ny],
            owner_v: vec![NO_NET; nx * ny],
        }
    }

    /// Index of the line nearest to `v`.
    pub fn xi(&self, v: f32) -> usize {
        nearest(&self.xs, v)
    }

    pub fn yi(&self, v: f32) -> usize {
        nearest(&self.ys, v)
    }

    pub fn node(&self, i: usize, j: usize) -> usize {
        j * self.nx + i
    }

    /// Marks the unit segments between consecutive nodes of `path` as used.
    pub fn occupy(&mut self, path: &[(usize, usize)], net: u32) {
        for w in path.windows(2) {
            let ((i0, j0), (i1, j1)) = (w[0], w[1]);
            let (counts, owners, idx) = if j0 == j1 {
                (
                    &mut self.occ_h,
                    &mut self.owner_h,
                    j0 * self.nx + i0.min(i1),
                )
            } else {
                (
                    &mut self.occ_v,
                    &mut self.owner_v,
                    j0.min(j1) * self.nx + i0,
                )
            };
            if owners[idx] == NO_NET {
                owners[idx] = net;
                counts[idx] = counts[idx].saturating_add(1);
            } else if owners[idx] != net {
                counts[idx] = counts[idx].saturating_add(1);
            }
        }
    }
}

fn nearest(lines: &[f32], v: f32) -> usize {
    let p = lines.partition_point(|&x| x < v);
    if p == 0 {
        0
    } else if p == lines.len() {
        lines.len() - 1
    } else if (lines[p] - v).abs() < (v - lines[p - 1]).abs() {
        p
    } else {
        p - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_has_a_line_in_every_corridor_and_marks_box_interiors() {
        let a = Rect::new(0.0, 0.0, 100.0, 50.0).inflate(4.0);
        let b = Rect::new(200.0, 0.0, 100.0, 50.0).inflate(4.0);
        let g = Grid::build(&[a, b], &[], &[]);
        // corridor between 104 and 196 is centred at 150
        assert!(g.xs.iter().any(|&x| (x - 150.0).abs() < 0.1));
        let (i, j) = (g.xi(50.0), g.yi(25.0));
        assert!(g.inside[g.node(i, j)]);
        let (i, j) = (g.xi(150.0), g.yi(25.0));
        assert!(!g.inside[g.node(i, j)]);
    }
}
