//! Sequence diagrams: participant boxes, dashed lifelines and chronological messages. The
//! canvas grows with the participant count and the message list.

use crate::css::stylesheet;
use crate::icons::{role_css, sigil};
use crate::xml::{esc, num};
use archify_ir::SequenceDiagram;
use archify_style::{font, text, Theme, Tokens};
use std::collections::HashMap;
use std::fmt::Write;

pub struct SequenceSvgRenderer;

const MARGIN: f32 = 40.0;
const MSG_SPACING: f32 = 45.0;
const BOX_Y: f32 = 72.0;
const BOX_H: f32 = 40.0;
const FIRST_MSG_Y: f32 = 150.0;

impl SequenceSvgRenderer {
    pub fn render(seq: &SequenceDiagram) -> String {
        let rtl = seq.meta.locale == "ar";
        let tokens = Tokens::new(seq.meta.visual_preset, Theme::Dark);
        let widths: Vec<f32> = seq
            .participants
            .iter()
            .map(|p| {
                (text::units(&p.label) * font::NODE_LABEL * font::MONO_ADVANCE + 28.0)
                    .clamp(110.0, 200.0)
            })
            .collect();
        let pitch = widths.iter().copied().fold(0.0f32, f32::max).max(86.0) + 24.0;
        let pitch = pitch.max(108.0);
        let n = seq.participants.len();
        let widest = widths.iter().copied().fold(0.0f32, f32::max);
        let content_w = pitch * (n.max(1) as f32 - 1.0) + widest;
        let title_w = text::width(&seq.meta.title, font::TITLE);
        let width = (content_w + 2.0 * MARGIN)
            .max(title_w + 2.0 * MARGIN)
            .max(520.0);
        let height = FIRST_MSG_Y + seq.messages.len() as f32 * MSG_SPACING + 50.0;

        // centre x of each participant (mirrored for right-to-left)
        let first = (width - pitch * (n.max(1) as f32 - 1.0)) * 0.5;
        let cx = |i: usize| {
            let x = first + i as f32 * pitch;
            if rtl {
                width - x
            } else {
                x
            }
        };
        let mut xs: HashMap<&str, f32> = HashMap::new();
        for (i, p) in seq.participants.iter().enumerate() {
            xs.insert(p.id.as_str(), cx(i));
        }

        let mut o = String::new();
        let _ = writeln!(
            o,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\" role=\"img\" data-preset=\"{}\" data-theme=\"auto\"{}>\n",
            Tokens::preset_name(seq.meta.visual_preset)
                .chars()
                .map(|c| if c == ' ' { '-' } else { c.to_ascii_lowercase() })
                .collect::<String>(),
            if rtl { " dir=\"rtl\"" } else { "" },
            w = num(width),
            h = num(height)
        );
        let _ = writeln!(o, "<title>{}</title>", esc(&seq.meta.title));
        let _ = writeln!(
            o,
            "<desc>{} · {} / {}</desc>",
            esc(&seq.meta.title),
            seq.participants.len(),
            seq.messages.len()
        );
        let _ = writeln!(
            o,
            "<style>\n{}</style>",
            stylesheet(seq.meta.visual_preset, "")
        );
        o.push_str("<defs>\n");
        for (id, cls) in [("m-default", "m-default"), ("m-dashed", "m-dashed")] {
            let _ = writeln!(
                o,
                "<marker id=\"{id}\" markerWidth=\"10\" markerHeight=\"7\" refX=\"9\" refY=\"3.5\" orient=\"auto\"><polygon class=\"{cls}\" points=\"0 0, 10 3.5, 0 7\"/></marker>"
            );
        }
        o.push_str("</defs>\n");
        let _ = writeln!(
            o,
            "<rect class=\"bg\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
            num(width),
            num(height),
            tokens.bg.hex()
        );
        let (tx, anchor) = if rtl {
            (width - MARGIN, "end")
        } else {
            (MARGIN, "start")
        };
        let _ = writeln!(
            o,
            "<text class=\"t-title\" x=\"{}\" y=\"40\" font-size=\"{}\" text-anchor=\"{anchor}\">{}</text>",
            num(tx),
            font::TITLE,
            esc(&seq.meta.title)
        );

        let bottom = height - 30.0;
        for (i, p) in seq.participants.iter().enumerate() {
            let c = cx(i);
            let w = widths[i];
            let role = role_css(p.role);
            let fitted = text::fit(&p.label, w - 16.0, font::NODE_LABEL, 8.0);
            let _ = writeln!(
                o,
                "<line x1=\"{x}\" y1=\"{}\" x2=\"{x}\" y2=\"{}\" stroke=\"var(--lane-stroke)\" stroke-width=\"1\" stroke-dasharray=\"4,4\"/>",
                num(BOX_Y + BOX_H),
                num(bottom),
                x = num(c)
            );
            let _ = writeln!(
                o,
                "<g class=\"node\" data-node-id=\"{id}\" data-node-kind=\"{role}\" style=\"--step:{}\"><title>{label}</title><rect class=\"mask\" x=\"{x}\" y=\"{BOX_Y}\" width=\"{w}\" height=\"{BOX_H}\" rx=\"6\"/><rect class=\"box c-{role}\" x=\"{x}\" y=\"{BOX_Y}\" width=\"{w}\" height=\"{BOX_H}\" rx=\"6\"/><g class=\"sigil i-{role}\" transform=\"translate({sx} {sy}) scale(.6875)\">{}</g><text class=\"t-primary\" x=\"{}\" y=\"{}\" font-size=\"{}\" font-weight=\"600\" text-anchor=\"middle\">{}</text></g>",
                i.min(12),
                sigil(p.role),
                num(c),
                num(BOX_Y + BOX_H * 0.5 + fitted.font * 0.35),
                num(fitted.font),
                esc(&fitted.text),
                id = esc(&p.id),
                label = esc(&p.label),
                x = num(c - w * 0.5),
                w = num(w),
                sx = num(c - w * 0.5 + 5.0),
                sy = num(BOX_Y + 5.0),
            );
        }

        for (idx, m) in seq.messages.iter().enumerate() {
            let y = FIRST_MSG_Y + idx as f32 * MSG_SPACING;
            let xf = xs.get(m.from.as_str()).copied().unwrap_or(first);
            let xt = xs.get(m.to.as_str()).copied().unwrap_or(first);
            let (cls, marker, dash) = if m.is_async {
                ("a-dashed", "m-dashed", " stroke-dasharray=\"4,4\"")
            } else {
                ("a-default", "m-default", "")
            };
            let label = format!("{}. {}", m.order, m.action);
            let fitted = text::fit(&label, 260.0, 9.0, 7.0);
            let d = if (xf - xt).abs() < 1.0 {
                format!(
                    "M {} {} L {} {} L {} {} L {} {}",
                    num(xf),
                    num(y - 8.0),
                    num(xf + 36.0),
                    num(y - 8.0),
                    num(xf + 36.0),
                    num(y + 8.0),
                    num(xf + 1.0),
                    num(y + 8.0)
                )
            } else {
                format!("M {} {} L {} {}", num(xf), num(y), num(xt), num(y))
            };
            let mid = if (xf - xt).abs() < 1.0 {
                xf + 18.0
            } else {
                (xf + xt) * 0.5
            };
            let plate_w = text::edge_label_width(&fitted.text)
                .max(text::width(&fitted.text, fitted.font) + 10.0);
            let _ = writeln!(
                o,
                "<g class=\"edge\" data-edge-from=\"{}\" data-edge-to=\"{}\" style=\"--step:{}\"><path class=\"{cls}\" d=\"{d}\" marker-end=\"url(#{marker})\"{dash}/></g><g class=\"lbl\"><rect class=\"mask\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"14\" rx=\"3\"/><text class=\"t-primary\" x=\"{}\" y=\"{}\" font-size=\"{}\" text-anchor=\"middle\">{}</text></g>",
                esc(&m.from),
                esc(&m.to),
                idx.min(12),
                num(mid - plate_w * 0.5),
                num(y - 22.0),
                num(plate_w),
                num(mid),
                num(y - 12.0),
                num(fitted.font),
                esc(&fitted.text)
            );
        }
        o.push_str("</svg>\n");
        o
    }
}
