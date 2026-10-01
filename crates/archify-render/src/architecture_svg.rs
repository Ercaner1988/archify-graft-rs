//! Architecture diagrams: scene (layout + routing) drawn by the shared pipeline.

use crate::scene_svg::{draw, LegendKind};
use archify_ir::ArchitectureDiagram;
use archify_scene::Scene;

pub struct ArchitectureSvgRenderer;

impl ArchitectureSvgRenderer {
    pub fn render(diagram: &ArchitectureDiagram) -> String {
        draw(&Scene::from_architecture(diagram), LegendKind::Roles)
    }
}
