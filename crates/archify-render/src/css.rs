//! The one stylesheet of every picture: theme variables, role/edge classes, hover and the
//! CSS-only motion. Colours come from `archify_style::Tokens`; nothing else holds a palette.

use crate::icons::role_css;
use archify_ir::{DiffStatus, VisualPreset};
use archify_style::{font, EdgeKind, Glow, Theme, Tokens, ROLE_ORDER};

pub(crate) const KINDS: [EdgeKind; 5] = [
    EdgeKind::Default,
    EdgeKind::Emphasis,
    EdgeKind::Security,
    EdgeKind::Dashed,
    EdgeKind::Dotted,
];

pub(crate) const STATUSES: [DiffStatus; 4] = [
    DiffStatus::Added,
    DiffStatus::Removed,
    DiffStatus::Modified,
    DiffStatus::Unchanged,
];

pub(crate) fn status_css(s: DiffStatus) -> &'static str {
    match s {
        DiffStatus::Added => "added",
        DiffStatus::Removed => "removed",
        DiffStatus::Modified => "modified",
        DiffStatus::Unchanged => "unchanged",
    }
}

fn dash(d: &[f32]) -> String {
    d.iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

/// `--name:value;` declarations of one theme.
fn vars(t: &Tokens) -> String {
    let mut s = String::new();
    let mut add = |name: &str, c: String| s.push_str(&format!("--{name}:{c};"));
    add("bg", t.bg.css());
    add("grid", t.grid.css());
    add("text", t.text.css());
    add("muted", t.muted.css());
    add("dim", t.dim.css());
    add("faint", t.faint.css());
    add("mask", t.mask.css());
    add("panel", t.panel.css());
    add("lane-stroke", t.lane_stroke.css());
    add("arrow", t.arrow.css());
    add("arrow-emphasis", t.arrow_emphasis.css());
    add("region-fill", t.region_fill.css());
    add("region-stroke", t.region_stroke.css());
    for r in ROLE_ORDER {
        let c = t.role(r);
        add(&format!("{}-fill", role_css(r)), c.fill.css());
        add(&format!("{}-stroke", role_css(r)), c.stroke.css());
    }
    for k in KINDS {
        let e = t.edge_style(k);
        add(&format!("e-{}", k.class()), e.color.css());
        add(&format!("l-{}", k.class()), e.label_color.css());
    }
    s
}

/// Hover glow of a box.
fn glow(t: &Tokens) -> String {
    match t.glow {
        Glow::None => "none".to_string(),
        Glow::Bloom(b) => format!("drop-shadow(0 0 {b}px var(--frontend-stroke))"),
        Glow::Paper { dy, blur } => {
            format!("drop-shadow(0 {dy}px {blur}px color-mix(in srgb,var(--text) 18%,transparent))")
        }
    }
}

/// Per-node dimming rules: hovering a box keeps it, its neighbours and their edges lit.
/// `adjacency[i] = (neighbour node indices, incident edge indices)`.
pub(crate) fn hover_rules(adjacency: &[(Vec<usize>, Vec<usize>)]) -> String {
    let mut s = String::new();
    if adjacency.len() > 300 {
        s.push_str("svg:has(.node:is(:hover,:focus-visible)) .edge,svg:has(.node:is(:hover,:focus-visible)) .lbl{opacity:.25}\n");
        return s;
    }
    for (i, (nodes, edges)) in adjacency.iter().enumerate() {
        let on = format!("svg:has(#n{i}:is(:hover,:focus-visible))");
        let keep_n: String = std::iter::once(i)
            .chain(nodes.iter().copied())
            .map(|n| format!(":not(#n{n})"))
            .collect();
        let keep_e: String = edges.iter().map(|e| format!(":not(#e{e})")).collect();
        let keep_l: String = edges.iter().map(|e| format!(":not(#l{e})")).collect();
        s.push_str(&format!(
            "{on} .node{keep_n},{on} .edge{keep_e},{on} .lbl{keep_l}{{opacity:.2}}\n"
        ));
    }
    s
}

pub(crate) fn stylesheet(preset: VisualPreset, hover: &str) -> String {
    let d = Tokens::new(preset, Theme::Dark);
    let l = Tokens::new(preset, Theme::Light);
    let (dark, light) = (vars(&d), vars(&l));
    let mut s = String::with_capacity(6000);
    s.push_str(&format!(
        "svg{{{dark}background:var(--bg);font-family:{}}}\n",
        font::MONO_STACK
    ));
    s.push_str(&format!("svg[data-theme=\"light\"]{{{light}}}\n"));
    s.push_str(&format!(
        "@media (prefers-color-scheme:light){{svg:not([data-theme=\"dark\"]):not([data-theme=\"light\"]){{{light}}}}}\n"
    ));
    s.push_str(".bg{fill:var(--bg)}\n");
    s.push_str(&format!(
        ".grid-line{{fill:none;stroke:var(--grid);stroke-width:.5;opacity:{}{}}}\n",
        d.grid_opacity,
        if d.grid_dash.is_empty() {
            String::new()
        } else {
            format!(";stroke-dasharray:{}", dash(d.grid_dash))
        }
    ));
    s.push_str(".mask{fill:var(--mask)}\n");
    s.push_str(&format!(
        ".region-frame{{fill:var(--region-fill);stroke:var(--region-stroke);stroke-width:1;stroke-dasharray:{}}}\n",
        dash(d.region_dash)
    ));
    s.push_str(".t-primary{fill:var(--text)}.t-muted{fill:var(--muted)}.t-dim{fill:var(--dim)}.t-region{fill:var(--region-stroke);font-weight:600}\n");
    s.push_str(&format!(
        ".t-title{{fill:var(--text);font-weight:700{}}}\n",
        if d.serif_titles {
            format!(";font-family:{}", font::SERIF_STACK)
        } else {
            String::new()
        }
    ));
    for r in ROLE_ORDER {
        let n = role_css(r);
        s.push_str(&format!(
            ".c-{n}{{fill:var(--{n}-fill);stroke:var(--{n}-stroke);stroke-width:1.5}}.i-{n}{{color:var(--{n}-stroke)}}.sw-{n}{{fill:var(--{n}-fill);stroke:var(--{n}-stroke);stroke-width:1}}\n"
        ));
    }
    s.push_str(".sigil{fill:none;stroke:currentColor;stroke-width:1.35;stroke-linecap:round;stroke-linejoin:round;opacity:.76}.tag{fill:currentColor}\n");
    let cap = if d.square_caps { "square" } else { "butt" };
    for k in KINDS {
        let e = d.edge_style(k);
        let c = k.class();
        let da = if e.dash.is_empty() {
            String::new()
        } else {
            format!(";stroke-dasharray:{}", dash(e.dash))
        };
        s.push_str(&format!(
            ".a-{c}{{fill:none;stroke:var(--e-{c});stroke-width:{}{da};stroke-linecap:{cap}}}.m-{c}{{fill:var(--e-{c})}}.l-{c}{{fill:var(--l-{c})}}\n",
            e.width
        ));
    }
    for st in STATUSES {
        let hex = st.neon_hex();
        let n = status_css(st);
        let (tint, extra) = match st {
            DiffStatus::Removed => ("rgba(244,63,94,.10)", ";stroke-dasharray:7 5"),
            DiffStatus::Unchanged => ("rgba(71,85,105,.12)", ""),
            DiffStatus::Added => ("rgba(34,197,94,.14)", ""),
            DiffStatus::Modified => ("rgba(245,158,11,.14)", ""),
        };
        s.push_str(&format!(
            ".s-{n}{{fill:{tint};stroke:{hex};stroke-width:1.8{extra}}}.a-st-{n}{{fill:none;stroke:{hex};stroke-width:1.8{extra}}}.m-st-{n}{{fill:{hex}}}.l-st-{n}{{fill:{hex}}}.sw-st-{n}{{fill:{hex}}}\n"
        ));
    }
    let g = glow(&d);
    s.push_str(&format!(
        "[data-node-id],[data-edge-from]{{transition:opacity .18s ease,filter .18s ease}}\n.node{{cursor:pointer;outline:none}}.node:is(:hover,:focus-visible){{filter:{g}}}\n"
    ));
    s.push_str(hover);
    s.push_str(".node:focus-visible .box{stroke-width:2.4}\n");
    s.push_str("@keyframes edge-flow{0%{stroke-dasharray:10 8;stroke-dashoffset:54;opacity:.42}88%{stroke-dasharray:10 8;stroke-dashoffset:0;opacity:1}100%{stroke-dashoffset:0;opacity:1}}\n");
    s.push_str("@keyframes node-pulse{0%,72%,100%{filter:none}18%,36%{filter:drop-shadow(0 0 8px var(--arrow-emphasis))}}\n@keyframes edge-dash{to{stroke-dashoffset:-12}}\n.edge path{animation:edge-flow 2.4s linear 1 backwards;animation-delay:calc(var(--step,0)*160ms)}\n.edge.flow path{stroke-dasharray:6 6;animation:edge-dash 1.2s linear infinite}\n.node{animation:node-pulse 3.6s ease-in-out 1 backwards;animation-delay:calc(var(--step,0)*160ms)}\n");
    s.push_str("@media (prefers-reduced-motion:reduce){*{animation:none!important;transition:none!important}}\n");
    s
}
