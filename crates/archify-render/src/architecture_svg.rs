//! Architecture diagram renderer: regions, role-coloured components, labelled
//! links and a role legend, on a canvas that grows with the content.

use crate::xml::{esc, fit};
use archify_ir::{ArchitectureDiagram, Component, Connection, SemanticRole, VisualPreset};

pub struct ArchitectureSvgRenderer;

struct Theme {
    bg: &'static str,
    grid: &'static str,
    text_main: &'static str,
    text_sub: &'static str,
    font: &'static str,
    corner_rx: &'static str,
    glow: &'static str,
}

fn theme(preset: VisualPreset) -> Theme {
    let (bg, grid, text_main, text_sub, font, corner_rx, glow) = match preset {
        VisualPreset::SignalFlow => (
            "#020617",
            "#1e293b",
            "#f8fafc",
            "#94a3b8",
            "system-ui, sans-serif",
            "6",
            "8px",
        ),
        VisualPreset::Classic => (
            "#090d16",
            "#1e293b",
            "#f8fafc",
            "#94a3b8",
            "system-ui, sans-serif",
            "6",
            "4px",
        ),
        VisualPreset::Blueprint => (
            "#0a192f",
            "#172a45",
            "#e6f1ff",
            "#8892b0",
            "monospace, Courier, sans-serif",
            "0",
            "0px",
        ),
        VisualPreset::Editorial => (
            "#f8fafc",
            "#e2e8f0",
            "#0f172a",
            "#475569",
            "Georgia, Cambria, serif",
            "2",
            "2px",
        ),
    };
    Theme {
        bg,
        grid,
        text_main,
        text_sub,
        font,
        corner_rx,
        glow,
    }
}

const MIN_W: f32 = 900.0;
const MIN_H: f32 = 650.0;
const MARGIN: f32 = 40.0;
const LEGEND_H: f32 = 44.0;
const ROLE_ORDER: [SemanticRole; 7] = [
    SemanticRole::Frontend,
    SemanticRole::Backend,
    SemanticRole::Database,
    SemanticRole::Security,
    SemanticRole::Messagebus,
    SemanticRole::Cloud,
    SemanticRole::External,
];

fn canvas(d: &ArchitectureDiagram) -> (f32, f32) {
    let right = (d.components.iter().map(|c| c.x + c.width))
        .chain(d.regions.iter().map(|r| r.x + r.width))
        .fold(0.0, f32::max);
    let bottom = (d.components.iter().map(|c| c.y + c.height))
        .chain(d.regions.iter().map(|r| r.y + r.height))
        .fold(0.0, f32::max);
    let legend = if d.components.is_empty() {
        0.0
    } else {
        LEGEND_H
    };
    (
        (right + MARGIN).max(MIN_W),
        (bottom + MARGIN + legend).max(MIN_H),
    )
}

fn role_name(role: SemanticRole, locale: &str) -> &'static str {
    let (tr, ar, en) = match role {
        SemanticRole::Frontend => ("Arayüz", "واجهة", "Frontend"),
        SemanticRole::Backend => ("Çekirdek", "منطق", "Backend"),
        SemanticRole::Database => ("Veri", "بيانات", "Database"),
        SemanticRole::Cloud => ("Bulut", "سحابة", "Cloud"),
        SemanticRole::Security => ("Güvenlik", "أمان", "Security"),
        SemanticRole::Messagebus => ("Olay/mesaj", "رسائل", "Message bus"),
        SemanticRole::External => ("Dış", "خارجي", "External"),
    };
    match locale {
        "tr" => tr,
        "ar" => ar,
        _ => en,
    }
}

impl ArchitectureSvgRenderer {
    pub fn render(d: &ArchitectureDiagram) -> String {
        let rtl = d.meta.locale == "ar";
        let t = theme(d.meta.visual_preset);
        let (w, h) = canvas(d);
        let mut svg = String::new();
        svg.push_str(&format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"100%\" height=\"100%\" style=\"background:{};font-family:{};\"{}>\n",
            t.bg, t.font, if rtl { " dir=\"rtl\"" } else { "" }
        ));
        svg.push_str(&format!(
            "<defs>\n  <pattern id=\"grid\" width=\"40\" height=\"40\" patternUnits=\"userSpaceOnUse\">\n    <path d=\"M 40 0 L 0 0 0 40\" fill=\"none\" stroke=\"{}\" stroke-width=\"0.5\"/>\n  </pattern>\n  <marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"6\" refY=\"5\" markerWidth=\"6\" markerHeight=\"6\" orient=\"auto-start-reverse\">\n    <path d=\"M 0 1 L 10 5 L 0 9 z\" fill=\"{}\"/>\n  </marker>\n</defs>\n",
            t.grid, t.text_sub
        ));
        svg.push_str(&format!(
            "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>\n",
            t.bg
        ));
        svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"url(#grid)\"/>\n\n");

        let (tx, anchor) = if rtl {
            (w - MARGIN, "end")
        } else {
            (MARGIN, "start")
        };
        svg.push_str(&format!(
            "<text x=\"{tx}\" y=\"40\" fill=\"{}\" font-size=\"20\" font-weight=\"bold\" text-anchor=\"{anchor}\">{}</text>\n",
            t.text_main, esc(&d.meta.title)
        ));
        if let Some(sub) = &d.meta.subtitle {
            svg.push_str(&format!(
                "<text x=\"{tx}\" y=\"62\" fill=\"{}\" font-size=\"12\" text-anchor=\"{anchor}\">{}</text>\n",
                t.text_sub, esc(sub)
            ));
        }

        regions(&mut svg, d, &t, rtl);
        for conn in &d.connections {
            link(&mut svg, d, conn, &t);
        }
        for comp in &d.components {
            component(&mut svg, comp, d.meta.visual_preset, &t);
        }
        legend(&mut svg, d, (w, h), &t, rtl);
        svg.push_str("</svg>\n");
        svg
    }
}

fn regions(svg: &mut String, d: &ArchitectureDiagram, t: &Theme, rtl: bool) {
    let fill = if d.meta.visual_preset == VisualPreset::Editorial {
        "rgba(15,23,42,0.04)"
    } else {
        "rgba(148,163,184,0.06)"
    };
    for r in &d.regions {
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"12\" fill=\"{fill}\" stroke=\"{}\" stroke-opacity=\"0.45\" stroke-width=\"1.2\" stroke-dasharray=\"6 4\"/>\n",
            r.x, r.y, r.width, r.height, t.text_sub
        ));
        let (near, far, a_near, a_far) = if rtl {
            (r.x + r.width - 16.0, r.x + 16.0, "end", "start")
        } else {
            (r.x + 16.0, r.x + r.width - 16.0, "start", "end")
        };
        svg.push_str(&format!(
            "<text x=\"{near}\" y=\"{}\" fill=\"{}\" font-size=\"13\" font-weight=\"700\" text-anchor=\"{a_near}\">{}</text>\n",
            r.y + 22.0, t.text_main, esc(&fit(&r.label, 28))
        ));
        if let Some(sub) = &r.sublabel {
            svg.push_str(&format!(
                "<text x=\"{far}\" y=\"{}\" fill=\"{}\" font-size=\"10\" text-anchor=\"{a_far}\">{}</text>\n",
                r.y + 22.0, t.text_sub, esc(sub)
            ));
        }
    }
}

/// Connects the facing sides of two components; horizontal when they are further
/// apart sideways than vertically.
fn link(svg: &mut String, d: &ArchitectureDiagram, conn: &Connection, t: &Theme) {
    let (Some(a), Some(b)) = (
        d.components.iter().find(|c| c.id == conn.from),
        d.components.iter().find(|c| c.id == conn.to),
    ) else {
        return;
    };
    let ((acx, acy), (bcx, bcy)) = (
        (a.x + a.width / 2.0, a.y + a.height / 2.0),
        (b.x + b.width / 2.0, b.y + b.height / 2.0),
    );
    let (dx, dy) = (bcx - acx, bcy - acy);
    let ((x1, y1), (x2, y2), horizontal) = if dx.abs() > dy.abs() {
        if dx >= 0.0 {
            ((a.x + a.width, acy), (b.x, bcy), true)
        } else {
            ((a.x, acy), (b.x + b.width, bcy), true)
        }
    } else if dy >= 0.0 {
        ((acx, a.y + a.height), (bcx, b.y), false)
    } else {
        ((acx, a.y), (bcx, b.y + b.height), false)
    };
    let (mx, my) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0);
    let (c1, c2) = if horizontal {
        ((mx, y1), (mx, y2))
    } else {
        ((x1, my), (x2, my))
    };
    let (stroke, width, dash) = match conn.line_style.as_str() {
        "strong" => (a.role.stroke_hex(), "2.6", ""),
        "dashed" => (t.text_sub, "1.4", " stroke-dasharray=\"5 4\""),
        _ => (t.text_sub, "1.4", ""),
    };
    svg.push_str(&format!(
        "<path d=\"M {x1} {y1} C {} {}, {} {}, {x2} {y2}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{width}\" stroke-opacity=\"0.8\"{dash} marker-end=\"url(#arrow)\"/>\n",
        c1.0, c1.1, c2.0, c2.1
    ));
    if let Some(label) = &conn.label {
        svg.push_str(&format!(
            "<text x=\"{mx}\" y=\"{}\" fill=\"{}\" font-size=\"10\" text-anchor=\"middle\" stroke=\"{}\" stroke-width=\"4\" stroke-linejoin=\"round\" paint-order=\"stroke\">{}</text>\n",
            my - 5.0, t.text_sub, t.bg, esc(label)
        ));
    }
}

fn component(svg: &mut String, c: &Component, preset: VisualPreset, t: &Theme) {
    let stroke = c.role.stroke_hex();
    let fill = if preset == VisualPreset::Editorial {
        "rgba(255, 255, 255, 0.9)"
    } else {
        c.role.fill_rgba()
    };
    let filter = match preset {
        VisualPreset::Blueprint => String::new(),
        VisualPreset::Editorial => {
            "style=\"filter:drop-shadow(0 2px 3px rgba(0,0,0,0.15));\"".to_string()
        }
        _ => format!("style=\"filter:drop-shadow(0 0 {} {stroke});\"", t.glow),
    };
    let cx = c.x + c.width / 2.0;
    svg.push_str(&format!("<g {filter}>\n"));
    svg.push_str(&format!(
        "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.8\"/>\n",
        c.x, c.y, c.width, c.height, t.corner_rx
    ));
    // ~7 px per character at 13 px, ~5.6 px at 10 px: keep text inside the box.
    let (max_label, max_sub) = ((c.width / 7.0) as usize, (c.width / 5.6) as usize);
    svg.push_str(&format!(
        "  <text x=\"{cx}\" y=\"{}\" fill=\"{}\" font-size=\"13\" font-weight=\"600\" text-anchor=\"middle\">{}</text>\n",
        c.y + c.height / 2.0 - 4.0, t.text_main, esc(&fit(&c.label, max_label))
    ));
    if let Some(sub) = &c.sublabel {
        svg.push_str(&format!(
            "  <text x=\"{cx}\" y=\"{}\" fill=\"{}\" font-size=\"10\" text-anchor=\"middle\">{}</text>\n",
            c.y + c.height / 2.0 + 14.0, t.text_sub, esc(&fit(sub, max_sub))
        ));
    }
    svg.push_str("</g>\n");
}

fn legend(svg: &mut String, d: &ArchitectureDiagram, (w, h): (f32, f32), t: &Theme, rtl: bool) {
    let y = h - 26.0;
    let present = ROLE_ORDER
        .iter()
        .filter(|r| d.components.iter().any(|c| c.role == **r));
    for (i, role) in present.enumerate() {
        let x = if rtl {
            w - MARGIN - 12.0 - i as f32 * 140.0
        } else {
            MARGIN + i as f32 * 140.0
        };
        svg.push_str(&format!(
            "<rect x=\"{x}\" y=\"{}\" width=\"12\" height=\"12\" rx=\"3\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.4\"/>\n",
            y - 10.0, role.fill_rgba(), role.stroke_hex()
        ));
        let (lx, anchor) = if rtl {
            (x - 8.0, "end")
        } else {
            (x + 20.0, "start")
        };
        svg.push_str(&format!(
            "<text x=\"{lx}\" y=\"{y}\" fill=\"{}\" font-size=\"11\" text-anchor=\"{anchor}\">{}</text>\n",
            t.text_sub, esc(role_name(*role, &d.meta.locale))
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archify_ir::{DiagramMeta, Region};

    fn comp(id: &str, label: &str, role: SemanticRole, x: f32, y: f32) -> Component {
        Component {
            id: id.into(),
            label: label.into(),
            sublabel: Some("1 files · 2 symbols".into()),
            role,
            x,
            y,
            width: 150.0,
            height: 56.0,
        }
    }

    fn diagram(components: Vec<Component>) -> ArchitectureDiagram {
        ArchitectureDiagram {
            meta: DiagramMeta {
                title: "T".into(),
                subtitle: Some("sub".into()),
                locale: "en".into(),
                visual_preset: VisualPreset::SignalFlow,
            },
            components,
            connections: vec![],
            regions: vec![],
            story_beats: vec![],
        }
    }

    #[test]
    fn regions_are_drawn_before_components() {
        let mut d = diagram(vec![comp("a/x", "x", SemanticRole::Backend, 60.0, 130.0)]);
        d.regions.push(Region {
            id: "a".into(),
            label: "core-region".into(),
            sublabel: Some("1/1 modules".into()),
            x: 40.0,
            y: 96.0,
            width: 200.0,
            height: 120.0,
        });
        let svg = ArchitectureSvgRenderer::render(&d);
        assert!(svg.contains("core-region") && svg.contains("1/1 modules"));
        assert!(
            svg.find("stroke-dasharray=\"6 4\"").unwrap() < svg.find("filter:drop-shadow").unwrap()
        );
    }

    #[test]
    fn names_are_escaped_and_long_labels_are_cut() {
        let svg = ArchitectureSvgRenderer::render(&diagram(vec![
            comp("c", "a&b<c>", SemanticRole::Backend, 0.0, 0.0),
            comp(
                "d",
                "an_extremely_long_module_name",
                SemanticRole::Backend,
                200.0,
                0.0,
            ),
        ]));
        assert!(svg.contains("a&amp;b&lt;c&gt;") && !svg.contains("a&b<c>"));
        assert!(svg.contains('…') && !svg.contains("an_extremely_long_module_name"));
    }

    #[test]
    fn canvas_grows_with_content_but_never_below_the_minimum() {
        let small = ArchitectureSvgRenderer::render(&diagram(vec![comp(
            "a",
            "a",
            SemanticRole::Backend,
            10.0,
            10.0,
        )]));
        assert!(small.contains("viewBox=\"0 0 900 650\""));
        let big = ArchitectureSvgRenderer::render(&diagram(vec![comp(
            "a",
            "a",
            SemanticRole::Backend,
            2000.0,
            1500.0,
        )]));
        assert!(big.contains("viewBox=\"0 0 2190 1640\""), "{}", &big[..120]);
    }

    #[test]
    fn links_face_each_other_carry_labels_and_styles() {
        let mut d = diagram(vec![
            comp("a", "a", SemanticRole::Frontend, 0.0, 0.0),
            comp("b", "b", SemanticRole::Backend, 300.0, 0.0),
        ]);
        d.connections = vec![
            Connection {
                from: "a".into(),
                to: "b".into(),
                label: Some("calls ×12".into()),
                line_style: "strong".into(),
            },
            Connection {
                from: "b".into(),
                to: "a".into(),
                label: None,
                line_style: "dashed".into(),
            },
        ];
        let svg = ArchitectureSvgRenderer::render(&d);
        assert!(svg.contains("calls ×12") && svg.contains("stroke-width=\"2.6\""));
        assert!(
            svg.contains("M 150 28 C 225 28, 225 28, 300 28"),
            "a→b leaves a's right side"
        );
        assert!(
            svg.contains("stroke-dasharray=\"5 4\"") && svg.contains("M 300 28 C"),
            "b→a leaves b's left side"
        );
    }

    #[test]
    fn legend_lists_only_present_roles_in_the_locale() {
        let mut d = diagram(vec![comp("a", "a", SemanticRole::Backend, 0.0, 0.0)]);
        d.meta.locale = "tr".into();
        let svg = ArchitectureSvgRenderer::render(&d);
        assert!(svg.contains(">Çekirdek<") && !svg.contains(">Veri<"));
    }
}
