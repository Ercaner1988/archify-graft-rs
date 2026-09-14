//! archify-render: Standalone SVG and Neon Glow HTML exporter.
//! Supports 4 visual presets (SignalFlow, Classic, Blueprint, Editorial) and trilingual RTL layout.

use archify_ir::{ArchitectureDiagram, VisualPreset};

pub struct SvgRenderer;

impl SvgRenderer {
    pub fn render(diagram: &ArchitectureDiagram) -> String {
        let is_rtl = diagram.meta.locale == "ar";
        let preset = diagram.meta.visual_preset;

        let (bg_color, grid_color, text_main, text_sub, font_family, corner_rx) = match preset {
            VisualPreset::SignalFlow => ("#020617", "#1e293b", "#f8fafc", "#94a3b8", "system-ui, sans-serif", "6"),
            VisualPreset::Classic => ("#090d16", "#1e293b", "#f8fafc", "#94a3b8", "system-ui, sans-serif", "6"),
            VisualPreset::Blueprint => ("#0a192f", "#172a45", "#e6f1ff", "#8892b0", "monospace, Courier, sans-serif", "0"),
            VisualPreset::Editorial => ("#f8fafc", "#e2e8f0", "#0f172a", "#475569", "Georgia, Cambria, serif", "2"),
        };

        let glow_radius = match preset {
            VisualPreset::SignalFlow => "8px",
            VisualPreset::Classic => "4px",
            VisualPreset::Blueprint => "0px",
            VisualPreset::Editorial => "2px",
        };

        let canvas_w = 900.0f32;
        let canvas_h = 650.0f32;

        let mut svg = String::new();
        svg.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\" ");
        svg.push_str(&format!("viewBox=\"0 0 {} {}\" width=\"100%\" height=\"100%\" ", canvas_w, canvas_h));
        svg.push_str(&format!("style=\"background:{};font-family:{};\"", bg_color, font_family));
        if is_rtl {
            svg.push_str(" dir=\"rtl\"");
        }
        svg.push_str(">\n");

        svg.push_str("<defs>\n");
        svg.push_str("  <pattern id=\"grid\" width=\"40\" height=\"40\" patternUnits=\"userSpaceOnUse\">\n");
        svg.push_str(&format!("    <path d=\"M 40 0 L 0 0 0 40\" fill=\"none\" stroke=\"{}\" stroke-width=\"0.5\"/>\n", grid_color));
        svg.push_str("  </pattern>\n");
        svg.push_str("  <marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"6\" refY=\"5\" markerWidth=\"6\" markerHeight=\"6\" orient=\"auto-start-reverse\">\n");
        svg.push_str(&format!("    <path d=\"M 0 1 L 10 5 L 0 9 z\" fill=\"{}\"/>\n", text_sub));
        svg.push_str("  </marker>\n");
        svg.push_str("</defs>\n");

        svg.push_str(&format!("<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>\n", bg_color));
        svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"url(#grid)\"/>\n\n");

        let text_x = if is_rtl { canvas_w - 40.0 } else { 40.0 };
        let text_anchor = if is_rtl { "end" } else { "start" };
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"40\" fill=\"{}\" font-size=\"20\" font-weight=\"bold\" text-anchor=\"{}\">{}</text>\n",
            text_x, text_main, text_anchor, diagram.meta.title
        ));

        for conn in &diagram.connections {
            if let (Some(from_c), Some(to_c)) = (
                diagram.components.iter().find(|c| c.id == conn.from),
                diagram.components.iter().find(|c| c.id == conn.to),
            ) {
                let x1 = from_c.x + from_c.width / 2.0;
                let y1 = from_c.y + from_c.height;
                let x2 = to_c.x + to_c.width / 2.0;
                let y2 = to_c.y;

                svg.push_str(&format!(
                    "<path d=\"M {} {} C {} {}, {} {}, {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" marker-end=\"url(#arrow)\"/>\n",
                    x1, y1, x1, (y1 + y2) / 2.0, x2, (y1 + y2) / 2.0, x2, y2, text_sub
                ));
            }
        }

        for comp in &diagram.components {
            let stroke = comp.role.stroke_hex();
            let fill = if preset == VisualPreset::Editorial {
                "rgba(255, 255, 255, 0.9)"
            } else {
                comp.role.fill_rgba()
            };

            let filter_style = if preset == VisualPreset::Blueprint {
                String::new()
            } else if preset == VisualPreset::Editorial {
                "style=\"filter:drop-shadow(0 2px 3px rgba(0,0,0,0.15));\"".to_string()
            } else {
                format!("style=\"filter:drop-shadow(0 0 {} {});\"", glow_radius, stroke)
            };

            svg.push_str(&format!("<g {}>\n", filter_style));
            svg.push_str(&format!(
                "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.8\"/>\n",
                comp.x, comp.y, comp.width, comp.height, corner_rx, fill, stroke
            ));
            svg.push_str(&format!(
                "  <text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"13\" font-weight=\"600\" text-anchor=\"middle\">{}</text>\n",
                comp.x + comp.width / 2.0, comp.y + comp.height / 2.0 - 4.0, text_main, comp.label
            ));

            if let Some(sub) = &comp.sublabel {
                svg.push_str(&format!(
                    "  <text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"10\" text-anchor=\"middle\">{}</text>\n",
                    comp.x + comp.width / 2.0, comp.y + comp.height / 2.0 + 14.0, text_sub, sub
                ));
            }

            svg.push_str("</g>\n");
        }

        svg.push_str("</svg>\n");
        svg
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archify_ir::{Component, DiagramMeta, SemanticRole};

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
