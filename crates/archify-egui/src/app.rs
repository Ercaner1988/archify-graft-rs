//! Interactive Archify & Graft Desktop Studio Application using egui.

use crate::{CanvasRenderer, NeonPainter, TrilingualUi};
use archify_ir::{ArchitectureDiagram, DataflowDiagram, VisualPreset};
use egui::{Color32, Pos2, Rect, Sense, Vec2};

/// Which diagram the central canvas currently draws. Independent layouts
/// (grid vs. force-free component placement), so pan/zoom resets on switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiagramView {
    #[default]
    Architecture,
    Dataflow,
}

pub struct ArchifyApp {
    pub diagram: Option<ArchitectureDiagram>,
    pub dataflow: Option<DataflowDiagram>,
    pub view: DiagramView,
    pub pan: Vec2,
    pub zoom: f32,
    pub selected_id: Option<String>,
    pub route_start: Option<String>,
    pub route_target: Option<String>,
    pub active_route: Option<Vec<String>>,
    pub active_story_beat: Option<usize>,
    pub locale: String,
    pub preset: VisualPreset,
    pub search_text: String,
}

impl Default for ArchifyApp {
    fn default() -> Self {
        Self {
            diagram: None,
            dataflow: None,
            view: DiagramView::default(),
            pan: Vec2::new(50.0, 50.0),
            zoom: 1.0,
            selected_id: None,
            route_start: None,
            route_target: None,
            active_route: None,
            active_story_beat: None,
            locale: "tr".to_string(),
            preset: VisualPreset::SignalFlow,
            search_text: String::new(),
        }
    }
}

impl ArchifyApp {
    pub fn new(
        diagram: Option<ArchitectureDiagram>,
        dataflow: Option<DataflowDiagram>,
        locale: &str,
    ) -> Self {
        Self {
            diagram,
            dataflow,
            locale: locale.to_string(),
            ..Default::default()
        }
    }

    /// Primary UI rendering function called every frame by eframe/egui
    pub fn render_ui(&mut self, ui: &mut egui::Ui) {
        // Bottom Status Bar with Route Diagnostics & Story Beat Navigator
        egui::Panel::bottom("archify_statusbar").show(ui, |ui| {
            ui.horizontal(|ui| {
                let status = TrilingualUi::route_label(
                    &self.locale,
                    self.route_start.as_deref(),
                    self.route_target.as_deref(),
                    self.active_route.is_some(),
                );
                ui.label(status);
                if self.route_start.is_some() && ui.button("✖").clicked() {
                    self.route_start = None;
                    self.route_target = None;
                    self.active_route = None;
                }

                if let Some(diag) = &self.diagram {
                    if !diag.story_beats.is_empty() {
                        ui.separator();
                        ui.label("📖 Story:");
                        if ui.button("◀").clicked() {
                            let curr = self.active_story_beat.unwrap_or(0);
                            if curr > 0 {
                                self.active_story_beat = Some(curr - 1);
                            }
                        }
                        if let Some(idx) = self.active_story_beat {
                            if let Some(beat) = diag.story_beats.get(idx) {
                                ui.colored_label(Color32::from_rgb(0x38, 0xbd, 0xf8), &beat.title);
                            }
                        } else {
                            ui.label("Overview");
                        }
                        if ui.button("▶").clicked() {
                            let curr = self.active_story_beat.map_or(0, |c| c + 1);
                            if curr < diag.story_beats.len() {
                                self.active_story_beat = Some(curr);
                            }
                        }
                        if self.active_story_beat.is_some() && ui.button("⏹").clicked() {
                            self.active_story_beat = None;
                        }
                    }
                }
            });
        });

        // Top Toolbar
        egui::Panel::top("archify_toolbar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(TrilingualUi::title(&self.locale));
                ui.separator();

                // Language Switcher
                ui.label("🌐");
                if ui.selectable_label(self.locale == "tr", "Türkçe").clicked() {
                    self.locale = "tr".to_string();
                }
                if ui
                    .selectable_label(self.locale == "ar", "العربية")
                    .clicked()
                {
                    self.locale = "ar".to_string();
                }
                if ui
                    .selectable_label(self.locale == "en", "English")
                    .clicked()
                {
                    self.locale = "en".to_string();
                }

                ui.separator();

                // Preset Selector
                if ui
                    .selectable_label(self.preset == VisualPreset::SignalFlow, "⚡ Neon")
                    .clicked()
                {
                    self.preset = VisualPreset::SignalFlow;
                }
                if ui
                    .selectable_label(self.preset == VisualPreset::Classic, "🌙 Classic")
                    .clicked()
                {
                    self.preset = VisualPreset::Classic;
                }
                if ui
                    .selectable_label(self.preset == VisualPreset::Blueprint, "📐 Blueprint")
                    .clicked()
                {
                    self.preset = VisualPreset::Blueprint;
                }

                ui.separator();

                // View switcher — only when a dataflow diagram was actually
                // compiled (caller may pass None, e.g. archify-graft-cli's
                // own `gui` command when the graph is empty).
                if self.dataflow.is_some() {
                    if ui
                        .selectable_label(self.view == DiagramView::Architecture, "🏗 Architecture")
                        .clicked()
                    {
                        self.view = DiagramView::Architecture;
                        self.pan = Vec2::new(50.0, 50.0);
                        self.zoom = 1.0;
                    }
                    if ui
                        .selectable_label(self.view == DiagramView::Dataflow, "🌊 Dataflow")
                        .clicked()
                    {
                        self.view = DiagramView::Dataflow;
                        self.pan = Vec2::new(50.0, 50.0);
                        self.zoom = 1.0;
                    }
                    ui.separator();
                }

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
        egui::CentralPanel::default().show(ui, |ui| {
            let (response, painter) =
                ui.allocate_painter(ui.available_size(), Sense::click_and_drag());

            if response.dragged() {
                self.pan += response.drag_delta();
            }

            let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll_delta != 0.0 {
                let zoom_factor = if scroll_delta > 0.0 { 1.1 } else { 0.9 };
                self.zoom = (self.zoom * zoom_factor).clamp(0.2, 4.0);
            }

            let canvas_rect = response.rect;
            CanvasRenderer::draw_grid(&painter, canvas_rect, self.pan, self.zoom);

            if self.view == DiagramView::Dataflow {
                self.render_dataflow(ui, &painter, canvas_rect);
                return;
            }

            if let Some(diagram) = &self.diagram {
                let to_screen = |x: f32, y: f32| -> Pos2 {
                    Pos2::new(
                        canvas_rect.min.x + self.pan.x + x * self.zoom,
                        canvas_rect.min.y + self.pan.y + y * self.zoom,
                    )
                };

                CanvasRenderer::draw_connections(
                    &painter,
                    diagram,
                    to_screen,
                    self.active_route.as_deref(),
                );

                let pointer_pos = ui.input(|i| i.pointer.hover_pos());
                let mut newly_selected = None;

                for comp in &diagram.components {
                    if !self.search_text.is_empty()
                        && !comp
                            .label
                            .to_lowercase()
                            .contains(&self.search_text.to_lowercase())
                    {
                        continue;
                    }

                    let min_pos = to_screen(comp.x, comp.y);
                    let size = Vec2::new(comp.width * self.zoom, comp.height * self.zoom);
                    let rect = Rect::from_min_size(min_pos, size);

                    let is_hovered = pointer_pos.is_some_and(|pos| rect.contains(pos));
                    if is_hovered && response.clicked() {
                        newly_selected = Some(comp.id.clone());
                        if self.route_start.is_none() {
                            self.route_start = Some(comp.id.clone());
                            self.route_target = None;
                            self.active_route = None;
                        } else if self.route_start.as_deref() == Some(&comp.id) {
                            self.route_start = None;
                            self.route_target = None;
                            self.active_route = None;
                        } else {
                            self.route_target = Some(comp.id.clone());
                            self.active_route = archify_geometry::ReachabilityEngine::find_route(
                                diagram,
                                self.route_start.as_ref().unwrap(),
                                &comp.id,
                            );
                        }
                    }

                    let is_selected = self.selected_id.as_deref() == Some(&comp.id);
                    let is_in_route = self
                        .active_route
                        .as_ref()
                        .is_some_and(|r| r.contains(&comp.id))
                        || self.route_start.as_deref() == Some(&comp.id);

                    let is_beat_active = self
                        .active_story_beat
                        .and_then(|idx| {
                            diagram
                                .story_beats
                                .get(idx)
                                .map(|b| b.highlighted_nodes.contains(&comp.id))
                        })
                        .unwrap_or(true);

                    NeonPainter::paint_neon_rect(
                        &painter,
                        rect,
                        comp.role,
                        6.0 * self.zoom,
                        (is_hovered || is_selected || is_in_route) && is_beat_active,
                    );

                    let text_alpha = if is_beat_active { 255 } else { 70 };
                    let text_color = Color32::from_rgba_unmultiplied(0xf8, 0xfa, 0xfc, text_alpha);
                    let font_size = 13.0 * self.zoom;
                    painter.text(
                        Pos2::new(rect.center().x, rect.center().y - 4.0 * self.zoom),
                        egui::Align2::CENTER_CENTER,
                        &comp.label,
                        egui::FontId::proportional(font_size),
                        text_color,
                    );

                    if let Some(sub) = &comp.sublabel {
                        let sub_color =
                            Color32::from_rgba_unmultiplied(0x94, 0xa3, 0xb8, text_alpha);
                        painter.text(
                            Pos2::new(rect.center().x, rect.center().y + 10.0 * self.zoom),
                            egui::Align2::CENTER_CENTER,
                            sub,
                            egui::FontId::proportional(font_size * 0.75),
                            sub_color,
                        );
                    }
                }

                if let Some(sel) = newly_selected {
                    self.selected_id = Some(sel);
                }
            }
        });
    }

    /// Dataflow canvas: mirrors `archify_render::DataflowSvgRenderer`'s grid
    /// layout exactly (same constants, same index-order placement) since
    /// `DataflowNode` carries no position — only this renderer computes one.
    /// No route/story-beat/search interaction (the SVG export has none
    /// either); node hover glow is the only affordance, matching the bar the
    /// static export already sets.
    fn render_dataflow(&mut self, ui: &mut egui::Ui, painter: &egui::Painter, canvas_rect: Rect) {
        let Some(df) = &self.dataflow else {
            return;
        };

        const CELL_W: f32 = 160.0;
        const CELL_H: f32 = 70.0;
        const GAP_X: f32 = 70.0;
        const GAP_Y: f32 = 60.0;
        const COLS: usize = 3;

        let to_screen = |x: f32, y: f32| -> Pos2 {
            Pos2::new(
                canvas_rect.min.x + self.pan.x + x * self.zoom,
                canvas_rect.min.y + self.pan.y + y * self.zoom,
            )
        };

        let mut positions: std::collections::HashMap<&str, (f32, f32)> =
            std::collections::HashMap::with_capacity(df.nodes.len());
        for (i, node) in df.nodes.iter().enumerate() {
            let col = (i % COLS) as f32;
            let row = (i / COLS) as f32;
            positions.insert(
                node.id.as_str(),
                (
                    60.0 + col * (CELL_W + GAP_X),
                    100.0 + row * (CELL_H + GAP_Y),
                ),
            );
        }

        for pipe in &df.pipelines {
            let (Some(&(x1, y1)), Some(&(x2, y2))) = (
                positions.get(pipe.from.as_str()),
                positions.get(pipe.to.as_str()),
            ) else {
                continue;
            };
            let start = to_screen(x1 + CELL_W, y1 + CELL_H / 2.0);
            let end = to_screen(x2, y2 + CELL_H / 2.0);
            NeonPainter::paint_neon_line(painter, start, end, Color32::from_rgb(0x38, 0xbd, 0xf8));
            if let Some(throughput) = &pipe.throughput {
                painter.text(
                    Pos2::new(
                        (start.x + end.x) / 2.0,
                        (start.y + end.y) / 2.0 - 6.0 * self.zoom,
                    ),
                    egui::Align2::CENTER_CENTER,
                    throughput,
                    egui::FontId::proportional(9.0 * self.zoom),
                    Color32::from_rgb(0x94, 0xa3, 0xb8),
                );
            }
        }

        let pointer_pos = ui.input(|i| i.pointer.hover_pos());
        for node in &df.nodes {
            let Some(&(x, y)) = positions.get(node.id.as_str()) else {
                continue;
            };
            let min_pos = to_screen(x, y);
            let size = Vec2::new(CELL_W * self.zoom, CELL_H * self.zoom);
            let rect = Rect::from_min_size(min_pos, size);
            let is_hovered = pointer_pos.is_some_and(|pos| rect.contains(pos));
            NeonPainter::paint_neon_rect(painter, rect, node.role, 8.0 * self.zoom, is_hovered);

            let font_size = 13.0 * self.zoom;
            painter.text(
                Pos2::new(rect.center().x, rect.center().y - 4.0 * self.zoom),
                egui::Align2::CENTER_CENTER,
                &node.label,
                egui::FontId::proportional(font_size),
                Color32::from_rgb(0xf8, 0xfa, 0xfc),
            );
            if let Some(rate) = &node.stream_rate {
                painter.text(
                    Pos2::new(rect.center().x, rect.center().y + 12.0 * self.zoom),
                    egui::Align2::CENTER_CENTER,
                    format!("⚡ {rate}"),
                    egui::FontId::proportional(font_size * 0.75),
                    Color32::from_rgb(0x38, 0xbd, 0xf8),
                );
            }
        }
    }
}

#[cfg(any(feature = "glow", feature = "wgpu"))]
impl eframe::App for ArchifyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.render_ui(ui);
    }
}
