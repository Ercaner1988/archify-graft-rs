//! Painting primitives: backdrop, regions, boxes, edges, arrowheads, plates and beams.

use crate::theme::{c, fade, paint_sigil};
use archify_motion::{beam::gradient_pieces, slice, BeamCycle};
use archify_scene::{SceneEdge, SceneNode, SceneRegion};
use archify_style::{font, EdgeStyle, Glow, Rgba, Tokens};
use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke, StrokeKind};

/// World to screen: `screen = origin + world * zoom`.
#[derive(Clone, Copy)]
pub struct Xf {
    pub origin: Pos2,
    pub zoom: f32,
}

impl Xf {
    pub fn pt(&self, p: [f32; 2]) -> Pos2 {
        Pos2::new(
            self.origin.x + p[0] * self.zoom,
            self.origin.y + p[1] * self.zoom,
        )
    }

    pub fn rect(&self, r: &archify_scene::Rect) -> Rect {
        Rect::from_min_size(
            self.pt([r.x, r.y]),
            egui::vec2(r.w * self.zoom, r.h * self.zoom),
        )
    }
}

pub fn mono(size: f32) -> FontId {
    FontId::monospace(size)
}

/// Text is skipped below this on-screen size; it would be an unreadable smear.
const MIN_TEXT_PX: f32 = 4.6;
/// Smallest size the main label is lifted to when it still fits its box.
const READABLE_PX: f32 = 7.0;

pub fn background(p: &Painter, t: &Tokens, canvas: Rect, xf: &Xf) {
    p.rect_filled(canvas, 0.0, c(t.bg));
    if let Some((a, b)) = t.wash {
        let r = canvas.width().max(canvas.height()) * 0.55;
        for (center, col) in [
            (
                canvas.left_top() + egui::vec2(canvas.width() * 0.18, -canvas.height() * 0.05),
                a,
            ),
            (
                canvas.left_top() + egui::vec2(canvas.width() * 0.92, canvas.height() * 0.18),
                b,
            ),
        ] {
            for i in 0..12 {
                let k = 1.0 - i as f32 / 12.0;
                p.circle_filled(center, r * k, fade(col, 0.11 * (1.0 - k * 0.3)));
            }
        }
    }
    let mut step = 40.0 * xf.zoom;
    while step < 14.0 {
        step *= 2.0;
    }
    let grid = t.grid.scale_alpha(t.grid_opacity);
    let (ox, oy) = (
        (xf.origin.x - canvas.min.x).rem_euclid(step),
        (xf.origin.y - canvas.min.y).rem_euclid(step),
    );
    let stroke = Stroke::new(1.0, c(grid.scale_alpha(0.75)));
    if t.grid_dash.is_empty() {
        let mut x = canvas.min.x + ox;
        while x < canvas.max.x {
            p.line_segment(
                [Pos2::new(x, canvas.min.y), Pos2::new(x, canvas.max.y)],
                stroke,
            );
            x += step;
        }
        let mut y = canvas.min.y + oy;
        while y < canvas.max.y {
            p.line_segment(
                [Pos2::new(canvas.min.x, y), Pos2::new(canvas.max.x, y)],
                stroke,
            );
            y += step;
        }
    } else {
        // dashed grids read as dots on the crossings
        let col = c(grid.scale_alpha(0.95));
        let mut y = canvas.min.y + oy;
        while y < canvas.max.y {
            let mut x = canvas.min.x + ox;
            while x < canvas.max.x {
                p.circle_filled(Pos2::new(x, y), 1.0, col);
                x += step;
            }
            y += step;
        }
    }
}

pub fn region(p: &Painter, t: &Tokens, r: &SceneRegion, xf: &Xf) {
    let rect = xf.rect(&r.rect);
    let radius = t.region_radius * xf.zoom;
    p.rect_filled(rect, radius, c(t.region_fill));
    let stroke = Stroke::new(1.0, fade(t.region_stroke, 0.7));
    let dash: Vec<f32> = t
        .region_dash
        .iter()
        .map(|d| d * xf.zoom.clamp(0.6, 1.4))
        .collect();
    let outline = rounded_rect_outline(rect, radius);
    let (dashes, gaps): (Vec<f32>, Vec<f32>) = dash
        .chunks(2)
        .map(|w| (w[0], *w.get(1).unwrap_or(&w[0])))
        .unzip();
    p.extend(Shape::dashed_line_with_offset(
        &outline, stroke, &dashes, &gaps, 0.0,
    ));
    let plate = xf.rect(&r.title_plate);
    if plate.width() > 6.0 {
        p.rect_filled(plate, 3.0 * xf.zoom, c(t.mask));
        let size = r.title.font * xf.zoom;
        if size >= MIN_TEXT_PX {
            p.text(
                Pos2::new(plate.min.x + 6.0 * xf.zoom, plate.center().y),
                Align2::LEFT_CENTER,
                &r.title.text,
                mono(size),
                c(t.region_stroke),
            );
        }
        if let (Some(sub), Some(sp)) = (&r.sub, &r.sub_plate) {
            let sp = xf.rect(sp);
            let size = sub.font * xf.zoom;
            if size >= MIN_TEXT_PX {
                p.text(
                    Pos2::new(sp.max.x - 6.0 * xf.zoom, sp.center().y),
                    Align2::RIGHT_CENTER,
                    &sub.text,
                    mono(size),
                    c(t.muted),
                );
            }
        }
    }
}

/// Clockwise outline with quarter-circle corners (8 segments each).
fn rounded_rect_outline(r: Rect, radius: f32) -> Vec<Pos2> {
    let rad = radius.min(r.width() * 0.5).min(r.height() * 0.5).max(0.0);
    let mut pts = Vec::with_capacity(40);
    let corners = [
        (Pos2::new(r.max.x - rad, r.min.y + rad), -90.0f32),
        (Pos2::new(r.max.x - rad, r.max.y - rad), 0.0),
        (Pos2::new(r.min.x + rad, r.max.y - rad), 90.0),
        (Pos2::new(r.min.x + rad, r.min.y + rad), 180.0),
    ];
    for (center, start) in corners {
        for i in 0..=8 {
            let a = (start + i as f32 * 90.0 / 8.0).to_radians();
            pts.push(Pos2::new(
                center.x + rad * a.cos(),
                center.y + rad * a.sin(),
            ));
        }
    }
    pts.push(pts[0]);
    pts
}

/// How a box is lit this frame.
#[derive(Clone, Copy, Default)]
pub struct NodeLook {
    pub alpha: f32,
    pub glow: f32,
    pub selected: bool,
    pub pulse: f32,
    pub search_hit: bool,
}

pub fn node(p: &Painter, t: &Tokens, n: &SceneNode, xf: &Xf, look: NodeLook) {
    let rect = xf.rect(&n.rect);
    let z = xf.zoom;
    let a = look.alpha.clamp(0.0, 1.0);
    let role = t.role(n.role);
    let radius = t.node_radius * z;
    if look.glow > 0.02 {
        match t.glow {
            Glow::Bloom(b) => {
                for (k, al) in [(1.0, 0.07), (0.62, 0.11), (0.3, 0.18)] {
                    let e = b * z * k * look.glow;
                    p.rect_stroke(
                        rect.expand(e * 0.5),
                        radius + e * 0.5,
                        Stroke::new(e.max(1.0), fade(role.stroke, al * a)),
                        StrokeKind::Middle,
                    );
                }
            }
            Glow::Paper { dy, blur } => {
                let shadow = t.text.with_alpha(0.16 * look.glow * a);
                p.rect_filled(
                    rect.translate(egui::vec2(0.0, dy * z)).expand(blur * z),
                    radius + blur * z,
                    c(shadow),
                );
            }
            Glow::None => {}
        }
    }
    if look.pulse > 0.02 {
        let e = 5.0 * z * look.pulse;
        p.rect_stroke(
            rect.expand(e),
            radius + e,
            Stroke::new(
                1.5 * z.max(0.7),
                fade(t.arrow_emphasis, 0.6 * look.pulse * a),
            ),
            StrokeKind::Middle,
        );
    }
    p.rect_filled(rect, radius, fade(t.mask, a));
    let stroke_w = (1.5 * z).clamp(0.8, 2.4) + if look.selected { 0.9 } else { 0.0 };
    p.rect(
        rect,
        radius,
        fade(role.fill, a),
        Stroke::new(stroke_w, fade(role.stroke, a)),
        StrokeKind::Middle,
    );
    if look.search_hit {
        p.rect_stroke(
            rect.expand(3.0 * z),
            radius + 3.0 * z,
            Stroke::new(1.5, fade(t.arrow_emphasis, a)),
            StrokeKind::Middle,
        );
    }
    let cx = rect.center().x;
    // `floor` keeps the main label readable when zoomed out, as long as it fits the box
    let draw = |line: &archify_scene::Line, color: Rgba, weight: f32, floor: bool| {
        let mut size = line.font * z;
        if floor && size < READABLE_PX {
            let per_px = archify_style::text::width(&line.text, 1.0).max(0.1);
            let fit = ((rect.width() - 8.0) / per_px).min(READABLE_PX);
            if fit >= 5.0 && fit > size {
                size = fit;
            }
        }
        if size >= MIN_TEXT_PX {
            p.text(
                Pos2::new(cx, rect.min.y + line.y * z),
                Align2::CENTER_CENTER,
                &line.text,
                mono(size),
                fade(color, a * weight),
            );
        }
    };
    draw(&n.label, t.text, 1.0, true);
    if let Some(s) = &n.sublabel {
        if s.font * z >= 5.4 {
            draw(s, t.muted, 1.0, false);
        }
    }
    if let Some(tag) = &n.tag {
        if tag.font * z >= 5.8 {
            draw(tag, role.stroke, 0.85, false);
        }
    }
    if 11.0 * z >= 7.0 {
        let s = Rect::from_min_size(
            rect.min + egui::vec2(6.0 * z, 6.0 * z),
            egui::vec2(11.0 * z, 11.0 * z),
        );
        paint_sigil(
            p,
            n.role,
            s,
            Stroke::new((1.35 * z * 0.6875).max(0.8), fade(role.stroke, 0.76 * a)),
        );
    }
}

/// Flattened, cached geometry of an edge in world space.
pub struct EdgeGeom {
    pub flat: Vec<[f32; 2]>,
    pub len: f32,
}

impl EdgeGeom {
    pub fn of(e: &SceneEdge) -> EdgeGeom {
        let flat = archify_route::flatten_rounded(&e.route.points, archify_route::CORNER_RADIUS, 6);
        let len = archify_motion::polyline_len(&flat);
        EdgeGeom { flat, len }
    }
}

/// Draws one edge from its start to `progress` (0..=1) of its length.
pub fn edge(
    p: &Painter,
    e: &SceneEdge,
    g: &EdgeGeom,
    style: &EdgeStyle,
    xf: &Xf,
    alpha: f32,
    progress: f32,
) {
    if alpha < 0.01 || progress <= 0.0 {
        return;
    }
    let pts: Vec<Pos2> = if progress >= 0.999 {
        g.flat.iter().map(|&q| xf.pt(q)).collect()
    } else {
        slice(&g.flat, 0.0, g.len * progress)
            .into_iter()
            .map(|q| xf.pt(q))
            .collect()
    };
    if pts.len() < 2 {
        return;
    }
    let col = c(style.color).gamma_multiply(alpha);
    let w = (style.width * xf.zoom).clamp(0.9, 3.2);
    let stroke = Stroke::new(w, col);
    if style.dash.is_empty() {
        p.add(Shape::line(pts, stroke));
    } else {
        let k = xf.zoom.clamp(0.7, 1.6);
        let dashes: Vec<f32> = style.dash.iter().step_by(2).map(|d| d * k).collect();
        let gaps: Vec<f32> = style
            .dash
            .iter()
            .skip(1)
            .step_by(2)
            .map(|d| d * k)
            .collect();
        p.extend(Shape::dashed_line_with_offset(
            &pts, stroke, &dashes, &gaps, 0.0,
        ));
    }
    if progress >= 0.97 {
        arrowhead(p, e, xf, col);
    }
}

fn arrowhead(p: &Painter, e: &SceneEdge, xf: &Xf, col: Color32) {
    let a = e.route.arrow;
    let k = xf.zoom.clamp(0.7, 1.5);
    let (len, half) = (
        archify_route::ARROW_LEN * k,
        archify_route::ARROW_HALF_W * k,
    );
    let tip = xf.pt(a.tip);
    let dir = egui::vec2(a.dir[0], a.dir[1]);
    let back = tip - dir * len;
    let perp = egui::vec2(-dir.y, dir.x) * half;
    p.add(Shape::convex_polygon(
        vec![tip, back + perp, back - perp],
        col,
        Stroke::NONE,
    ));
}

/// Edge-label plate with its text.
pub fn edge_label(p: &Painter, t: &Tokens, e: &SceneEdge, style: &EdgeStyle, xf: &Xf, alpha: f32) {
    let (Some(text), Some(plate)) = (&e.label, e.route.label) else {
        return;
    };
    let size = font::EDGE_LABEL * xf.zoom;
    if size < MIN_TEXT_PX || alpha < 0.05 {
        return;
    }
    let r = xf.rect(&plate);
    p.rect_filled(r, 3.0 * xf.zoom, fade(t.mask, alpha));
    p.text(
        r.center(),
        Align2::CENTER_CENTER,
        text,
        mono(size),
        fade(style.label_color, alpha),
    );
}

/// One looping light beam along an edge.
#[allow(clippy::too_many_arguments)]
pub fn beam(
    p: &Painter,
    g: &EdgeGeom,
    xf: &Xf,
    cycle: &BeamCycle,
    t: f32,
    phase: f32,
    color: Rgba,
    width: f32,
    halo: bool,
    strength: f32,
) {
    if g.len < 4.0 {
        return;
    }
    let (head, tail) = cycle.window(g.len, t, phase);
    let vis = cycle.intensity(t, phase) * strength;
    if head - tail < 1.0 || vis < 0.02 {
        return;
    }
    let w = (width * xf.zoom).clamp(1.6, 4.5);
    for (s0, s1, a) in gradient_pieces(head, tail, 10) {
        let pts: Vec<Pos2> = slice(&g.flat, s0, s1)
            .into_iter()
            .map(|q| xf.pt(q))
            .collect();
        if pts.len() < 2 {
            continue;
        }
        if halo {
            p.add(Shape::line(
                pts.clone(),
                Stroke::new(w * 2.6, fade(color, a * vis * 0.16)),
            ));
        }
        p.add(Shape::line(
            pts,
            Stroke::new(w, fade(color, (a * vis).min(1.0) * 0.95)),
        ));
    }
}

/// Distance in px from `q` to a screen polyline.
pub fn dist_to_polyline(pts: &[Pos2], q: Pos2) -> f32 {
    pts.windows(2)
        .map(|w| {
            let (a, b) = (w[0], w[1]);
            let ab = b - a;
            let l2 = ab.length_sq().max(1e-6);
            let t = (((q - a).dot(ab)) / l2).clamp(0.0, 1.0);
            (a + ab * t).distance(q)
        })
        .fold(f32::INFINITY, f32::min)
}
