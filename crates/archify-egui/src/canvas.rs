//! The interactive canvas: input, motion update, hit testing and the paint order.

use crate::overlay;
use crate::view_data::{BeamColor, FocusKey, ViewData};
use archify_egui_paint::paint::{self, NodeLook, Xf};
use archify_motion::{
    wheel_factor, BeamCycle, MotionSettings, TourEvent, CAMERA_SECS, TOUR_CAMERA_SECS,
};
use archify_style::Tokens;
use egui::{CursorIcon, Key, PointerButton, Pos2, Sense, Ui};

/// Everything the canvas needs from the app for one frame.
pub struct Cx<'a> {
    pub tokens: &'a Tokens,
    pub motion: MotionSettings,
    pub clock: f32,
    pub dt: f32,
    pub search: &'a str,
    pub locale: &'a str,
    pub route_mode: bool,
    pub can_drill: bool,
}

#[derive(Default)]
pub struct Out {
    /// Box id double-clicked while a drill hook is installed.
    pub drill: Option<String>,
    /// Text for the status bar.
    pub status: String,
    /// The view needs another frame.
    pub animating: bool,
}

/// A selected box keeps its beams flowing this long, then the canvas goes quiet again.
const SELECT_BEAM_SECS: f32 = 4.2;
const FIT_PADDING: f32 = 56.0;
const FIT_MAX_ZOOM: f32 = 1.25;

pub fn show(ui: &mut Ui, vd: &mut ViewData, cx: &Cx) -> Out {
    let mut out = Out::default();
    let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let rect = resp.rect;
    let ctx = ui.ctx().clone();
    let t = cx.tokens;
    let reduced = cx.motion.reduced_motion;
    let viewport = [rect.width(), rect.height()];
    vd.vs.set_viewport(viewport);

    let resized = (viewport[0] - vd.last_viewport[0]).abs() > 1.0
        || (viewport[1] - vd.last_viewport[1]).abs() > 1.0;
    if rect.width() > 8.0 && (!vd.fitted || (resized && !vd.touched)) {
        let (z, pan) = vd.initial_view(viewport);
        vd.vs.set(z, pan);
        if !vd.fitted {
            vd.intro_t = 0.0;
        }
        vd.fitted = true;
    }
    vd.last_viewport = viewport;

    // ---- input: camera ----
    let typing = ctx.egui_wants_keyboard_input();
    if resp.dragged_by(PointerButton::Primary) || resp.dragged_by(PointerButton::Middle) {
        let d = resp.drag_delta();
        vd.vs.pan_by([d.x, d.y]);
        vd.touched = true;
        ctx.set_cursor_icon(CursorIcon::Grabbing);
    }
    if resp.hovered() {
        let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
        let pinch = ctx.input(|i| i.zoom_delta());
        let factor = wheel_factor(scroll) * pinch;
        if (factor - 1.0).abs() > 1e-4 {
            if let Some(p) = resp.hover_pos() {
                vd.vs.zoom_at([p.x - rect.min.x, p.y - rect.min.y], factor);
                vd.touched = true;
            }
        }
    }
    let fit_now = |vd: &mut ViewData| {
        vd.vs.fly_to_rect(
            vd.content_rect(),
            viewport,
            FIT_PADDING,
            FIT_MAX_ZOOM,
            CAMERA_SECS,
        );
    };
    if !typing {
        let (next, prev, play, fit, esc, zin, zout, zero) = ctx.input(|i| {
            (
                i.key_pressed(Key::CloseBracket),
                i.key_pressed(Key::OpenBracket),
                i.key_pressed(Key::P),
                i.key_pressed(Key::F),
                i.key_pressed(Key::Escape),
                i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals),
                i.key_pressed(Key::Minus),
                i.key_pressed(Key::Num0),
            )
        });
        if next {
            vd.tour.next();
        }
        if prev {
            vd.tour.prev();
        }
        if play {
            vd.tour.toggle_play();
        }
        if fit || zero {
            fit_now(vd);
        }
        if esc {
            vd.tour.stop();
            vd.selected = None;
            vd.legend_role = None;
            vd.route.clear();
        }
        let centre = [viewport[0] * 0.5, viewport[1] * 0.5];
        if zin {
            vd.vs.zoom_at(centre, 1.25);
        }
        if zout {
            vd.vs.zoom_at(centre, 0.8);
        }
    }

    if vd.tour.is_active() {
        vd.touched = true;
    }

    // ---- time: advance every state machine ----
    let mut animating = vd.vs.update(cx.dt);
    let ev = vd.tour.update(cx.dt);
    if let Some(ev) = ev {
        match ev {
            TourEvent::Enter(_) => {
                if let Some(frame) = vd.tour_frame() {
                    let secs = if reduced { 0.0 } else { TOUR_CAMERA_SECS };
                    vd.vs.fly_to_rect(frame, viewport, 64.0, 1.65, secs);
                }
            }
            TourEvent::Finished => fit_now(vd),
        }
    }
    animating |= vd.tour.is_animating();
    if !vd.intro.is_done(vd.intro_t, vd.scene.edges.len()) {
        vd.intro_t += cx.dt;
        animating = true;
    }

    // ---- hit testing ----
    let xf = Xf {
        origin: rect.min + egui::vec2(vd.vs.pan[0], vd.vs.pan[1]),
        zoom: vd.vs.zoom,
    };
    let pointer = if resp.dragged() {
        None
    } else {
        resp.hover_pos()
    };
    let hit_node = pointer.and_then(|p| {
        (0..vd.scene.nodes.len())
            .rev()
            .find(|&i| xf.rect(&vd.scene.nodes[i].rect).expand(2.0).contains(p))
    });
    let hit_edge = if hit_node.is_none() {
        pointer.and_then(|p| {
            (0..vd.scene.edges.len())
                .filter(|&i| vd.focus.edge_alpha(i) > 0.5)
                .map(|i| {
                    let pts: Vec<Pos2> = vd.geoms[i].flat.iter().map(|&q| xf.pt(q)).collect();
                    (i, paint::dist_to_polyline(&pts, p))
                })
                .filter(|&(_, d)| d < 6.0)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(i, _)| i)
        })
    } else {
        None
    };
    let active = vd.hover.update(cx.dt, hit_node);
    animating |= vd.hover.is_pending();
    if hit_node.is_some() {
        ctx.set_cursor_icon(CursorIcon::PointingHand);
    }
    vd.hovered_edge = hit_edge;

    // ---- clicks ----
    if resp.double_clicked() {
        match hit_node {
            Some(i) if cx.can_drill => out.drill = Some(vd.scene.nodes[i].id.clone()),
            Some(i) => {
                vd.selected = Some(i);
                if let Some(r) = vd.nodes_rect(&[i]) {
                    vd.vs.fly_to_rect(r, viewport, 120.0, 1.6, CAMERA_SECS);
                }
            }
            None => {
                vd.selected = None;
                fit_now(vd);
            }
        }
    } else if resp.clicked() {
        match hit_node {
            Some(i) if cx.route_mode => {
                if let Some(start) = vd.route_start.take() {
                    vd.route = vd.shortest_path(start, i).unwrap_or_default();
                } else {
                    vd.route_start = Some(i);
                    vd.route.clear();
                }
            }
            Some(i) => {
                vd.selected = if vd.selected == Some(i) {
                    None
                } else {
                    Some(i)
                }
            }
            None => {
                vd.selected = None;
                vd.legend_role = None;
                vd.route_start = None;
                vd.route.clear();
            }
        }
    }

    // ---- focus ----
    let query = cx.search.trim();
    let key = if vd.tour.is_active() {
        FocusKey::Tour(vd.tour.current().unwrap_or(0))
    } else if !vd.route.is_empty() {
        FocusKey::Route
    } else if let Some(s) = vd.selected {
        FocusKey::Selected(s)
    } else if let Some(r) = vd.legend_role {
        FocusKey::Legend(r)
    } else if !query.is_empty() {
        FocusKey::Search(query.to_string())
    } else if let Some(e) = hit_edge {
        FocusKey::Edge(e)
    } else if let Some(a) = active {
        FocusKey::Hover(a)
    } else {
        FocusKey::None
    };
    if key != vd.key {
        match vd.focus_for(&key, query) {
            Some((set, dim)) => vd.focus.set_focus(Some(&set), dim),
            None => vd.focus.set_focus(None, 1.0),
        }
        vd.key = key;
        vd.beam_t0 = cx.clock;
    }
    animating |= vd.focus.update(cx.dt);

    // ---- beams ----
    let beam_node = active.or(vd
        .selected
        .filter(|_| cx.clock - vd.beam_t0 < SELECT_BEAM_SECS));
    let mut beams = if reduced {
        Vec::new()
    } else {
        vd.beams(beam_node, cx.motion.ambient)
    };
    if let (Some(e), false, false) = (hit_edge, reduced, beams.iter().any(|b| !b.ambient)) {
        beams.push(crate::view_data::BeamSpec {
            edge: e,
            color: BeamColor::Accent,
            ambient: false,
            phase: 0.0,
        });
    }
    animating |= beams.iter().any(|b| !b.ambient);
    // ambient light alone is throttled (about 40 fps) instead of redrawing every vsync
    let ambient_only = !animating && !beams.is_empty();

    // ---- paint ----
    paint::background(&painter, t, rect, &xf);
    let canvas = rect.expand(40.0);
    let painter = painter.with_clip_rect(rect);
    for r in &vd.scene.regions {
        if xf.rect(&r.rect).intersects(canvas) {
            paint::region(&painter, t, r, &xf);
        }
    }
    let n_edges = vd.scene.edges.len();
    for (i, e) in vd.scene.edges.iter().enumerate() {
        let alpha = vd.focus.edge_alpha(i) * vd.intro.edge_opacity(i, vd.intro_t);
        let style = t.edge_style(e.kind);
        let progress = vd.intro.edge_progress(i, vd.intro_t);
        let g = &vd.geoms[i];
        let bbox = egui::Rect::from_points(&g.flat.iter().map(|&q| xf.pt(q)).collect::<Vec<_>>());
        if bbox.intersects(canvas) {
            paint::edge(&painter, e, g, &style, &xf, alpha, progress);
        }
    }
    for b in &beams {
        let cycle = if b.ambient {
            BeamCycle::ambient()
        } else {
            BeamCycle::default()
        };
        let color = match b.color {
            BeamColor::Out => t.stroke(archify_ir::SemanticRole::Frontend),
            BeamColor::In => t.stroke(archify_ir::SemanticRole::Database),
            BeamColor::Loop => t.stroke(archify_ir::SemanticRole::Security),
            BeamColor::Accent => t.arrow_emphasis,
        };
        let time = if b.ambient {
            cx.clock
        } else {
            cx.clock - vd.beam_t0
        };
        let strength = if b.ambient {
            0.7 * vd.focus.edge_alpha(b.edge)
        } else {
            1.0
        };
        let width = if b.ambient { 1.8 } else { 3.2 };
        paint::beam(
            &painter,
            &vd.geoms[b.edge],
            &xf,
            &cycle,
            time,
            b.phase,
            color,
            width,
            t.beam_halo,
            strength,
        );
    }
    for (i, e) in vd.scene.edges.iter().enumerate() {
        let a = vd.focus.edge_alpha(i) * vd.intro.edge_opacity(i, vd.intro_t);
        if a > 0.3 && i < n_edges {
            paint::edge_label(&painter, t, e, &t.edge_style(e.kind), &xf, a);
        }
    }
    let search_hits: Vec<usize> = if matches!(vd.key, FocusKey::Search(_)) {
        vd.search_hits(query)
    } else {
        Vec::new()
    };
    for (i, n) in vd.scene.nodes.iter().enumerate() {
        if !xf.rect(&n.rect).intersects(canvas) {
            continue;
        }
        let look = NodeLook {
            alpha: vd.focus.node_alpha(i),
            glow: vd.focus.node_glow(i),
            selected: vd.selected == Some(i),
            pulse: if reduced {
                0.0
            } else {
                vd.intro.node_pulse(i, vd.intro_t)
            },
            search_hit: search_hits.contains(&i),
        };
        paint::node(&painter, t, n, &xf, look);
    }

    // ---- tooltip, overlays, status ----
    if let Some(i) = hit_node {
        let n = &vd.scene.nodes[i];
        let role = archify_style::role_name(n.role, cx.locale);
        let (inc, outg) = vd.pairs.iter().fold((0, 0), |(a, b), &(f, to)| {
            (a + usize::from(to == i), b + usize::from(f == i))
        });
        let (w_in, w_out) = if cx.locale == "tr" {
            ("gelen", "giden")
        } else {
            ("in", "out")
        };
        let tip = n.tooltip.clone();
        resp.clone().on_hover_ui_at_pointer(|ui| {
            ui.label(tip);
            ui.small(format!("{role} · {w_in} {inc} · {w_out} {outg}"));
        });
        out.status = format!("{} — {role} · {w_in} {inc} · {w_out} {outg}", n.tooltip);
    } else if let Some(e) = hit_edge {
        let (a, b) = vd.pairs[e];
        let edge = &vd.scene.edges[e];
        out.status = format!(
            "{} → {}{}",
            vd.scene.nodes[a].label.text,
            vd.scene.nodes[b].label.text,
            edge.full_label
                .as_deref()
                .map(|l| format!("  ({l})"))
                .unwrap_or_default()
        );
    }
    overlay::draw(ui, &painter, rect, t, vd, cx, &mut out);
    out.animating = animating || vd.vs.is_animating();
    if out.animating {
        ctx.request_repaint();
    } else if ambient_only {
        ctx.request_repaint_after(std::time::Duration::from_millis(25));
    }
    out
}
