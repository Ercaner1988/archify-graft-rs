//! Delta diagrams: status-coloured boxes and edges, status legend.

use crate::scene_svg::{draw, LegendKind};
use archify_ir::DeltaDiagram;

pub struct DeltaSvgRenderer;

impl DeltaSvgRenderer {
    pub fn render(delta: &DeltaDiagram) -> String {
        draw(&archify_scene::delta_scene(delta), LegendKind::Status)
    }
}
