//! archify-egui: Pure-Rust immediate mode GUI with GPU neon bloom painter.
//! Zero C dependencies. Supports optional Linux `glow` (OpenGL) backend.

pub mod app;

use egui::{Color32, Pos2, Rect, Stroke};
use archify_ir::SemanticRole;

pub use app::ArchifyApp;

pub struct NeonPainter;

impl NeonPainter {
    /// Paints an additive multi-layer neon glow rectangle around a component
    pub fn paint_neon_rect(
        painter: &egui::Painter,
        rect: Rect,
        role: SemanticRole,
        rounding: f32,
        is_hovered: bool,
    ) {
        let (stroke_color, fill_color) = match role {
            SemanticRole::Frontend => (Color32::from_rgb(0x22, 0xd3, 0xee), Color32::from_rgba_premultiplied(0x08, 0x33, 0x44, 0x66)),
            SemanticRole::Backend => (Color32::from_rgb(0x34, 0xd3, 0x99), Color32::from_rgba_premultiplied(0x06, 0x4e, 0x3b, 0x66)),
            SemanticRole::Database => (Color32::from_rgb(0xa7, 0x8b, 0xfa), Color32::from_rgba_premultiplied(0x4c, 0x1d, 0x95, 0x66)),
            SemanticRole::Cloud => (Color32::from_rgb(0xfb, 0xbf, 0x24), Color32::from_rgba_premultiplied(0x78, 0x35, 0x0f, 0x4d)),
            SemanticRole::Security => (Color32::from_rgb(0xfb, 0x71, 0x85), Color32::from_rgba_premultiplied(0x88, 0x13, 0x37, 0x66)),
            SemanticRole::Messagebus => (Color32::from_rgb(0xfb, 0x92, 0x3c), Color32::from_rgba_premultiplied(0xfb, 0x92, 0x3c, 0x4d)),
            SemanticRole::External => (Color32::from_rgb(0x94, 0xa3, 0xb8), Color32::from_rgba_premultiplied(0x1e, 0x29, 0x3b, 0x80)),
        };

        let glow_alpha_outer = if is_hovered { 35 } else { 15 };
        let glow_alpha_mid = if is_hovered { 70 } else { 40 };

        // Layer 1: Outer diffuse bloom (8px expansion)
        let outer_rect = rect.expand(if is_hovered { 8.0_f32 } else { 5.0_f32 });
        let outer_color = Color32::from_rgba_unmultiplied(stroke_color.r(), stroke_color.g(), stroke_color.b(), glow_alpha_outer);
        painter.rect_stroke(outer_rect, rounding + 4.0_f32, Stroke::new(6.0_f32, outer_color));

        // Layer 2: Mid concentrated glow (2.5px expansion)
        let mid_rect = rect.expand(2.5_f32);
        let mid_color = Color32::from_rgba_unmultiplied(stroke_color.r(), stroke_color.g(), stroke_color.b(), glow_alpha_mid);
        painter.rect_stroke(mid_rect, rounding + 2.0_f32, Stroke::new(3.0_f32, mid_color));

        // Layer 3: Solid component body and sharp neon border
        painter.rect(rect, rounding, fill_color, Stroke::new(1.8_f32, stroke_color));
    }

    /// Paints an orthogonal or curved connection line with neon bloom
    pub fn paint_neon_line(
        painter: &egui::Painter,
        start: Pos2,
        end: Pos2,
        color: Color32,
    ) {
        // Outer diffuse beam
        let diffuse_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 40);
        painter.line_segment([start, end], Stroke::new(5.0_f32, diffuse_color));

        // Core sharp line
        painter.line_segment([start, end], Stroke::new(1.8_f32, color));
    }
}

pub struct TrilingualUi;

impl TrilingualUi {
    pub fn title(locale: &str) -> &'static str {
        match locale {
            "ar" => "استوديو بنية النظام (Archify & Graft)",
            "tr" => "Archify Mimari Stüdyosu & Graft Kod Analitiği",
            _ => "Archify Architecture Studio & Graft Code Intelligence",
        }
    }

    pub fn search_label(locale: &str) -> &'static str {
        match locale {
            "ar" => "ابحث في الكود أو البنية...",
            "tr" => "Kod veya mimaride ara...",
            _ => "Search code or architecture...",
        }
    }
}
