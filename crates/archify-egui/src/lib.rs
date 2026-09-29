//! archify-egui: Pure-Rust immediate mode GUI with GPU neon bloom painter.
//! Zero C dependencies. Supports optional Linux `glow` (OpenGL) backend.

pub mod app;
pub mod canvas;

use archify_ir::SemanticRole;
use egui::{Color32, Pos2, Rect, Stroke};

pub use app::ArchifyApp;
pub use canvas::CanvasRenderer;

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
            SemanticRole::Frontend => (
                Color32::from_rgb(0x22, 0xd3, 0xee),
                Color32::from_rgba_premultiplied(0x08, 0x33, 0x44, 0x66),
            ),
            SemanticRole::Backend => (
                Color32::from_rgb(0x34, 0xd3, 0x99),
                Color32::from_rgba_premultiplied(0x06, 0x4e, 0x3b, 0x66),
            ),
            SemanticRole::Database => (
                Color32::from_rgb(0xa7, 0x8b, 0xfa),
                Color32::from_rgba_premultiplied(0x4c, 0x1d, 0x95, 0x66),
            ),
            SemanticRole::Cloud => (
                Color32::from_rgb(0xfb, 0xbf, 0x24),
                Color32::from_rgba_premultiplied(0x78, 0x35, 0x0f, 0x4d),
            ),
            SemanticRole::Security => (
                Color32::from_rgb(0xfb, 0x71, 0x85),
                Color32::from_rgba_premultiplied(0x88, 0x13, 0x37, 0x66),
            ),
            SemanticRole::Messagebus => (
                Color32::from_rgb(0xfb, 0x92, 0x3c),
                Color32::from_rgba_premultiplied(0xfb, 0x92, 0x3c, 0x4d),
            ),
            SemanticRole::External => (
                Color32::from_rgb(0x94, 0xa3, 0xb8),
                Color32::from_rgba_premultiplied(0x1e, 0x29, 0x3b, 0x80),
            ),
        };

        let glow_alpha_outer = if is_hovered { 35 } else { 15 };
        let glow_alpha_mid = if is_hovered { 70 } else { 40 };

        // Layer 1: Outer diffuse bloom (8px expansion)
        let outer_rect = rect.expand(if is_hovered { 8.0_f32 } else { 5.0_f32 });
        let outer_color = Color32::from_rgba_unmultiplied(
            stroke_color.r(),
            stroke_color.g(),
            stroke_color.b(),
            glow_alpha_outer,
        );
        painter.rect_stroke(
            outer_rect,
            rounding + 4.0_f32,
            Stroke::new(6.0_f32, outer_color),
            egui::StrokeKind::Middle,
        );

        // Layer 2: Mid concentrated glow (2.5px expansion)
        let mid_rect = rect.expand(2.5_f32);
        let mid_color = Color32::from_rgba_unmultiplied(
            stroke_color.r(),
            stroke_color.g(),
            stroke_color.b(),
            glow_alpha_mid,
        );
        painter.rect_stroke(
            mid_rect,
            rounding + 2.0_f32,
            Stroke::new(3.0_f32, mid_color),
            egui::StrokeKind::Middle,
        );

        // Layer 3: Solid component body and sharp neon border
        painter.rect(
            rect,
            rounding,
            fill_color,
            Stroke::new(1.8_f32, stroke_color),
            egui::StrokeKind::Middle,
        );
    }

    /// Paints an orthogonal or curved connection line with neon bloom
    pub fn paint_neon_line(painter: &egui::Painter, start: Pos2, end: Pos2, color: Color32) {
        // Outer diffuse beam
        let diffuse_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 40);
        painter.line_segment([start, end], Stroke::new(5.0_f32, diffuse_color));

        // Core sharp line
        painter.line_segment([start, end], Stroke::new(1.8_f32, color));
    }

    /// Paints a component rectangle with differential neon status (Added, Removed, Modified, Unchanged)
    pub fn paint_delta_rect(
        painter: &egui::Painter,
        rect: Rect,
        status: archify_ir::DiffStatus,
        rounding: f32,
        is_hovered: bool,
    ) {
        let (stroke_color, fill_color) = match status {
            archify_ir::DiffStatus::Added => (
                Color32::from_rgb(0x22, 0xc5, 0x5e),
                Color32::from_rgba_premultiplied(0x05, 0x2e, 0x16, 0x66),
            ),
            archify_ir::DiffStatus::Removed => (
                Color32::from_rgb(0xf4, 0x3f, 0x5e),
                Color32::from_rgba_premultiplied(0x4c, 0x05, 0x19, 0x66),
            ),
            archify_ir::DiffStatus::Modified => (
                Color32::from_rgb(0xf5, 0x9e, 0x0b),
                Color32::from_rgba_premultiplied(0x45, 0x1a, 0x03, 0x66),
            ),
            archify_ir::DiffStatus::Unchanged => (
                Color32::from_rgb(0x64, 0x74, 0x8b),
                Color32::from_rgba_premultiplied(0x1e, 0x29, 0x3b, 0x4d),
            ),
        };

        let glow_alpha = if is_hovered { 60 } else { 30 };
        let outer_rect = rect.expand(if is_hovered { 6.0_f32 } else { 3.0_f32 });
        let outer_color = Color32::from_rgba_unmultiplied(
            stroke_color.r(),
            stroke_color.g(),
            stroke_color.b(),
            glow_alpha,
        );
        painter.rect_stroke(
            outer_rect,
            rounding + 2.0_f32,
            Stroke::new(4.0_f32, outer_color),
            egui::StrokeKind::Middle,
        );
        painter.rect(
            rect,
            rounding,
            fill_color,
            Stroke::new(1.8_f32, stroke_color),
            egui::StrokeKind::Middle,
        );
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

    pub fn route_label(
        locale: &str,
        start: Option<&str>,
        target: Option<&str>,
        active: bool,
    ) -> String {
        match locale {
            "ar" => {
                if let (Some(s), Some(t)) = (start, target) {
                    if active {
                        format!("المسار النشط: {} ➔ {}", s, t)
                    } else {
                        format!("لا يوجد مسار بين {} و {}", s, t)
                    }
                } else if let Some(s) = start {
                    format!("البداية: {}. اختر الهدف...", s)
                } else {
                    "انقر على عقدتين لتتبع المسار المباشر".to_string()
                }
            }
            "tr" => {
                if let (Some(s), Some(t)) = (start, target) {
                    if active {
                        format!("Aktif Rota: {} ➔ {}", s, t)
                    } else {
                        format!("{} ile {} arasında rota bulunamadı", s, t)
                    }
                } else if let Some(s) = start {
                    format!("Başlangıç: {}. Hedef düğümü seçin...", s)
                } else {
                    "Canlı rota takibi için iki düğüme tıklayın".to_string()
                }
            }
            _ => {
                if let (Some(s), Some(t)) = (start, target) {
                    if active {
                        format!("Active Route: {} ➔ {}", s, t)
                    } else {
                        format!("No route between {} and {}", s, t)
                    }
                } else if let Some(s) = start {
                    format!("Start: {}. Select target...", s)
                } else {
                    "Click two nodes to probe shortest route".to_string()
                }
            }
        }
    }
}

#[cfg(any(feature = "glow", feature = "wgpu"))]
pub fn run_desktop(
    diagram: Option<archify_ir::ArchitectureDiagram>,
    locale: &str,
) -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title(TrilingualUi::title(locale)),
        ..Default::default()
    };
    let loc = locale.to_string();
    eframe::run_native(
        TrilingualUi::title(locale),
        native_options,
        Box::new(move |_cc| Ok(Box::new(ArchifyApp::new(diagram, &loc)))),
    )
}
