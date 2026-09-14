//! archify-render: Sequence Diagram SVG renderer with chronological lifelines and message arrows.

use archify_ir::SequenceDiagram;
use std::collections::HashMap;

pub struct SequenceSvgRenderer;

impl SequenceSvgRenderer {
    pub fn render(seq: &SequenceDiagram) -> String {
        let is_rtl = seq.meta.locale == "ar";
        let canvas_w = 900.0f32;
        let msg_spacing = 45.0f32;
        let start_y = 120.0f32;
        let canvas_h = (start_y + (seq.messages.len() as f32 + 2.0) * msg_spacing).max(650.0);

        let mut svg = String::new();
        svg.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\" ");
        svg.push_str(&format!("viewBox=\"0 0 {} {}\" width=\"100%\" height=\"100%\" ", canvas_w, canvas_h));
        svg.push_str("style=\"background:#020617;font-family:system-ui, sans-serif;\"");
        if is_rtl {
            svg.push_str(" dir=\"rtl\"");
        }
        svg.push_str(">\n");

        svg.push_str("<defs>\n");
        svg.push_str("  <marker id=\"seq-arrow\" viewBox=\"0 0 10 10\" refX=\"6\" refY=\"5\" markerWidth=\"6\" markerHeight=\"6\" orient=\"auto-start-reverse\">\n");
        svg.push_str("    <path d=\"M 0 1 L 10 5 L 0 9 z\" fill=\"#22d3ee\"/>\n");
        svg.push_str("  </marker>\n");
        svg.push_str("</defs>\n");

        svg.push_str(&format!("<rect width=\"100%\" height=\"{}\" fill=\"#020617\"/>\n\n", canvas_h));

        // Title
        let text_x = if is_rtl { canvas_w - 40.0 } else { 40.0 };
        let text_anchor = if is_rtl { "end" } else { "start" };
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"40\" fill=\"#f8fafc\" font-size=\"20\" font-weight=\"bold\" text-anchor=\"{}\">{}</text>\n",
            text_x, text_anchor, seq.meta.title
        ));

        let part_count = seq.participants.len().max(1);
        let part_width = 130.0f32;
        let margin_x = 80.0f32;
        let part_step = (canvas_w - margin_x * 2.0 - part_width) / (part_count.saturating_sub(1).max(1) as f32);

        let mut part_x_map = HashMap::new();

        // Render Participants and vertical Lifelines
        for (i, part) in seq.participants.iter().enumerate() {
            let px = margin_x + i as f32 * part_step;
            let center_x = px + part_width / 2.0;
            part_x_map.insert(part.id.as_str(), center_x);

            let stroke = part.role.stroke_hex();

            // Lifeline
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"100\" x2=\"{}\" y2=\"{}\" stroke=\"#334155\" stroke-width=\"1.5\" stroke-dasharray=\"4,4\"/>\n",
                center_x, center_x, canvas_h - 40.0
            ));

            // Participant Box
            svg.push_str(&format!(
                "<g style=\"filter:drop-shadow(0 0 6px {});\">\n", stroke
            ));
            svg.push_str(&format!(
                "  <rect x=\"{}\" y=\"60\" width=\"{}\" height=\"40\" rx=\"6\" fill=\"#090d16\" stroke=\"{}\" stroke-width=\"1.8\"/>\n",
                px, part_width, stroke
            ));
            svg.push_str(&format!(
                "  <text x=\"{}\" y=\"85\" fill=\"#f8fafc\" font-size=\"12\" font-weight=\"600\" text-anchor=\"middle\">{}</text>\n",
                center_x, part.label
            ));
            svg.push_str("</g>\n");
        }

        // Render Chronological Messages
        for (idx, msg) in seq.messages.iter().enumerate() {
            let from_x = part_x_map.get(msg.from.as_str()).copied().unwrap_or(100.0);
            let to_x = part_x_map.get(msg.to.as_str()).copied().unwrap_or(200.0);
            let y = start_y + idx as f32 * msg_spacing;

            let dash = if msg.is_async { " stroke-dasharray=\"3,3\"" } else { "" };
            let color = if msg.is_async { "#a78bfa" } else { "#22d3ee" };

            svg.push_str(&format!(
                "<g style=\"filter:drop-shadow(0 0 4px {});\">\n", color
            ));
            svg.push_str(&format!(
                "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.8\" marker-end=\"url(#seq-arrow)\"{}/>\n",
                from_x, y, to_x, y, color, dash
            ));
            let mid_x = (from_x + to_x) / 2.0;
            svg.push_str(&format!(
                "  <text x=\"{}\" y=\"{}\" fill=\"#f8fafc\" font-size=\"11\" text-anchor=\"middle\">{}. {}</text>\n",
                mid_x, y - 6.0, msg.order, msg.action
            ));
            svg.push_str("</g>\n");
        }

        svg.push_str("</svg>\n");
        svg
    }
}
