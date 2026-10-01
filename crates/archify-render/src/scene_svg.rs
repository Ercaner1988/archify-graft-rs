//! Draws a resolved [`Scene`] (architecture, dataflow or delta) as one self-contained SVG.

use crate::css::{hover_rules, status_css, stylesheet, KINDS, STATUSES};
use crate::icons::{role_css, sigil};
use crate::xml::{esc, num};
use archify_ir::DiffStatus;
use archify_route::{rounded_path, PathCmd, Rect, CORNER_RADIUS};
use archify_scene::{Scene, SceneEdge, SceneNode, MARGIN};
use archify_style::{font, role_name, text, Theme, Tokens};
use std::fmt::Write;

/// What the legend row lists.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LegendKind {
    Roles,
    Status,
}

const TITLE_BAND: f32 = 56.0;
const LEGEND_BAND: f32 = 44.0;

pub(crate) fn path_d(points: &[[f32; 2]]) -> String {
    let mut d = String::new();
    for cmd in rounded_path(points, CORNER_RADIUS) {
        match cmd {
            PathCmd::MoveTo(p) => {
                let _ = write!(d, "M {} {}", num(p[0]), num(p[1]));
            }
            PathCmd::LineTo(p) => {
                let _ = write!(d, " L {} {}", num(p[0]), num(p[1]));
            }
            PathCmd::QuadTo(c, e) => {
                let _ = write!(
                    d,
                    " Q {} {} {} {}",
                    num(c[0]),
                    num(c[1]),
                    num(e[0]),
                    num(e[1])
                );
            }
        }
    }
    d
}

/// Marker/class family of an edge: a variant kind or a delta status.
fn edge_family(e: &SceneEdge) -> String {
    match e.status {
        Some(s) => format!("st-{}", status_css(s)),
        None => e.kind.class().to_string(),
    }
}

fn legend_entries(scene: &Scene, kind: LegendKind) -> Vec<(String, String, String)> {
    // (swatch class, label, count)
    match kind {
        LegendKind::Roles => scene
            .legend
            .iter()
            .map(|(r, n)| {
                (
                    format!("sw-{}", role_css(*r)),
                    role_name(*r, &scene.locale).to_string(),
                    n.to_string(),
                )
            })
            .collect(),
        LegendKind::Status => [
            (DiffStatus::Added, "[+] Added"),
            (DiffStatus::Removed, "[-] Removed"),
            (DiffStatus::Modified, "[Δ] Modified"),
        ]
        .iter()
        .map(|(s, l)| {
            (
                format!("sw-st-{}", status_css(*s)),
                l.to_string(),
                String::new(),
            )
        })
        .collect(),
    }
}

pub(crate) fn draw(scene: &Scene, legend: LegendKind) -> String {
    let rtl = scene.locale == "ar";
    let tokens = Tokens::new(scene.preset, Theme::Dark);
    let entries = legend_entries(scene, legend);
    let item_w = |label: &str, count: &str| {
        24.0 + text::width(label, font::LEGEND) + if count.is_empty() { 0.0 } else { 26.0 } + 18.0
    };
    let legend_w: f32 = entries.iter().map(|(_, l, c)| item_w(l, c)).sum();
    let title_w = text::width(&scene.title, font::TITLE);

    let mut vb = scene.view_box(TITLE_BAND, LEGEND_BAND);
    let want_w = (legend_w + 2.0 * MARGIN)
        .max(title_w + 2.0 * MARGIN)
        .max(520.0);
    if vb.w < want_w {
        // centre the content in the widened picture
        vb.x -= (want_w - vb.w) * 0.5;
        vb.w = want_w;
    }

    let mut adjacency: Vec<(Vec<usize>, Vec<usize>)> =
        vec![(Vec::new(), Vec::new()); scene.nodes.len()];
    for (j, e) in scene.edges.iter().enumerate() {
        adjacency[e.from].0.push(e.to);
        adjacency[e.from].1.push(j);
        adjacency[e.to].0.push(e.from);
        adjacency[e.to].1.push(j);
    }

    let mut o = String::with_capacity(4096 + scene.nodes.len() * 900 + scene.edges.len() * 600);
    let _ = writeln!(
        o,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\" width=\"{}\" height=\"{}\" role=\"img\" data-preset=\"{}\" data-theme=\"auto\"{}>\n",
        num(vb.x),
        num(vb.y),
        num(vb.w),
        num(vb.h),
        num(vb.w),
        num(vb.h),
        preset_attr(Tokens::preset_name(scene.preset)),
        if rtl { " dir=\"rtl\"" } else { "" }
    );
    let _ = writeln!(o, "<title>{}</title>", esc(&scene.title));
    let (nc, ec) = if scene.locale == "tr" {
        ("bileşen", "bağlantı")
    } else {
        ("components", "connections")
    };
    let _ = writeln!(
        o,
        "<desc>{} · {} {} · {} {}</desc>",
        esc(&scene.title),
        scene.nodes.len(),
        nc,
        scene.edges.len(),
        ec
    );
    let hover = hover_rules(&adjacency);
    let _ = writeln!(o, "<style>\n{}</style>", stylesheet(scene.preset, &hover));
    o.push_str(&defs(scene));
    let _ = writeln!(
        o,
        "<rect class=\"bg\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
        num(vb.x),
        num(vb.y),
        num(vb.w),
        num(vb.h),
        tokens.bg.hex()
    );
    let _ = writeln!(
        o,
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"url(#grid)\"/>",
        num(vb.x),
        num(vb.y),
        num(vb.w),
        num(vb.h)
    );
    header(&mut o, scene, &vb, rtl);

    for r in &scene.regions {
        let _ = writeln!(
            o,
            "<g class=\"region\" data-region-id=\"{}\"><rect class=\"region-frame\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\"/></g>",
            esc(&r.id),
            num(r.rect.x),
            num(r.rect.y),
            num(r.rect.w),
            num(r.rect.h),
            tokens.region_radius
        );
    }
    for (j, e) in scene.edges.iter().enumerate() {
        edge(&mut o, scene, e, j);
    }
    for (i, n) in scene.nodes.iter().enumerate() {
        node(&mut o, n, i);
    }
    for (j, e) in scene.edges.iter().enumerate() {
        edge_label(&mut o, scene, e, j);
    }
    for r in &scene.regions {
        let p = r.title_plate;
        let _ = writeln!(
            o,
            "<g class=\"region-title\" data-region-id=\"{}\"><rect class=\"mask\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\"/><text class=\"t-region\" x=\"{}\" y=\"{}\" font-size=\"{}\">{}</text>",
            esc(&r.id),
            num(p.x),
            num(p.y),
            num(p.w),
            num(p.h),
            num(p.x + 6.0),
            num(r.rect.y + 7.0 + 11.5),
            num(r.title.font),
            esc(&r.title.text)
        );
        if let (Some(sub), Some(sp)) = (&r.sub, r.sub_plate) {
            let _ = writeln!(
                o,
                "<rect class=\"mask\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\"/><text class=\"t-dim\" x=\"{}\" y=\"{}\" font-size=\"{}\">{}</text>",
                num(sp.x),
                num(sp.y),
                num(sp.w),
                num(sp.h),
                num(sp.x + 6.0),
                num(r.rect.y + 7.0 + 11.0),
                num(sub.font),
                esc(&sub.text)
            );
        }
        o.push_str("</g>\n");
    }
    draw_legend(&mut o, &entries, &vb, rtl, legend, &item_w);
    o.push_str("</svg>\n");
    o
}

fn preset_attr(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c == ' ' {
                '-'
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect()
}

fn defs(scene: &Scene) -> String {
    let mut s = String::from("<defs>\n");
    let _ = writeln!(
        s,
        "<pattern id=\"grid\" width=\"40\" height=\"40\" patternUnits=\"userSpaceOnUse\"><path class=\"grid-line\" d=\"M 40 0 L 0 0 0 40\"/></pattern>"
    );
    let families: Vec<String> = if scene.nodes.iter().any(|n| n.status.is_some()) {
        STATUSES
            .iter()
            .map(|s| format!("st-{}", status_css(*s)))
            .collect()
    } else {
        KINDS.iter().map(|k| k.class().to_string()).collect()
    };
    for f in families {
        let _ = writeln!(
            s,
            "<marker id=\"m-{f}\" markerWidth=\"10\" markerHeight=\"7\" refX=\"9\" refY=\"3.5\" orient=\"auto\"><polygon class=\"m-{f}\" points=\"0 0, 10 3.5, 0 7\"/></marker>"
        );
    }
    s.push_str("</defs>\n");
    s
}

fn header(o: &mut String, scene: &Scene, vb: &Rect, rtl: bool) {
    let (x, anchor) = if rtl {
        (vb.right() - MARGIN, "end")
    } else {
        (vb.x + MARGIN, "start")
    };
    let _ = writeln!(
        o,
        "<text class=\"t-title\" x=\"{}\" y=\"{}\" font-size=\"{}\" text-anchor=\"{anchor}\">{}</text>",
        num(x),
        num(vb.y + 34.0),
        font::TITLE,
        esc(&scene.title)
    );
    if let Some(sub) = &scene.subtitle {
        let _ = writeln!(
            o,
            "<text class=\"t-muted\" x=\"{}\" y=\"{}\" font-size=\"11\" text-anchor=\"{anchor}\">{}</text>",
            num(x),
            num(vb.y + 52.0),
            esc(sub)
        );
    }
}

fn edge(o: &mut String, scene: &Scene, e: &SceneEdge, j: usize) {
    let fam = edge_family(e);
    let (from, to) = (&scene.nodes[e.from], &scene.nodes[e.to]);
    let label = e.full_label.as_deref().unwrap_or("");
    let _ = writeln!(
        o,
        "<g class=\"edge{}\" id=\"e{j}\" data-edge-id=\"e{j}\" data-edge-from=\"{}\" data-edge-to=\"{}\" data-edge-label=\"{}\" style=\"--step:{}\"><path class=\"a-{fam}\" d=\"{}\" marker-end=\"url(#m-{fam})\"/></g>",
        if e.animated { " flow" } else { "" },
        esc(&from.id),
        esc(&to.id),
        esc(label),
        j.min(12),
        path_d(&e.route.points)
    );
}

fn edge_label(o: &mut String, scene: &Scene, e: &SceneEdge, j: usize) {
    let (Some(text), Some(plate)) = (&e.label, e.route.label) else {
        return;
    };
    let (from, to) = (&scene.nodes[e.from], &scene.nodes[e.to]);
    let _ = writeln!(
        o,
        "<g class=\"lbl\" id=\"l{j}\" data-detail=\"context\" data-edge-from=\"{}\" data-edge-to=\"{}\"><rect class=\"mask\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\"/><text class=\"l-{}\" x=\"{}\" y=\"{}\" font-size=\"{}\" text-anchor=\"middle\">{}</text></g>",
        esc(&from.id),
        esc(&to.id),
        num(plate.x),
        num(plate.y),
        num(plate.w),
        num(plate.h),
        edge_family(e),
        num(plate.cx()),
        num(plate.y + 10.2),
        font::EDGE_LABEL,
        esc(text)
    );
}

fn node(o: &mut String, n: &SceneNode, i: usize) {
    let r = n.rect;
    let role = role_css(n.role);
    let box_class = match n.status {
        Some(s) => format!("box s-{}", status_css(s)),
        None => format!("box c-{role}"),
    };
    let aria = esc(&n.tooltip);
    let _ = write!(
        o,
        "<g id=\"n{i}\" class=\"node\" data-node-id=\"{}\" data-node-label=\"{}\" data-node-kind=\"{role}\"",
        esc(&n.id),
        esc(&n.label.text)
    );
    if let Some(sub) = &n.sublabel {
        let _ = write!(o, " data-node-sublabel=\"{}\"", esc(&sub.text));
    }
    if let Some(ctx) = &n.context {
        let _ = write!(o, " data-node-context=\"{}\"", esc(ctx));
    }
    if let Some(tag) = &n.tag {
        let _ = write!(o, " data-node-tag=\"{}\"", esc(&tag.text));
    }
    let _ = writeln!(
        o,
        " tabindex=\"0\" role=\"button\" aria-pressed=\"false\" aria-label=\"{aria}\" style=\"--step:{}\">",
        i.min(12)
    );
    let _ = writeln!(o, "<title>{aria}</title>");
    let _ = writeln!(
        o,
        "<rect class=\"mask\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"6\"/><rect class=\"{box_class}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"6\"/>",
        x = num(r.x),
        y = num(r.y),
        w = num(r.w),
        h = num(r.h)
    );
    let _ = writeln!(
        o,
        "<g class=\"sigil i-{role}\" transform=\"translate({} {}) scale(.6875)\">{}</g>",
        num(r.x + 6.0),
        num(r.y + 6.0),
        sigil(n.role)
    );
    let cx = r.cx();
    let ln = &n.label;
    let _ = writeln!(
        o,
        "<text class=\"t-primary\" x=\"{}\" y=\"{}\" font-size=\"{}\" font-weight=\"600\" text-anchor=\"middle\">{}</text>",
        num(cx),
        num(r.y + ln.y + ln.font * 0.35),
        num(ln.font),
        esc(&ln.text)
    );
    if let Some(s) = &n.sublabel {
        let _ = writeln!(
            o,
            "<text class=\"t-muted\" data-detail=\"context\" x=\"{}\" y=\"{}\" font-size=\"{}\" text-anchor=\"middle\">{}</text>",
            num(cx),
            num(r.y + s.y + s.font * 0.35),
            num(s.font),
            esc(&s.text)
        );
    }
    if let Some(t) = &n.tag {
        let _ = writeln!(
            o,
            "<text class=\"tag i-{role}\" data-detail=\"fine\" x=\"{}\" y=\"{}\" font-size=\"{}\" text-anchor=\"middle\">{}</text>",
            num(cx),
            num(r.y + t.y + t.font * 0.35),
            num(t.font),
            esc(&t.text)
        );
    }
    o.push_str("</g>\n");
}

fn draw_legend(
    o: &mut String,
    entries: &[(String, String, String)],
    vb: &Rect,
    rtl: bool,
    kind: LegendKind,
    item_w: &dyn Fn(&str, &str) -> f32,
) {
    if entries.is_empty() {
        return;
    }
    let y = vb.bottom() - 22.0;
    let mut cursor = if rtl {
        vb.right() - MARGIN
    } else {
        vb.x + MARGIN
    };
    o.push_str("<g class=\"legend\">\n");
    for (swatch, label, count) in entries {
        let w = item_w(label, count);
        let (sx, tx, anchor) = if rtl {
            (cursor - 16.0, cursor - 22.0, "end")
        } else {
            (cursor, cursor + 22.0, "start")
        };
        let rx = if kind == LegendKind::Status { 2.0 } else { 2.5 };
        let _ = writeln!(
            o,
            "<rect class=\"{swatch}\" x=\"{}\" y=\"{}\" width=\"16\" height=\"10\" rx=\"{rx}\"/><text class=\"{}\" x=\"{}\" y=\"{}\" font-size=\"{}\" text-anchor=\"{anchor}\">{}</text>",
            num(sx),
            num(y - 9.0),
            if kind == LegendKind::Status { swatch.replace("sw-", "l-") } else { "t-primary".to_string() },
            num(tx),
            num(y),
            font::LEGEND,
            esc(label)
        );
        if !count.is_empty() {
            let cx = if rtl {
                tx - text::width(label, font::LEGEND) - 8.0
            } else {
                tx + text::width(label, font::LEGEND) + 8.0
            };
            let _ = writeln!(
                o,
                "<text class=\"t-dim\" x=\"{}\" y=\"{}\" font-size=\"8\" font-weight=\"700\" text-anchor=\"{anchor}\">{}</text>",
                num(cx),
                num(y),
                esc(count)
            );
        }
        if rtl {
            cursor -= w;
        } else {
            cursor += w;
        }
    }
    o.push_str("</g>\n");
}
