//! Bridges `archify-style` tokens to egui colours, plus the role sigils.

use archify_ir::SemanticRole;
use archify_style::Rgba;
use egui::{Color32, Pos2, Rect, Stroke};

pub fn c(x: Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(x.r, x.g, x.b, x.a)
}

/// Colour with its alpha scaled by `k`.
pub fn fade(x: Rgba, k: f32) -> Color32 {
    c(x.scale_alpha(k))
}

/// Open polylines on a 16x16 grid, one set per role (the original's "semantic sigil").
pub fn sigil(role: SemanticRole) -> &'static [&'static [[f32; 2]]] {
    match role {
        SemanticRole::Frontend => &[
            &[
                [2.0, 3.0],
                [14.0, 3.0],
                [14.0, 13.0],
                [2.0, 13.0],
                [2.0, 3.0],
            ],
            &[[2.0, 6.0], [14.0, 6.0]],
        ],
        SemanticRole::Backend => &[
            &[[6.0, 4.0], [2.0, 8.0], [6.0, 12.0]],
            &[[10.0, 4.0], [14.0, 8.0], [10.0, 12.0]],
            &[[9.0, 3.0], [7.0, 13.0]],
        ],
        SemanticRole::Database => &[
            &[[3.0, 4.0], [8.0, 2.5], [13.0, 4.0], [8.0, 5.5], [3.0, 4.0]],
            &[
                [3.0, 4.0],
                [3.0, 12.0],
                [8.0, 13.5],
                [13.0, 12.0],
                [13.0, 4.0],
            ],
            &[[3.0, 8.0], [8.0, 9.5], [13.0, 8.0]],
        ],
        SemanticRole::Cloud => &[&[
            [4.0, 12.0],
            [3.0, 9.0],
            [5.0, 7.0],
            [7.0, 5.0],
            [10.0, 5.0],
            [12.0, 7.0],
            [14.0, 9.0],
            [13.0, 12.0],
            [4.0, 12.0],
        ]],
        SemanticRole::Security => &[&[
            [8.0, 2.0],
            [13.0, 4.0],
            [13.0, 8.0],
            [8.0, 14.0],
            [3.0, 8.0],
            [3.0, 4.0],
            [8.0, 2.0],
        ]],
        SemanticRole::Messagebus => &[
            &[[2.0, 4.0], [14.0, 4.0]],
            &[[2.0, 8.0], [14.0, 8.0]],
            &[[2.0, 12.0], [14.0, 12.0]],
        ],
        SemanticRole::External => &[
            &[[6.0, 3.0], [13.0, 3.0], [13.0, 10.0]],
            &[[13.0, 3.0], [3.0, 13.0]],
        ],
    }
}

/// Draws the sigil of `role` inside `rect` (an 11px square at zoom 1).
pub fn paint_sigil(painter: &egui::Painter, role: SemanticRole, rect: Rect, stroke: Stroke) {
    let k = rect.width() / 16.0;
    for line in sigil(role) {
        let pts: Vec<Pos2> = line
            .iter()
            .map(|p| Pos2::new(rect.min.x + p[0] * k, rect.min.y + p[1] * k))
            .collect();
        painter.add(egui::Shape::line(pts, stroke));
    }
}

/// Scopes egui's visuals inside `ui` to the preset: panel colours, widget fills, selection.
/// Only the given `Ui` is touched, so an embedding application keeps its own look.
pub fn style_ui(ui: &mut egui::Ui, t: &archify_style::Tokens) {
    let v = ui.visuals_mut();
    *v = if t.is_dark() {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    v.override_text_color = Some(c(t.text));
    v.panel_fill = c(t.panel);
    v.extreme_bg_color = c(t.mask);
    v.selection.bg_fill = fade(t.arrow_emphasis, 0.35);
    v.selection.stroke = Stroke::new(1.0, c(t.arrow_emphasis));
    let plate = c(t.panel_border.scale_alpha(0.55));
    let hover = c(t.panel_border);
    v.widgets.inactive.weak_bg_fill = plate;
    v.widgets.inactive.bg_fill = plate;
    v.widgets.hovered.weak_bg_fill = hover;
    v.widgets.hovered.bg_fill = hover;
    v.widgets.active.weak_bg_fill = hover;
    v.widgets.active.bg_fill = hover;
    v.widgets.open.weak_bg_fill = plate;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, c(t.panel_border));
}
