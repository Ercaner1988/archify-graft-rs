use super::*;
use crate::poly::{crossings, hits_rect, overlap_length};
use std::time::Instant;

fn req(from: usize, to: usize, label: Option<&str>) -> EdgeRequest {
    EdgeRequest {
        from,
        to,
        label_width: label.map(|l| (l.chars().count() as f32 * 4.8 + 10.0).max(30.0)),
    }
}

/// The hand-drawn reference layout of the original pasli-beyin diagram (10 boxes, 9 edges).
fn reference() -> (Vec<Rect>, Vec<EdgeRequest>) {
    let at = |c: f32, r: f32| Rect::new(40.0 + c * 270.0, 80.0 + r * 220.0, 170.0, 64.0);
    let boxes = vec![
        at(0.0, 1.0), // 0 pano
        at(0.0, 2.0), // 1 kanca
        at(1.0, 2.0), // 2 serve
        at(2.0, 2.0), // 3 pencere
        at(2.0, 1.0), // 4 cekirdek
        at(3.0, 1.0), // 5 cli
        at(2.0, 0.0), // 6 fihrist
        at(3.0, 0.0), // 7 graphify
        at(4.0, 1.0), // 8 kasa
        at(4.0, 2.0), // 9 mistral
    ];
    let edges = vec![
        req(0, 4, Some("envanter oku")),
        req(1, 2, Some("olay (boru/spool)")),
        req(2, 3, Some("beyin.json tazele")),
        req(4, 6, Some("üç kanallı arama")),
        req(4, 7, Some("AST ölçüm (grafa YAZMAZ)")),
        req(3, 4, Some("bağlam · istem · gölge")),
        req(5, 4, Some("komutlar")),
        req(5, 8, Some("anahtar oku")),
        req(5, 9, Some("yalnız gonder")),
    ];
    (boxes, edges)
}

fn assert_clear_of_boxes(boxes: &[Rect], routed: &[RoutedEdge]) -> usize {
    let mut bad = 0;
    for r in routed {
        if boxes.iter().any(|b| hits_rect(&r.points, b)) {
            bad += 1;
        }
    }
    bad
}

fn assert_orthogonal(routed: &[RoutedEdge]) {
    for r in routed {
        for w in r.points.windows(2) {
            assert!(
                (w[0][0] - w[1][0]).abs() < 0.01 || (w[0][1] - w[1][1]).abs() < 0.01,
                "diagonal segment {:?}",
                w
            );
        }
    }
}

#[test]
fn reference_layout_is_clean() {
    let (boxes, edges) = reference();
    let routed = route_all(&boxes, &[], &edges, &Options::default());
    assert_eq!(routed.len(), 9);
    assert_eq!(assert_clear_of_boxes(&boxes, &routed), 0);
    assert_orthogonal(&routed);
    assert!(routed.iter().all(|r| r.clear));
    // every edge starts and ends on a box side
    for (r, e) in routed.iter().zip(&edges) {
        let on_side = |p: Pt, b: &Rect| {
            ((p[0] - b.x).abs() < 0.01 || (p[0] - b.right()).abs() < 0.01)
                && p[1] >= b.y
                && p[1] <= b.bottom()
                || ((p[1] - b.y).abs() < 0.01 || (p[1] - b.bottom()).abs() < 0.01)
                    && p[0] >= b.x
                    && p[0] <= b.right()
        };
        assert!(on_side(r.points[0], &boxes[e.from]));
        assert!(on_side(*r.points.last().unwrap(), &boxes[e.to]));
    }
    // labels fit plates and never overlap boxes or each other
    let plates: Vec<Rect> = routed.iter().filter_map(|r| r.label).collect();
    assert_eq!(plates.len(), 9);
    for (i, p) in plates.iter().enumerate() {
        assert!(boxes.iter().all(|b| !b.overlaps(p)), "plate {i} on a box");
        for q in &plates[i + 1..] {
            assert!(!p.overlaps(q), "plates overlap");
        }
    }
}

#[test]
fn facing_boxes_get_a_straight_line() {
    let boxes = vec![
        Rect::new(0.0, 0.0, 100.0, 50.0),
        Rect::new(200.0, 0.0, 100.0, 50.0),
    ];
    let r = route_all(&boxes, &[], &[req(0, 1, None)], &Options::default());
    assert_eq!(r[0].points, vec![[100.0, 25.0], [200.0, 25.0]]);
    assert_eq!(r[0].arrow.dir, [1.0, 0.0]);
}

#[test]
fn route_detours_around_a_blocking_box() {
    let boxes = vec![
        Rect::new(0.0, 100.0, 100.0, 50.0),
        Rect::new(150.0, 60.0, 80.0, 130.0),
        Rect::new(300.0, 100.0, 100.0, 50.0),
    ];
    let r = route_all(&boxes, &[], &[req(0, 2, None)], &Options::default());
    assert!(r[0].clear);
    assert_eq!(assert_clear_of_boxes(&boxes, &r), 0);
    assert!(r[0].points.len() >= 4, "{:?}", r[0].points);
}

#[test]
fn keepout_is_avoided() {
    let boxes = vec![
        Rect::new(0.0, 0.0, 100.0, 50.0),
        Rect::new(300.0, 0.0, 100.0, 50.0),
    ];
    let title = Rect::new(150.0, 10.0, 60.0, 30.0);
    let r = route_all(&boxes, &[title], &[req(0, 1, None)], &Options::default());
    assert!(!hits_rect(&r[0].points, &title));
}

#[test]
fn bidirectional_pair_does_not_overlap() {
    let boxes = vec![
        Rect::new(0.0, 0.0, 100.0, 50.0),
        Rect::new(200.0, 0.0, 100.0, 50.0),
    ];
    let r = route_all(
        &boxes,
        &[],
        &[req(0, 1, None), req(1, 0, None)],
        &Options::default(),
    );
    assert!(overlap_length(&r[0].points, &r[1].points) < 1.0);
    assert_ne!(r[0].points[0][1], r[1].points.last().unwrap()[1]);
}

#[test]
fn self_loop_stays_outside_the_box() {
    let boxes = vec![Rect::new(0.0, 0.0, 100.0, 50.0)];
    let r = route_all(&boxes, &[], &[req(0, 0, None)], &Options::default());
    assert!(!hits_rect(&r[0].points, &boxes[0]));
}

/// Deterministic pseudo random (no dependency).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn range(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn big_graph(n_boxes: usize, n_edges: usize) -> (Vec<Rect>, Vec<EdgeRequest>) {
    let mut rng = Lcg(7);
    let cols = (n_boxes as f32).sqrt().ceil() as usize;
    let boxes: Vec<Rect> = (0..n_boxes)
        .map(|i| {
            let (c, r) = ((i % cols) as f32, (i / cols) as f32);
            let jitter = (rng.range(12) as f32) - 6.0;
            Rect::new(
                40.0 + c * 260.0 + jitter,
                60.0 + r * 140.0 - jitter * 0.5,
                170.0,
                64.0,
            )
        })
        .collect();
    let mut edges = Vec::new();
    while edges.len() < n_edges {
        let a = rng.range(n_boxes);
        // layered-looking: next column, occasionally a long haul
        let b = if rng.range(8) == 0 {
            rng.range(n_boxes)
        } else {
            (a + 1 + rng.range(cols + 2)).min(n_boxes - 1)
        };
        if a != b {
            let label = (edges.len() % 3 == 0).then_some("kullanır ×4");
            edges.push(req(a, b, label));
        }
    }
    (boxes, edges)
}

#[test]
fn jittered_graph_is_clean_and_fast() {
    let (boxes, edges) = big_graph(200, 400);
    let t = Instant::now();
    let routed = route_all(&boxes, &[], &edges, &Options::default());
    let secs = t.elapsed().as_secs_f32();
    let bad = assert_clear_of_boxes(&boxes, &routed);
    assert_orthogonal(&routed);
    let unclear = routed.iter().filter(|r| !r.clear).count();
    let mut overlaps = 0;
    let mut cross = 0;
    for i in 0..routed.len() {
        for j in i + 1..routed.len() {
            if overlap_length(&routed[i].points, &routed[j].points) > 3.0
                && edges[i].from != edges[j].from
            {
                overlaps += 1;
            }
            cross += crossings(&routed[i].points, &routed[j].points);
        }
    }
    eprintln!(
        "200 boxes / 400 edges: {secs:.2}s, through-box {bad}, blind {unclear}, overlapping pairs {overlaps}, crossings {cross}"
    );
    assert_eq!(bad, 0, "edges crossing a box");
    assert_eq!(unclear, 0, "edges routed blind");
    assert!(secs < 60.0, "too slow: {secs}");
}

/// Layered layout like the bridge produces: aligned columns, edges mostly to the next column.
fn layered_graph(cols: usize, rows: usize, n_edges: usize) -> (Vec<Rect>, Vec<EdgeRequest>) {
    let mut rng = Lcg(11);
    let boxes: Vec<Rect> = (0..cols * rows)
        .map(|i| {
            let (c, r) = ((i / rows) as f32, (i % rows) as f32);
            Rect::new(40.0 + c * 270.0, 60.0 + r * 120.0, 170.0, 64.0)
        })
        .collect();
    let mut edges = Vec::new();
    while edges.len() < n_edges {
        let a = rng.range(cols * rows);
        let ca = a / rows;
        let skip = if rng.range(6) == 0 { 2 } else { 1 };
        if ca + skip >= cols {
            continue;
        }
        let b = (ca + skip) * rows + rng.range(rows);
        let label = (edges.len() % 2 == 0).then_some("kullanır ×7");
        edges.push(req(a, b, label));
    }
    (boxes, edges)
}

#[test]
fn layered_graph_200_boxes_400_edges_is_clean_and_separated() {
    let (boxes, edges) = layered_graph(10, 20, 400);
    let t = Instant::now();
    let routed = route_all(&boxes, &[], &edges, &Options::default());
    let secs = t.elapsed().as_secs_f32();
    let bad = assert_clear_of_boxes(&boxes, &routed);
    assert_orthogonal(&routed);
    let unclear = routed.iter().filter(|r| !r.clear).count();
    let mut overlaps = 0;
    for i in 0..routed.len() {
        for j in i + 1..routed.len() {
            if overlap_length(&routed[i].points, &routed[j].points) > 3.0 {
                overlaps += 1;
            }
        }
    }
    let plates: Vec<Rect> = routed.iter().filter_map(|r| r.label).collect();
    let mut plate_clash = 0;
    for (i, p) in plates.iter().enumerate() {
        plate_clash += boxes.iter().filter(|b| b.overlaps(p)).count();
        plate_clash += plates[i + 1..].iter().filter(|q| q.overlaps(p)).count();
    }
    eprintln!(
        "layered 200/400: {secs:.2}s, through-box {bad}, blind {unclear}, overlapping pairs {overlaps}, label clashes {plate_clash}"
    );
    assert_eq!(bad, 0);
    assert_eq!(unclear, 0);
    assert!(plate_clash <= 2, "label clashes: {plate_clash}");
    assert!(secs < 30.0);
}

#[test]
fn moderate_layered_graph_has_separated_edges() {
    let (boxes, edges) = layered_graph(5, 8, 60);
    let routed = route_all(&boxes, &[], &edges, &Options::default());
    assert_eq!(assert_clear_of_boxes(&boxes, &routed), 0);
    assert_orthogonal(&routed);
    let mut overlaps = 0;
    let mut longest = 0.0f32;
    for i in 0..routed.len() {
        for j in i + 1..routed.len() {
            let o = overlap_length(&routed[i].points, &routed[j].points);
            if o > 3.0 && edges[i].from != edges[j].from {
                overlaps += 1;
                longest = longest.max(o);
            }
        }
    }
    eprintln!("moderate 40/60: overlapping pairs {overlaps}, longest {longest}");
    assert!(overlaps <= 6, "overlapping pairs: {overlaps}");
}

#[test]
fn edges_leaving_one_box_form_a_bus_from_a_single_port() {
    let mut boxes = vec![Rect::new(0.0, 160.0, 150.0, 64.0)];
    let mut edges = Vec::new();
    for i in 0..5 {
        boxes.push(Rect::new(400.0, i as f32 * 120.0, 150.0, 64.0));
        edges.push(req(0, i + 1, None));
    }
    let routed = route_all(&boxes, &[], &edges, &Options::default());
    assert_eq!(assert_clear_of_boxes(&boxes, &routed), 0);
    assert!(routed.iter().all(|r| r.points[0] == routed[0].points[0]));
    assert!(routed.iter().all(|r| r.sides[0] == routed[0].sides[0]));
}
