pub mod architecture_svg;
pub mod dataflow_svg;
pub mod delta_svg;
pub mod sequence_svg;
pub mod xml;

pub use architecture_svg::ArchitectureSvgRenderer;
pub use dataflow_svg::DataflowSvgRenderer;
pub use delta_svg::DeltaSvgRenderer;
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
mod tests {
    use super::*;
    use archify_ir::{Component, DiagramMeta, SemanticRole, VisualPreset};

    #[test]
    fn test_all_four_presets() {
        let mut diagram = ArchitectureDiagram {
            meta: DiagramMeta {
                title: "Preset Test".to_string(),
                subtitle: None,
                locale: "en".to_string(),
                visual_preset: VisualPreset::SignalFlow,
            },
            components: vec![Component {
                id: "c1".to_string(),
                label: "Client".to_string(),
                sublabel: None,
                role: SemanticRole::Frontend,
                x: 10.0,
                y: 10.0,
                width: 100.0,
                height: 50.0,
            }],
            connections: vec![],
            regions: vec![],
            story_beats: vec![],
        };

        // 1. SignalFlow
        let svg_signal = SvgRenderer::render(&diagram);
        assert!(svg_signal.contains("filter:drop-shadow(0 0 8px"));

        // 2. Blueprint (zero blur)
        diagram.meta.visual_preset = VisualPreset::Blueprint;
        let svg_bp = SvgRenderer::render(&diagram);
        assert!(svg_bp.contains("#0a192f"));
        assert!(!svg_bp.contains("filter:drop-shadow"));

        // 3. Editorial (warm paper)
        diagram.meta.visual_preset = VisualPreset::Editorial;
        let svg_ed = SvgRenderer::render(&diagram);
        assert!(svg_ed.contains("#f8fafc"));
        assert!(svg_ed.contains("Georgia, Cambria, serif"));
    }
}
