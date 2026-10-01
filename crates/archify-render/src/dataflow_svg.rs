//! Dataflow diagrams: layered placement, routed pipes, same drawing pipeline.

use crate::scene_svg::{draw, LegendKind};
use archify_ir::DataflowDiagram;

pub struct DataflowSvgRenderer;

impl DataflowSvgRenderer {
    pub fn render(df: &DataflowDiagram) -> String {
        draw(&archify_scene::dataflow_scene(df), LegendKind::Roles)
    }
}
