//! archify-render: Dataflow Diagram SVG renderer with streaming pipelines.

use archify_ir::DataflowDiagram;

pub struct DataflowSvgRenderer;

impl DataflowSvgRenderer {
    pub fn render(df: &DataflowDiagram) -> String {
        let is_rtl = df.meta.locale == "ar";
        let canvas_w = 900.0f32;
        let canvas_h = 650.0f32;

        let mut svg = String::new();
        svg.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\" ");
        svg.push_str(&format!("viewBox=\"0 0 {} {}\" width=\"100%\" height=\"100%\" ", canvas_w, canvas_h));
        svg.push_str("style=\"background:#020617;font-family:system-ui, sans-serif;\"");
        if is_rtl {
            svg.push_str(" dir=\"rtl\"");
        }
        svg.push_str(">\n");

        svg.push_str("<defs>\n");
        svg.push_str("  <pattern id=\"df-grid\" width=\"40\" height=\"40\" patternUnits=\"userSpaceOnUse\">\n");
        svg.push_str("    <path d=\"M 40 0 L 0 0 0 40\" fill=\"none\" stroke=\"#1e293b\" stroke-width=\"0.5\"/>\n");
        svg.push_str("  </pattern>\n");
        svg.push_str("  <marker id=\"df-arrow\" viewBox=\"0 0 10 10\" refX=\"6\" refY=\"5\" markerWidth=\"6\" markerHeight=\"6\" orient=\"auto-start-reverse\">\n");
        svg.push_str("    <path d=\"M 0 1 L 10 5 L 0 9 z\" fill=\"#38bdf8\"/>\n");
        svg.push_str("  </marker>\n");
        svg.push_str("</defs>\n");

        svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#020617\"/>\n");
        svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"url(#df-grid)\"/>\n\n");

        // Title
        let text_x = if is_rtl { canvas_w - 40.0 } else { 40.0 };
        let text_anchor = if is_rtl { "end" } else { "start" };
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"40\" fill=\"#f8fafc\" font-size=\"20\" font-weight=\"bold\" text-anchor=\"{}\">{}</text>\n",
            text_x, text_anchor, df.meta.title
        ));

        // Node Layout in a grid/pipeline
        let cell_w = 160.0f32;
        let cell_h = 70.0f32;
        let gap_x = 70.0f32;
        let gap_y = 60.0f32;
        let cols = 3;

        let mut node_positions = std::collections::HashMap::new();

        for (i, node) in df.nodes.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let x = 60.0 + col as f32 * (cell_w + gap_x);
            let y = 100.0 + row as f32 * (cell_h + gap_y);
            node_positions.insert(node.id.as_str(), (x, y));

            let stroke = node.role.stroke_hex();
            let fill = node.role.fill_rgba();

            svg.push_str(&format!(
                "<g style=\"filter:drop-shadow(0 0 8px {});\">\n", stroke
            ));
            svg.push_str(&format!(
                "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.8\"/>\n",
                x, y, cell_w, cell_h, fill, stroke
            ));
            svg.push_str(&format!(
                "  <text x=\"{}\" y=\"{}\" fill=\"#f8fafc\" font-size=\"13\" font-weight=\"600\" text-anchor=\"middle\">{}</text>\n",
                x + cell_w / 2.0, y + cell_h / 2.0 - 4.0, node.label
            ));

            if let Some(rate) = &node.stream_rate {
                svg.push_str(&format!(
                    "  <text x=\"{}\" y=\"{}\" fill=\"#38bdf8\" font-size=\"10\" text-anchor=\"middle\">⚡ {}</text>\n",
                    x + cell_w / 2.0, y + cell_h / 2.0 + 14.0, rate
                ));
            }
            svg.push_str("</g>\n");
        }

        // Pipelines
        for pipe in &df.pipelines {
            if let (Some(&(x1, y1)), Some(&(x2, y2))) = (node_positions.get(pipe.from.as_str()), node_positions.get(pipe.to.as_str())) {
                let p1_x = x1 + cell_w;
                let p1_y = y1 + cell_h / 2.0;
                let p2_x = x2;
                let p2_y = y2 + cell_h / 2.0;

                svg.push_str(&format!(
                    "<path d=\"M {} {} C {} {}, {} {}, {} {}\" fill=\"none\" stroke=\"#38bdf8\" stroke-width=\"2\" marker-end=\"url(#df-arrow)\">\n  <style>filter:drop-shadow(0 0 4px #38bdf8);</style>\n</path>\n",
                    p1_x, p1_y, (p1_x + p2_x) / 2.0, p1_y, (p1_x + p2_x) / 2.0, p2_y, p2_x, p2_y
                ));

                if let Some(throughput) = &pipe.throughput {
                    let mid_x = (p1_x + p2_x) / 2.0;
                    let mid_y = (p1_y + p2_y) / 2.0;
                    svg.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" fill=\"#94a3b8\" font-size=\"9\" text-anchor=\"middle\">{}</text>\n",
                        mid_x, mid_y - 6.0, throughput
                    ));
                }
            }
        }

        svg.push_str("</svg>\n");
        svg
    }
}
