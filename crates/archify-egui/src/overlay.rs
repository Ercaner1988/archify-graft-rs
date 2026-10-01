//! Overlays drawn above the canvas: title block, legend, tour panel and key hints.

use crate::canvas::{Cx, Out};
use crate::strings::strings;
use crate::view_data::ViewData;
use archify_egui_paint::paint::mono;
use archify_egui_paint::theme::{c, fade};
use archify_style::{role_name, Tokens};
use egui::{
    Align2, Area, FontId, Frame, Id, Margin, Order, Painter, Pos2, Rect, RichText, Sense, Stroke,
    Ui,
};

/// Heading face. Editorial would use a serif; egui ships none, so headings stay proportional.
pub fn serif(_t: &Tokens, _cx: &Cx, size: f32) -> FontId {
    FontId::proportional(size)
}

pub fn draw(
    ui: &mut Ui,
    painter: &Painter,
    rect: Rect,
    t: &Tokens,
    vd: &mut ViewData,
    cx: &Cx,
    out: &mut Out,
) {
    let s = strings(cx.locale);
    // title block
    let origin = rect.min + egui::vec2(18.0, 16.0);
    let title = painter.text(
        origin,
        Align2::LEFT_TOP,
        &vd.scene.title,
        serif(t, cx, 17.0),
        c(t.text),
    );
    let mut y = title.max.y + 3.0;
    if let Some(sub) = &vd.scene.subtitle {
        let r = painter.text(
            Pos2::new(origin.x, y),
            Align2::LEFT_TOP,
            sub,
            mono(10.0),
            c(t.muted),
        );
        y = r.max.y + 2.0;
    }
    painter.text(
        Pos2::new(origin.x, y),
        Align2::LEFT_TOP,
        format!(
            "{} {} · {} {} · {}/{} ok · {:.0}%",
            vd.scene.nodes.len(),
            s.boxes,
            vd.scene.edges.len(),
            s.links,
            vd.scene.stats.edges - vd.scene.stats.through_box - vd.scene.stats.blind,
            vd.scene.stats.edges,
            vd.vs.zoom * 100.0
        ),
        mono(9.5),
        c(t.dim),
    );

    legend(ui, painter, rect, t, vd, cx);
    tour_pill(ui, rect, t, vd, cx, out);
}

fn legend(ui: &mut Ui, painter: &Painter, rect: Rect, t: &Tokens, vd: &mut ViewData, cx: &Cx) {
    let mut x = rect.min.x + 18.0;
    let y = rect.max.y - 34.0;
    let items = vd.scene.legend.clone();
    for (i, (role, count)) in items.iter().enumerate() {
        let name = role_name(*role, cx.locale);
        let w = 34.0 + name.chars().count() as f32 * 6.1 + 22.0;
        let r = Rect::from_min_size(Pos2::new(x, y), egui::vec2(w, 22.0));
        let resp = ui.interact(r, Id::new(("archify-legend", i)), Sense::click());
        let active = vd.legend_role == Some(*role);
        if resp.hovered() || active {
            painter.rect_filled(r, 6.0, fade(t.panel, 0.9));
            painter.rect_stroke(
                r,
                6.0,
                Stroke::new(1.0, fade(t.stroke(*role), if active { 0.9 } else { 0.4 })),
                egui::StrokeKind::Middle,
            );
        }
        let rc = t.role(*role);
        let sw = Rect::from_min_size(Pos2::new(x + 8.0, y + 6.0), egui::vec2(16.0, 10.0));
        painter.rect(
            sw,
            2.5,
            c(rc.fill),
            Stroke::new(1.4, c(rc.stroke)),
            egui::StrokeKind::Middle,
        );
        painter.text(
            Pos2::new(x + 30.0, y + 11.0),
            Align2::LEFT_CENTER,
            name,
            mono(10.0),
            c(t.text),
        );
        let badge = Pos2::new(r.max.x - 12.0, y + 11.0);
        painter.circle_filled(badge, 7.0, c(t.mask));
        painter.circle_stroke(badge, 7.0, Stroke::new(1.0, c(rc.stroke)));
        painter.text(
            badge,
            Align2::CENTER_CENTER,
            count.to_string(),
            mono(8.0),
            c(rc.stroke),
        );
        if resp.clicked() {
            vd.legend_role = if active { None } else { Some(*role) };
        }
        x += w + 6.0;
    }
}

/// "Explore this system" pill (idle tour).
fn tour_pill(ui: &mut Ui, rect: Rect, t: &Tokens, vd: &mut ViewData, cx: &Cx, out: &mut Out) {
    if vd.tour.is_empty() || vd.tour.is_active() {
        return;
    }
    let s = strings(cx.locale);
    let ctx = ui.ctx().clone();
    let frame = Frame::new()
        .fill(c(t.panel))
        .stroke(Stroke::new(1.0, c(t.panel_border)))
        .corner_radius(10.0)
        .inner_margin(Margin::symmetric(14, 10));
    Area::new(Id::new("archify-tour-start"))
        .pivot(Align2::RIGHT_TOP)
        .fixed_pos(rect.right_top() + egui::vec2(-18.0, 16.0))
        .order(Order::Foreground)
        .show(&ctx, |ui| {
            frame.show(ui, |ui| {
                archify_egui_paint::theme::style_ui(ui, t);
                let label = RichText::new(format!("▶  {}", s.explore))
                    .color(c(t.arrow_emphasis))
                    .font(mono(12.0));
                if ui.add(egui::Button::new(label).frame(false)).clicked() {
                    vd.tour.start();
                    out.animating = true;
                }
            });
        });
}

/// Bottom bar of the running tour. A real panel, so the canvas shrinks instead of being covered
/// and the camera frames the story in the space that is left.
pub fn tour_bar(ui: &mut Ui, t: &Tokens, vd: &mut ViewData, locale: &str) {
    let Some(i) = vd.tour.current() else {
        return;
    };
    let s = strings(locale);
    let (b, st) = vd.tour_map[i];
    let beat = &vd.scene.beats[b];
    let prev = (st > 0).then(|| beat.nodes[st - 1]);
    let step = &beat.steps[st];
    let headline = step.headline(&vd.scene, prev, s.start);
    let labels = step.edge_labels.join(" · ");
    let desc = beat.description.clone();
    let title = beat.title.clone();
    let step_info = format!("{:02}/{:02}", st + 1, beat.nodes.len());
    let total = vd.tour.len();
    let progress = (i as f32 + vd.tour.progress()) / total as f32;
    egui::Panel::bottom("archify_tour")
        .frame(
            Frame::new()
                .fill(c(t.panel))
                .stroke(Stroke::new(1.0, c(t.panel_border)))
                .inner_margin(Margin::symmetric(16, 8)),
        )
        .show(ui, |ui| {
            archify_egui_paint::theme::style_ui(ui, t);
            let bar = ui.allocate_space(egui::vec2(ui.available_width(), 3.0)).1;
            ui.painter().rect_filled(bar, 1.5, c(t.panel_border));
            let mut fill = bar;
            fill.set_width(bar.width() * progress.clamp(0.0, 1.0));
            ui.painter().rect_filled(fill, 1.5, c(t.arrow_emphasis));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(s.explore)
                                .font(mono(9.0))
                                .color(c(t.arrow_emphasis)),
                        );
                        ui.label(
                            RichText::new(format!("{:02}/{:02}", i + 1, total))
                                .font(mono(9.0))
                                .color(c(t.dim)),
                        );
                    });
                    ui.label(
                        RichText::new(title)
                            .font(FontId::proportional(15.0))
                            .color(c(t.text)),
                    );
                    ui.label(
                        RichText::new(format!("{step_info} · {headline}"))
                            .font(mono(11.0))
                            .color(c(t.text)),
                    );
                    if !labels.is_empty() {
                        ui.label(RichText::new(labels).font(mono(10.0)).color(c(t.muted)));
                    }
                    if !desc.is_empty() {
                        ui.label(RichText::new(desc).font(mono(10.0)).color(c(t.faint)));
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let btn = |ui: &mut Ui, text: &str| {
                        ui.add(egui::Button::new(
                            RichText::new(text).color(c(t.text)).font(mono(12.0)),
                        ))
                    };
                    if btn(ui, &format!("■ {}", s.stop)).clicked() {
                        vd.tour.stop();
                    }
                    if btn(ui, "▶▶").clicked() {
                        vd.tour.next();
                    }
                    let play = if vd.tour.is_playing() { "⏸" } else { "▶" };
                    if btn(ui, play).clicked() {
                        vd.tour.toggle_play();
                    }
                    if btn(ui, "◀").clicked() {
                        vd.tour.prev();
                    }
                });
            });
        });
}
