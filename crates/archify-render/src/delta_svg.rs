//! archify-render: Delta Analysis SVG renderer with neon green/rose/amber diff bloom.

use archify_ir::{DeltaDiagram, DiffStatus};

pub struct DeltaSvgRenderer;

impl DeltaSvgRenderer {
    pub fn render(delta: &DeltaDiagram) -> String {
        let is_rtl = delta.meta.locale == "ar";
        let canvas_w = 900.0f32;
        let canvas_h = 650.0f32;

        let mut svg = String::new();
        svg.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\" ");
        svg.push_str(&format!(
            "viewBox=\"0 0 {} {}\" width=\"100%\" height=\"100%\" ",
            canvas_w, canvas_h
        ));
        svg.push_str("style=\"background:#020617;font-family:system-ui, sans-serif;\"");
        if is_rtl {
            svg.push_str(" dir=\"rtl\"");
        }
        svg.push_str(">\n");

        svg.push_str("<defs>\n");
        svg.push_str(
            "  <pattern id=\"grid\" width=\"40\" height=\"40\" patternUnits=\"userSpaceOnUse\">\n",
        );
        svg.push_str("    <path d=\"M 40 0 L 0 0 0 40\" fill=\"none\" stroke=\"#1e293b\" stroke-width=\"0.5\"/>\n");
        svg.push_str("  </pattern>\n");
        svg.push_str("  <marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"6\" refY=\"5\" markerWidth=\"6\" markerHeight=\"6\" orient=\"auto-start-reverse\">\n");
        svg.push_str("    <path d=\"M 0 1 L 10 5 L 0 9 z\" fill=\"#94a3b8\"/>\n");
        svg.push_str("  </marker>\n");
        svg.push_str("</defs>\n");

        svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#020617\"/>\n");
        svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"url(#grid)\"/>\n\n");

        // Header Title
        let text_x = if is_rtl { canvas_w - 40.0 } else { 40.0 };
        let text_anchor = if is_rtl { "end" } else { "start" };
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"40\" fill=\"#f8fafc\" font-size=\"20\" font-weight=\"bold\" text-anchor=\"{}\">{}</text>\n",
            text_x, text_anchor, delta.meta.title
        ));

        // Legend
        svg.push_str("<g transform=\"translate(40, 55)\" font-size=\"11\" font-weight=\"500\">\n");
        svg.push_str(
            "  <rect x=\"0\" y=\"0\" width=\"12\" height=\"12\" rx=\"2\" fill=\"#22c55e\"/>\n",
        );
        svg.push_str("  <text x=\"18\" y=\"10\" fill=\"#22c55e\">[+] Added</text>\n");
        svg.push_str(
            "  <rect x=\"90\" y=\"0\" width=\"12\" height=\"12\" rx=\"2\" fill=\"#f43f5e\"/>\n",
        );
        svg.push_str("  <text x=\"108\" y=\"10\" fill=\"#f43f5e\">[-] Removed</text>\n");
        svg.push_str(
            "  <rect x=\"190\" y=\"0\" width=\"12\" height=\"12\" rx=\"2\" fill=\"#f59e0b\"/>\n",
        );
        svg.push_str("  <text x=\"208\" y=\"10\" fill=\"#f59e0b\">[Δ] Modified</text>\n");
        svg.push_str("</g>\n\n");

        // Connections
        for d_conn in &delta.connections {
            let conn = &d_conn.connection;
            if let (Some(from_c), Some(to_c)) = (
                delta
                    .components
                    .iter()
                    .find(|c| c.component.id == conn.from),
                delta.components.iter().find(|c| c.component.id == conn.to),
            ) {
                let x1 = from_c.component.x + from_c.component.width / 2.0;
                let y1 = from_c.component.y + from_c.component.height;
                let x2 = to_c.component.x + to_c.component.width / 2.0;
                let y2 = to_c.component.y;
                let stroke = d_conn.status.neon_hex();

                let dash = if d_conn.status == DiffStatus::Removed {
                    " stroke-dasharray=\"4,4\""
                } else {
                    ""
                };

                svg.push_str(&format!(
                    "<path d=\"M {} {} C {} {}, {} {}, {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" marker-end=\"url(#arrow)\"{}>\n  <style>filter:drop-shadow(0 0 4px {});</style>\n</path>\n",
                    x1, y1, x1, (y1 + y2) / 2.0, x2, (y1 + y2) / 2.0, x2, y2, stroke, dash, stroke
                ));
            }
        }

        // Components
        for d_comp in &delta.components {
            let comp = &d_comp.component;
            let stroke = d_comp.status.neon_hex();
            let fill = match d_comp.status {
                DiffStatus::Added => "rgba(34, 197, 94, 0.2)",
                DiffStatus::Removed => "rgba(244, 63, 94, 0.15)",
                DiffStatus::Modified => "rgba(245, 158, 11, 0.2)",
                DiffStatus::Unchanged => "rgba(30, 41, 59, 0.5)",
            };

            let dash = if d_comp.status == DiffStatus::Removed {
                " stroke-dasharray=\"5,3\""
            } else {
                ""
            };

            let prefix = match d_comp.status {
                DiffStatus::Added => "[+] ",
                DiffStatus::Removed => "[-] ",
                DiffStatus::Modified => "[Δ] ",
                DiffStatus::Unchanged => "",
            };

            svg.push_str(&format!(
                "<g style=\"filter:drop-shadow(0 0 8px {});\">\n",
                stroke
            ));
            svg.push_str(&format!(
                "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.8\"{}>\n  </rect>\n",
                comp.x, comp.y, comp.width, comp.height, fill, stroke, dash
            ));
            svg.push_str(&format!(
                "  <text x=\"{}\" y=\"{}\" fill=\"#f8fafc\" font-size=\"13\" font-weight=\"600\" text-anchor=\"middle\">{}{}</text>\n",
                comp.x + comp.width / 2.0, comp.y + comp.height / 2.0 - 4.0, prefix, comp.label
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
