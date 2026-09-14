//! Interactive Archify & Graft Desktop Studio Application using egui.

use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};
use archify_ir::{ArchitectureDiagram, VisualPreset};
use crate::{NeonPainter, TrilingualUi};

pub struct ArchifyApp {
    pub diagram: Option<ArchitectureDiagram>,
    pub pan: Vec2,
    pub zoom: f32,
    pub selected_id: Option<String>,
    pub locale: String,
    pub preset: VisualPreset,
    pub search_text: String,
}

impl Default for ArchifyApp {
    fn default() -> Self {
        Self {
            diagram: None,
            pan: Vec2::new(50.0, 50.0),
            zoom: 1.0,
            selected_id: None,
            locale: "tr".to_string(),
            preset: VisualPreset::SignalFlow,
            search_text: String::new(),
        }
    }
}

impl ArchifyApp {
    pub fn new(diagram: Option<ArchitectureDiagram>, locale: &str) -> Self {
        Self {
            diagram,
            locale: locale.to_string(),
            ..Default::default()
        }
    }

    /// Primary UI rendering function called every frame by eframe/egui
    pub fn render_ui(&mut self, ctx: &egui::Context) {
        // Top Toolbar
        egui::TopBottomPanel::top("archify_toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(TrilingualUi::title(&self.locale));
                ui.separator();

                // Language Switcher
                ui.label("🌐");
                if ui.selectable_label(self.locale == "tr", "Türkçe").clicked() {
                    self.locale = "tr".to_string();
                }
                if ui.selectable_label(self.locale == "ar", "العربية").clicked() {
                    self.locale = "ar".to_string();
                }
                if ui.selectable_label(self.locale == "en", "English").clicked() {
                    self.locale = "en".to_string();
                }

                ui.separator();

                // Preset Selector
                if ui.selectable_label(self.preset == VisualPreset::SignalFlow, "⚡ Neon").clicked() {
                    self.preset = VisualPreset::SignalFlow;
                }
                if ui.selectable_label(self.preset == VisualPreset::Classic, "🌙 Classic").clicked() {
                    self.preset = VisualPreset::Classic;
                }
                if ui.selectable_label(self.preset == VisualPreset::Blueprint, "📐 Blueprint").clicked() {
                    self.preset = VisualPreset::Blueprint;
                }

                ui.separator();

                // Zoom controls
                if ui.button("🔍 100%").clicked() {
                    self.zoom = 1.0;
                    self.pan = Vec2::new(50.0, 50.0);
                }
                ui.label(format!("{:.0}%", self.zoom * 100.0));

                ui.separator();
                ui.text_edit_singleline(&mut self.search_text);
            });
        });

        // Central Interactive Canvas
        egui::CentralPanel::default().show(ctx, |ui| {
            let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());

            // Handle Drag Pan
            if response.dragged() {
                self.pan += response.drag_delta();
            }

            // Handle Scroll Zoom
            let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
            if scroll_delta != 0.0 {
                let zoom_factor = if scroll_delta > 0.0 { 1.1 } else { 0.9 };
                self.zoom = (self.zoom * zoom_factor).clamp(0.2, 4.0);
            }

            // Background Cyber Grid
            let canvas_rect = response.rect;
            let bg_color = Color32::from_rgb(0x02, 0x06, 0x17);
            painter.rect_filled(canvas_rect, 0.0, bg_color);

            let grid_step = 40.0 * self.zoom;
            let grid_color = Color32::from_rgba_unmultiplied(0x1e, 0x29, 0x3b, 80);
            let mut x = canvas_rect.min.x + (self.pan.x % grid_step);
            while x < canvas_rect.max.x {
                painter.line_segment([Pos2::new(x, canvas_rect.min.y), Pos2::new(x, canvas_rect.max.y)], Stroke::new(1.0_f32, grid_color));
                x += grid_step;
            }
            let mut y = canvas_rect.min.y + (self.pan.y % grid_step);
            while y < canvas_rect.max.y {
                painter.line_segment([Pos2::new(canvas_rect.min.x, y), Pos2::new(canvas_rect.max.x, y)], Stroke::new(1.0_f32, grid_color));
                y += grid_step;
            }

            // Render Diagram Elements
            if let Some(diagram) = &self.diagram {
                let to_screen = |x: f32, y: f32| -> Pos2 {
                    Pos2::new(
                        canvas_rect.min.x + self.pan.x + x * self.zoom,
                        canvas_rect.min.y + self.pan.y + y * self.zoom,
                    )
                };

                // 1. Connections
                for conn in &diagram.connections {
                    if let (Some(from_c), Some(to_c)) = (
                        diagram.components.iter().find(|c| c.id == conn.from),
                        diagram.components.iter().find(|c| c.id == conn.to),
                    ) {
                        let start = to_screen(from_c.x + from_c.width / 2.0, from_c.y + from_c.height);
                        let end = to_screen(to_c.x + to_c.width / 2.0, to_c.y);
                        let beam_color = Color32::from_rgb(0x64, 0x74, 0x8b);

                        NeonPainter::paint_neon_line(&painter, start, end, beam_color);
                    }
                }

                // 2. Components
                let pointer_pos = ctx.input(|i| i.pointer.hover_pos());
                let mut newly_selected = None;

                for comp in &diagram.components {
                    // Filter search if typed
                    if !self.search_text.is_empty() && !comp.label.to_lowercase().contains(&self.search_text.to_lowercase()) {
                        continue;
                    }

                    let min_pos = to_screen(comp.x, comp.y);
                    let size = Vec2::new(comp.width * self.zoom, comp.height * self.zoom);
                    let rect = Rect::from_min_size(min_pos, size);

                    let is_hovered = pointer_pos.map_or(false, |pos| rect.contains(pos));
                    if is_hovered && response.clicked() {
                        newly_selected = Some(comp.id.clone());
                    }

                    let is_selected = self.selected_id.as_deref() == Some(&comp.id);

                    // Draw neon box
                    NeonPainter::paint_neon_rect(&painter, rect, comp.role, 6.0 * self.zoom, is_hovered || is_selected);

                    // Text labels
                    let text_color = Color32::from_rgb(0xf8, 0xfa, 0xfc);
                    let font_size = 13.0 * self.zoom;
                    painter.text(
                        Pos2::new(rect.center().x, rect.center().y - 4.0 * self.zoom),
                        egui::Align2::CENTER_CENTER,
                        &comp.label,
                        egui::FontId::proportional(font_size),
                        text_color,
                    );

                    if let Some(sub) = &comp.sublabel {
                        painter.text(
                            Pos2::new(rect.center().x, rect.center().y + 10.0 * self.zoom),
                            egui::Align2::CENTER_CENTER,
                            sub,
                            egui::FontId::proportional(font_size * 0.75),
                            Color32::from_rgb(0x94, 0xa3, 0xb8),
                        );
                    }
                }

                if let Some(sel) = newly_selected {
                    self.selected_id = Some(sel);
                }
            }
        });
    }
}
