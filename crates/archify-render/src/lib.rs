//! archify-render: diagrams to SVG (and a script-free HTML page around it).
//!
//! Everything goes through `archify-scene` (layout, orthogonal routing, text fitting) and
//! `archify-style` (colours), the same sources the egui canvas uses. Motion is CSS only.

pub mod architecture_svg;
mod css;
pub mod dataflow_svg;
pub mod delta_svg;
mod html;
mod icons;
mod scene_svg;
pub mod sequence_svg;
pub mod xml;

pub use architecture_svg::ArchitectureSvgRenderer;
pub use dataflow_svg::DataflowSvgRenderer;
pub use delta_svg::DeltaSvgRenderer;
pub use html::wrap_html;
pub use sequence_svg::SequenceSvgRenderer;

use archify_ir::ArchitectureDiagram;

pub struct SvgRenderer;

impl SvgRenderer {
    pub fn render_delta(delta: &archify_ir::DeltaDiagram) -> String {
        DeltaSvgRenderer::render(delta)
    }

    pub fn render_sequence(seq: &archify_ir::SequenceDiagram) -> String {
        SequenceSvgRenderer::render(seq)
    }

    pub fn render_dataflow(df: &archify_ir::DataflowDiagram) -> String {
        DataflowSvgRenderer::render(df)
    }

    pub fn render(diagram: &ArchitectureDiagram) -> String {
        ArchitectureSvgRenderer::render(diagram)
    }
}

#[cfg(test)]
mod tests;
