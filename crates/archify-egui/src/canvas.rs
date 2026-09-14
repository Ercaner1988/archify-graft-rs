//! Canvas rendering for cyber grid, neon connections, and interactive components.

use crate::NeonPainter;
use archify_ir::ArchitectureDiagram;
use egui::{Color32, Pos2, Rect, Stroke, Vec2};

pub struct CanvasRenderer;

impl CanvasRenderer {
    pub fn draw_grid(painter: &egui::Painter, canvas_rect: Rect, pan: Vec2, zoom: f32) {
        let bg_color = Color32::from_rgb(0x02, 0x06, 0x17);
        painter.rect_filled(canvas_rect, 0.0, bg_color);

        let grid_step = 40.0 * zoom;
        let grid_color = Color32::from_rgba_unmultiplied(0x1e, 0x29, 0x3b, 80);
        let mut x = canvas_rect.min.x + (pan.x % grid_step);
        while x < canvas_rect.max.x {
            painter.line_segment(
                [
                    Pos2::new(x, canvas_rect.min.y),
                    Pos2::new(x, canvas_rect.max.y),
                ],
                Stroke::new(1.0_f32, grid_color),
            );
            x += grid_step;
        }
        let mut y = canvas_rect.min.y + (pan.y % grid_step);
        while y < canvas_rect.max.y {
            painter.line_segment(
                [
                    Pos2::new(canvas_rect.min.x, y),
                    Pos2::new(canvas_rect.max.x, y),
                ],
                Stroke::new(1.0_f32, grid_color),
            );
            y += grid_step;
        }
    }

    pub fn draw_connections(
        painter: &egui::Painter,
        diagram: &ArchitectureDiagram,
        to_screen: impl Fn(f32, f32) -> Pos2,
        active_route: Option<&[String]>,
    ) {
        for conn in &diagram.connections {
            if let (Some(from_c), Some(to_c)) = (
                diagram.components.iter().find(|c| c.id == conn.from),
                diagram.components.iter().find(|c| c.id == conn.to),
            ) {
                let start = to_screen(from_c.x + from_c.width / 2.0, from_c.y + from_c.height);
                let end = to_screen(to_c.x + to_c.width / 2.0, to_c.y);
                let is_route_edge = active_route.is_some_and(|route| {
                    route
                        .windows(2)
                        .any(|w| w[0] == conn.from && w[1] == conn.to)
                });
                let beam_color = if is_route_edge {
                    Color32::from_rgb(0x22, 0xd3, 0xee)
                } else {
                    Color32::from_rgb(0x64, 0x74, 0x8b)
                };

                NeonPainter::paint_neon_line(painter, start, end, beam_color);
            }
        }
    }
}
