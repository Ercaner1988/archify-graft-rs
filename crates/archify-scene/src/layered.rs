//! Layered (left-to-right) placement for diagrams that arrive without coordinates.
//!
//! Longest-path ranks with back edges ignored, barycentre ordering inside each rank, tall
//! ranks wrapped into several sub-columns, every column centred on the tallest one.

use archify_route::Rect;

#[derive(Debug, Clone, Copy)]
pub struct LayerOptions {
    pub gap_x: f32,
    pub gap_y: f32,
    pub origin: [f32; 2],
    /// Most boxes stacked in one column before it wraps.
    pub max_rows: usize,
}

impl Default for LayerOptions {
    fn default() -> Self {
        LayerOptions {
            gap_x: 100.0,
            gap_y: 56.0,
            origin: [40.0, 80.0],
            max_rows: 10,
        }
    }
}

/// Places `sizes.len()` boxes; `edges` are `(from, to)` indices. Returns one rect per box.
pub fn layered(sizes: &[[f32; 2]], edges: &[(usize, usize)], opts: &LayerOptions) -> Vec<Rect> {
    let n = sizes.len();
    if n == 0 {
        return Vec::new();
    }
    let rank = ranks(n, edges);
    let max_rank = rank.iter().copied().max().unwrap_or(0);
    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); max_rank + 1];
    for (i, &r) in rank.iter().enumerate() {
        layers[r].push(i);
    }
    order_layers(&mut layers, edges, &rank);

    // wrap tall layers into columns
    let mut columns: Vec<Vec<usize>> = Vec::new();
    for layer in &layers {
        for chunk in layer.chunks(opts.max_rows.max(1)) {
            columns.push(chunk.to_vec());
        }
    }
    let heights: Vec<f32> = columns
        .iter()
        .map(|c| c.iter().map(|&i| sizes[i][1]).sum::<f32>() + opts.gap_y * (c.len() as f32 - 1.0))
        .collect();
    let tallest = heights.iter().copied().fold(0.0f32, f32::max);
    let mut rects = vec![Rect::default(); n];
    let mut x = opts.origin[0];
    for (col, h) in columns.iter().zip(&heights) {
        let width = col.iter().map(|&i| sizes[i][0]).fold(0.0f32, f32::max);
        let mut y = opts.origin[1] + (tallest - h) * 0.5;
        for &i in col {
            rects[i] = Rect::new(x, y, sizes[i][0], sizes[i][1]);
            y += sizes[i][1] + opts.gap_y;
        }
        x += width + opts.gap_x;
    }
    rects
}

/// Longest-path rank from the sources; edges closing a cycle are skipped.
fn ranks(n: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(a, b) in edges {
        if a != b && a < n && b < n {
            adj[a].push(b);
        }
    }
    // 0 = unseen, 1 = on stack, 2 = done; back edges are dropped
    let mut state = vec![0u8; n];
    let mut dag: Vec<Vec<usize>> = vec![Vec::new(); n];
    for root in 0..n {
        if state[root] != 0 {
            continue;
        }
        let mut stack: Vec<(usize, usize)> = vec![(root, 0)];
        state[root] = 1;
        while let Some(&(u, i)) = stack.last() {
            if i < adj[u].len() {
                let v = adj[u][i];
                if let Some(top) = stack.last_mut() {
                    top.1 += 1;
                }
                match state[v] {
                    0 => {
                        dag[u].push(v);
                        state[v] = 1;
                        stack.push((v, 0));
                    }
                    2 => dag[u].push(v),
                    _ => {} // back edge
                }
            } else {
                state[u] = 2;
                stack.pop();
            }
        }
    }
    let mut indeg = vec![0usize; n];
    for outs in &dag {
        for &v in outs {
            indeg[v] += 1;
        }
    }
    let mut rank = vec![0usize; n];
    let mut queue: Vec<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();
    let mut head = 0;
    while head < queue.len() {
        let u = queue[head];
        head += 1;
        for &v in &dag[u] {
            rank[v] = rank[v].max(rank[u] + 1);
            indeg[v] -= 1;
            if indeg[v] == 0 {
                queue.push(v);
            }
        }
    }
    rank
}

/// A few barycentre sweeps: each box moves towards the mean position of its neighbours in the
/// adjacent layer.
fn order_layers(layers: &mut [Vec<usize>], edges: &[(usize, usize)], rank: &[usize]) {
    let n = rank.len();
    let mut nbrs: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(a, b) in edges {
        if a != b && a < n && b < n {
            nbrs[a].push(b);
            nbrs[b].push(a);
        }
    }
    let mut pos = vec![0.0f32; n];
    let refresh = |layers: &[Vec<usize>], pos: &mut [f32]| {
        for l in layers {
            for (i, &v) in l.iter().enumerate() {
                pos[v] = i as f32;
            }
        }
    };
    refresh(layers, &mut pos);
    for sweep in 0..6 {
        let forward = sweep % 2 == 0;
        let order: Vec<usize> = if forward {
            (1..layers.len()).collect()
        } else {
            (0..layers.len().saturating_sub(1)).rev().collect()
        };
        for li in order {
            let towards = if forward { li - 1 } else { li + 1 };
            let mut key: Vec<(usize, f32)> = layers[li]
                .iter()
                .map(|&v| {
                    let mut sum = 0.0;
                    let mut cnt = 0.0;
                    for &other in &nbrs[v] {
                        if rank[other] == towards {
                            sum += pos[other];
                            cnt += 1.0;
                        }
                    }
                    (v, if cnt > 0.0 { sum / cnt } else { pos[v] })
                })
                .collect();
            key.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            layers[li] = key.into_iter().map(|(v, _)| v).collect();
            refresh(layers, &mut pos);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_goes_left_to_right_and_boxes_do_not_overlap() {
        let sizes = vec![[100.0, 50.0]; 4];
        let r = layered(&sizes, &[(0, 1), (1, 2), (2, 3)], &LayerOptions::default());
        assert!(r[0].x < r[1].x && r[1].x < r[2].x && r[2].x < r[3].x);
        for i in 0..4 {
            for j in i + 1..4 {
                assert!(!r[i].overlaps(&r[j]));
            }
        }
    }

    #[test]
    fn cycles_do_not_hang() {
        let sizes = vec![[100.0, 50.0]; 3];
        let r = layered(&sizes, &[(0, 1), (1, 2), (2, 0)], &LayerOptions::default());
        assert_eq!(r.len(), 3);
        assert!(r[0].x < r[2].x);
    }

    #[test]
    fn tall_layers_wrap() {
        let sizes = vec![[100.0, 50.0]; 25];
        let edges: Vec<(usize, usize)> = (1..25).map(|i| (0, i)).collect();
        let opts = LayerOptions::default();
        let r = layered(&sizes, &edges, &opts);
        let bottom = r.iter().map(|b| b.bottom()).fold(0.0f32, f32::max);
        assert!(bottom < opts.origin[1] + 10.0 * 50.0 + 9.0 * opts.gap_y + 1.0);
    }
}
