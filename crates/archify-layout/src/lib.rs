//! Layered (Sugiyama-style) placement of grouped boxes. Dependencies point left to
//! right: a box that uses another sits in an earlier column. Groups (repositories,
//! workspaces) are rectangles that never overlap; inside a group the boxes are layered
//! again by their own links, and each column is ordered by where its neighbours are.

use std::collections::HashMap;

pub const NODE_W: f32 = 176.0;
pub const NODE_H: f32 = 64.0;
const COL_GAP: f32 = 100.0;
const ROW_GAP: f32 = 32.0;
const PAD_X: f32 = 28.0;
const PAD_Y: f32 = 22.0;
const HEADER: f32 = 40.0;
const GROUP_GAP: f32 = 30.0;
const BAND_GAP: f32 = 130.0;
pub const LEFT: f32 = 40.0;
pub const TOP: f32 = 110.0;

pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub weight: u32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

pub struct Placed {
    /// Top-left corner of each box.
    pub pos: Vec<(f32, f32)>,
    /// Rectangle of each group (zero-sized when the group has no box).
    pub groups: Vec<Rect>,
}

/// Longest-path layer of every node; links that close a cycle are ignored.
pub fn layers(n: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut indeg = vec![0usize; n];
    for &(a, b) in edges {
        if a != b && !out[a].contains(&b) {
            out[a].push(b);
            indeg[b] += 1;
        }
    }
    // Depth-first from the sources first; an edge into a node still on the stack closes
    // a cycle and is dropped.
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| (indeg[i], i));
    let mut state = vec![0u8; n]; // 0 new, 1 on stack, 2 done
    let mut back: Vec<(usize, usize)> = Vec::new();
    for &root in &order {
        if state[root] != 0 {
            continue;
        }
        let mut stack = vec![(root, 0usize)];
        state[root] = 1;
        while let Some(&mut (v, ref mut i)) = stack.last_mut() {
            if *i < out[v].len() {
                let w = out[v][*i];
                *i += 1;
                match state[w] {
                    0 => {
                        state[w] = 1;
                        stack.push((w, 0));
                    }
                    1 => back.push((v, w)),
                    _ => {}
                }
            } else {
                state[v] = 2;
                stack.pop();
            }
        }
    }
    let mut indeg = vec![0usize; n];
    for (v, targets) in out.iter_mut().enumerate() {
        targets.retain(|w| !back.contains(&(v, *w)));
        for &w in targets.iter() {
            indeg[w] += 1;
        }
    }
    let mut layer = vec![0usize; n];
    let mut ready: Vec<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();
    while let Some(v) = ready.pop() {
        for &w in &out[v] {
            layer[w] = layer[w].max(layer[v] + 1);
            indeg[w] -= 1;
            if indeg[w] == 0 {
                ready.push(w);
            }
        }
    }
    layer
}

/// Width / height the drawing aims for (a landscape window).
pub const TARGET_RATIO: f32 = 1.6;
pub const MIN_RATIO: f32 = 1.3;
pub const MAX_RATIO: f32 = 1.8;
/// Gap between two columns of groups that were split off one tall band.
const SPLIT_GAP: f32 = 90.0;
/// Column counts tried for deep groups; the best aspect ratio wins.
const COLUMN_CAPS: [usize; 6] = [2, 3, 4, 5, 6, usize::MAX];

/// Lower is better: any ratio inside [`MIN_RATIO`, `MAX_RATIO`] beats every other, and
/// among those the smaller area wins; outside the range the closer ratio wins.
fn score(w: f32, h: f32) -> f32 {
    let ratio = w / h;
    if (MIN_RATIO..=MAX_RATIO).contains(&ratio) {
        w * h
    } else {
        1.0e9 * (1.0 + (ratio / TARGET_RATIO).ln().abs())
    }
}

/// Bounding box `(width, height)` of the placed groups.
pub fn extent(p: &Placed) -> (f32, f32) {
    let w = p.groups.iter().map(|r| r.x + r.w).fold(0.0f32, f32::max) - LEFT;
    let h = p.groups.iter().map(|r| r.y + r.h).fold(0.0f32, f32::max) - TOP;
    (w.max(1.0), h.max(1.0))
}

/// `group[i]` is the group of box `i`; `n_groups` counts the groups. Tries a few
/// column caps and keeps the layout whose width/height is closest to [`TARGET_RATIO`].
pub fn place(group: &[usize], edges: &[Edge], n_groups: usize) -> Placed {
    let mut best: Option<(f32, Placed)> = None;
    for (cap, grid) in COLUMN_CAPS.into_iter().flat_map(|c| [(c, 2), (c, 3)]) {
        let p = place_with(group, edges, n_groups, cap, grid);
        let (w, h) = extent(&p);
        let s = score(w, h);
        if best.as_ref().is_none_or(|b| s < b.0) {
            best = Some((s, p));
        }
    }
    best.map(|b| b.1).unwrap_or(Placed {
        pos: Vec::new(),
        groups: Vec::new(),
    })
}

fn place_with(group: &[usize], edges: &[Edge], n_groups: usize, cap: usize, grid: usize) -> Placed {
    let n = group.len();
    // Group level: which band (column of groups) each group sits in.
    let mut gw: HashMap<(usize, usize), u32> = HashMap::new();
    for e in edges {
        let (a, b) = (group[e.from], group[e.to]);
        if a != b {
            *gw.entry((a, b)).or_default() += e.weight.max(1);
        }
    }
    let mut gedges: Vec<(usize, usize)> = gw.keys().copied().collect();
    gedges.sort_unstable();
    let band = layers(n_groups, &gedges);
    // Box level: layer inside the group from the group's own links; deep groups wrap
    // into `cap` columns, link-less groups become a small grid.
    let intra: Vec<(usize, usize)> = edges
        .iter()
        .filter(|e| group[e.from] == group[e.to])
        .map(|e| (e.from, e.to))
        .collect();
    let layer = layers(n, &intra);
    let mut count = vec![0usize; n_groups];
    let mut linked = vec![false; n_groups];
    for i in 0..n {
        count[group[i]] += 1;
    }
    for &(a, _) in &intra {
        linked[group[a]] = true;
    }
    let mut seen = vec![0usize; n_groups];
    let mut col = vec![0usize; n];
    let mut block = vec![0usize; n];
    for i in 0..n {
        let g = group[i];
        if !linked[g] && count[g] >= 3 {
            let cols = (count[g] as f32).sqrt().ceil() as usize;
            let cols = cols.clamp(2, grid);
            let rows = count[g].div_ceil(cols);
            col[i] = seen[g] / rows;
            seen[g] += 1;
        } else {
            // Small groups keep strict left-to-right dependency order (3 columns at least).
            let cap = if count[g] >= 6 { cap } else { cap.max(3) };
            col[i] = layer[i] % cap;
            block[i] = layer[i] / cap;
        }
    }

    let mut members: Vec<Vec<Vec<usize>>> = vec![Vec::new(); n_groups];
    for i in 0..n {
        let g = group[i];
        if members[g].len() <= col[i] {
            members[g].resize(col[i] + 1, Vec::new());
        }
        members[g][col[i]].push(i);
    }
    let size = |g: usize| -> (f32, f32) {
        let cols = members[g].len();
        if cols == 0 {
            return (0.0, 0.0);
        }
        let rows = members[g].iter().map(Vec::len).max().unwrap_or(1) as f32;
        (
            2.0 * PAD_X + cols as f32 * NODE_W + (cols as f32 - 1.0) * COL_GAP,
            HEADER + PAD_Y + rows * NODE_H + (rows - 1.0) * ROW_GAP + PAD_Y,
        )
    };

    // Bands left to right; the busiest groups sit on top of their band. A band that
    // makes the drawing too tall is split into side-by-side columns.
    let n_bands = band.iter().copied().max().map_or(0, |m| m + 1);
    let degree = |g: usize| -> u32 {
        gw.iter()
            .filter(|((a, b), _)| *a == g || *b == g)
            .map(|(_, w)| *w)
            .sum()
    };
    let mut bands: Vec<Vec<usize>> = vec![Vec::new(); n_bands];
    for g in 0..n_groups {
        if !members[g].is_empty() {
            bands[band[g]].push(g);
        }
    }
    bands.retain(|b| !b.is_empty());
    for b in &mut bands {
        b.sort_by_key(|&g| (std::cmp::Reverse(degree(g)), g));
    }
    // `split_next[i]`: column i was split off the column before it.
    let mut split_next = vec![false; bands.len()];
    let height = |b: &[usize]| -> f32 {
        b.iter().map(|&g| size(g).1).sum::<f32>() + GROUP_GAP * b.len().saturating_sub(1) as f32
    };
    let width = |b: &[usize]| -> f32 { b.iter().map(|&g| size(g).0).fold(0.0f32, f32::max) };
    let mut best_split: Option<(f32, Vec<Vec<usize>>, Vec<bool>)> = None;
    for _ in 0..=8 {
        let total_w: f32 = bands
            .iter()
            .enumerate()
            .map(|(i, b)| {
                width(b)
                    + if i == 0 {
                        0.0
                    } else if split_next[i] {
                        SPLIT_GAP
                    } else {
                        BAND_GAP
                    }
            })
            .sum();
        let (tall_ix, tall_h) = bands
            .iter()
            .enumerate()
            .map(|(i, b)| (i, height(b)))
            .fold((0, 0.0f32), |a, b| if b.1 > a.1 { b } else { a });
        let miss = score(total_w, tall_h.max(1.0));
        if best_split.as_ref().is_none_or(|b| miss < b.0) {
            best_split = Some((miss, bands.clone(), split_next.clone()));
        }
        if bands.is_empty() || bands[tall_ix].len() < 2 {
            break;
        }
        // Deal the groups of the tallest column onto two columns, tallest first.
        let mut groups_here = std::mem::take(&mut bands[tall_ix]);
        groups_here.sort_by(|&a, &b| size(b).1.total_cmp(&size(a).1).then(a.cmp(&b)));
        let (mut left, mut right) = (Vec::new(), Vec::new());
        for g in groups_here {
            if height(&left) <= height(&right) {
                left.push(g);
            } else {
                right.push(g);
            }
        }
        left.sort_by_key(|&g| (std::cmp::Reverse(degree(g)), g));
        right.sort_by_key(|&g| (std::cmp::Reverse(degree(g)), g));
        bands[tall_ix] = left;
        bands.insert(tall_ix + 1, right);
        split_next.insert(tall_ix + 1, true);
    }
    if let Some((_, b, s)) = best_split {
        bands = b;
        split_next = s;
    }

    let mut groups = vec![Rect::default(); n_groups];
    let tallest = bands.iter().map(|b| height(b)).fold(0.0f32, f32::max);
    let mut x = LEFT;
    for (bi, b) in bands.iter().enumerate() {
        if bi > 0 {
            x += if split_next[bi] { SPLIT_GAP } else { BAND_GAP };
        }
        let mut y = TOP + (tallest - height(b)) / 2.0;
        for &g in b {
            let (w, h) = size(g);
            groups[g] = Rect { x, y, w, h };
            y += h + GROUP_GAP;
        }
        x += width(b);
    }

    // Order each column by its neighbours' heights, a few sweeps.
    let mut pos = vec![(0.0f32, 0.0f32); n];
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for e in edges {
        adj[e.from].push(e.to);
        adj[e.to].push(e.from);
    }
    let assign = |members: &Vec<Vec<Vec<usize>>>, pos: &mut Vec<(f32, f32)>| {
        for g in 0..n_groups {
            let r = groups[g];
            for (c, column) in members[g].iter().enumerate() {
                let rows = members[g].iter().map(Vec::len).max().unwrap_or(1) as f32;
                let inner_h = rows * NODE_H + (rows - 1.0) * ROW_GAP;
                let col_h = column.len() as f32 * NODE_H + (column.len() as f32 - 1.0) * ROW_GAP;
                let y0 = r.y + HEADER + (inner_h - col_h) / 2.0;
                for (k, &i) in column.iter().enumerate() {
                    pos[i] = (
                        r.x + PAD_X + c as f32 * (NODE_W + COL_GAP),
                        y0 + k as f32 * (NODE_H + ROW_GAP),
                    );
                }
            }
        }
    };
    assign(&members, &mut pos);
    for _ in 0..4 {
        for group_columns in members.iter_mut() {
            for column in group_columns {
                let key = |i: usize| -> f32 {
                    if adj[i].is_empty() {
                        pos[i].1
                    } else {
                        adj[i].iter().map(|&j| pos[j].1).sum::<f32>() / adj[i].len() as f32
                    }
                };
                // Wrapped layers stay in their block (a block is one pass over the layers).
                let mut keyed: Vec<(usize, f32, usize)> =
                    column.iter().map(|&i| (block[i], key(i), i)).collect();
                keyed.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)).then(a.2.cmp(&b.2)));
                *column = keyed.into_iter().map(|(_, _, i)| i).collect();
            }
        }
        assign(&members, &mut pos);
    }
    Placed { pos, groups }
}

/// Mirrors x for right-to-left locales; `right` is the drawing's right edge.
pub fn mirror(x: f32, w: f32, right: f32) -> f32 {
    LEFT + right - x - w
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overlaps(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < NODE_W && (a.1 - b.1).abs() < NODE_H
    }

    fn edge(from: usize, to: usize) -> Edge {
        Edge {
            from,
            to,
            weight: 1,
        }
    }

    #[test]
    fn layers_follow_dependencies_and_survive_cycles() {
        assert_eq!(layers(3, &[(0, 1), (1, 2)]), [0, 1, 2]);
        assert_eq!(layers(3, &[(0, 2), (0, 1), (1, 2)]), [0, 1, 2]);
        let l = layers(2, &[(0, 1), (1, 0)]);
        assert_ne!(l[0], l[1], "a cycle still gets two columns");
    }

    #[test]
    fn users_sit_left_of_what_they_use_across_and_inside_groups() {
        // app(0) and gui(1) in group 0; core(2) in group 1; katla(3) in group 2.
        let group = [0, 0, 1, 2];
        let edges = [edge(1, 0), edge(0, 2), edge(1, 2), edge(2, 3)];
        let p = place(&group, &edges, 3);
        assert!(p.pos[1].0 < p.pos[0].0, "gui uses app inside the group");
        assert!(p.pos[0].0 < p.pos[2].0 && p.pos[2].0 < p.pos[3].0);
    }

    #[allow(clippy::needless_range_loop)]
    #[test]
    fn boxes_never_overlap_and_stay_inside_their_group_rect() {
        let n = 40;
        let group: Vec<usize> = (0..n).map(|i| i % 5).collect();
        let edges: Vec<Edge> = (0..n)
            .flat_map(|i| [edge(i, (i * 7 + 3) % n), edge(i, (i + 11) % n)])
            .collect();
        let p = place(&group, &edges, 5);
        for i in 0..n {
            for j in i + 1..n {
                assert!(!overlaps(p.pos[i], p.pos[j]), "{i} and {j} overlap");
            }
            let r = p.groups[group[i]];
            let (x, y) = p.pos[i];
            assert!(
                x >= r.x && y >= r.y && x + NODE_W <= r.x + r.w && y + NODE_H <= r.y + r.h,
                "{i} outside its group"
            );
        }
        for a in 0..5 {
            for b in a + 1..5 {
                let (r, s) = (p.groups[a], p.groups[b]);
                assert!(
                    r.x + r.w <= s.x || s.x + s.w <= r.x || r.y + r.h <= s.y || s.y + s.h <= r.y,
                    "groups {a} and {b} overlap"
                );
            }
        }
    }

    #[test]
    fn many_link_less_groups_wrap_into_a_landscape_drawing() {
        // 1 workspace of 6 boxes + 5 sibling repos of 1-5 boxes, only workspace -> repo links.
        let mut group = vec![0; 6];
        for (g, n) in [(1, 5), (2, 3), (3, 1), (4, 1), (5, 1)] {
            group.extend(std::iter::repeat_n(g, n));
        }
        let edges: Vec<Edge> = (0..6)
            .flat_map(|i| [edge(i, 6 + i), edge(i, 11 + (i % 5))])
            .collect();
        let p = place(&group, &edges, 6);
        let (w, h) = extent(&p);
        assert!(w / h >= 1.3 && w / h <= 2.0, "ratio {}", w / h);
    }

    #[test]
    fn a_deep_single_group_wraps_instead_of_growing_eight_columns_wide() {
        let n = 23;
        let group = vec![0; n];
        let edges: Vec<Edge> = (0..n - 1).map(|i| edge(i, i + 1)).collect();
        let p = place(&group, &edges, 1);
        let (w, h) = extent(&p);
        assert!(w / h >= 1.2 && w / h <= 2.0, "ratio {}", w / h);
        for i in 0..n {
            for j in i + 1..n {
                assert!(!overlaps(p.pos[i], p.pos[j]));
            }
        }
    }

    #[test]
    fn placement_is_deterministic_and_fast_for_large_graphs() {
        let n = 200;
        let group: Vec<usize> = (0..n).map(|i| i % 8).collect();
        let edges: Vec<Edge> = (0..n)
            .flat_map(|i| [edge(i, (i * 13 + 1) % n), edge(i, (i + 5) % n)])
            .collect();
        let t = std::time::Instant::now();
        let (a, b) = (place(&group, &edges, 8), place(&group, &edges, 8));
        assert!(t.elapsed().as_secs() < 5);
        assert_eq!(a.pos, b.pos);
    }
}
