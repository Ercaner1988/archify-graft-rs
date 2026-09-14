//! archify-render: Standalone SVG and Neon Glow HTML exporter.

use archify_ir::{ArchitectureDiagram, VisualPreset};

pub struct SvgRenderer;

impl SvgRenderer {
    pub fn render(diagram: &ArchitectureDiagram) -> String {
        let is_rtl = diagram.meta.locale == "ar";
        let is_neon = diagram.meta.visual_preset == VisualPreset::SignalFlow;
        
        let glow_radius = if is_neon { "8px" } else { "4px" };
        let canvas_w = 900.0f32;
        let canvas_h = 650.0f32;

        let mut svg = String::new();
        svg.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\" ");
        svg.push_str(&format!("viewBox=\"0 0 {} {}\" width=\"100%\" height=\"100%\" ", canvas_w, canvas_h));
        svg.push_str("style=\"background:#020617;font-family:system-ui,-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;\"");
        if is_rtl {
            svg.push_str(" dir=\"rtl\"");
        }
        svg.push_str(">\n");

        svg.push_str("<defs>\n");
        svg.push_str("  <pattern id=\"grid\" width=\"40\" height=\"40\" patternUnits=\"userSpaceOnUse\">\n");
        svg.push_str("    <path d=\"M 40 0 L 0 0 0 40\" fill=\"none\" stroke=\"#1e293b\" stroke-width=\"0.5\"/>\n");
        svg.push_str("  </pattern>\n");
        svg.push_str("  <marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"6\" refY=\"5\" markerWidth=\"6\" markerHeight=\"6\" orient=\"auto-start-reverse\">\n");
        svg.push_str("    <path d=\"M 0 1 L 10 5 L 0 9 z\" fill=\"#64748b\"/>\n");
        svg.push_str("  </marker>\n");
        svg.push_str("</defs>\n");

        svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#020617\"/>\n");
        svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"url(#grid)\"/>\n\n");

        let text_x = if is_rtl { canvas_w - 40.0 } else { 40.0 };
        let text_anchor = if is_rtl { "end" } else { "start" };
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"40\" fill=\"#f8fafc\" font-size=\"20\" font-weight=\"bold\" text-anchor=\"{}\">{}</text>\n",
            text_x, text_anchor, diagram.meta.title
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
                    "<path d=\"M {} {} C {} {}, {} {}, {} {}\" fill=\"none\" stroke=\"#475569\" stroke-width=\"1.5\" marker-end=\"url(#arrow)\"/>\n",
                    x1, y1, x1, (y1 + y2) / 2.0, x2, (y1 + y2) / 2.0, x2, y2
                ));
            }
        }

        for comp in &diagram.components {
            let stroke = comp.role.stroke_hex();
            let fill = comp.role.fill_rgba();

            svg.push_str(&format!(
                "<g style=\"filter:drop-shadow(0 0 {} {});\">\n",
                glow_radius, stroke
            ));
            svg.push_str(&format!(
                "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.8\"/>\n",
                comp.x, comp.y, comp.width, comp.height, fill, stroke
            ));
            svg.push_str(&format!(
                "  <text x=\"{}\" y=\"{}\" fill=\"#f8fafc\" font-size=\"13\" font-weight=\"600\" text-anchor=\"middle\">{}</text>\n",
                comp.x + comp.width / 2.0, comp.y + comp.height / 2.0 - 4.0, comp.label
            ));

            if let Some(sub) = &comp.sublabel {
                svg.push_str(&format!(
                    "  <text x=\"{}\" y=\"{}\" fill=\"#94a3b8\" font-size=\"10\" text-anchor=\"middle\">{}</text>\n",
                    comp.x + comp.width / 2.0, comp.y + comp.height / 2.0 + 14.0, sub
                ));
            }

            svg.push_str("</g>\n");
        }

        svg.push_str("</svg>\n");
        svg
    }
}
