//! The eight palettes (4 presets x dark/light), transcribed from the Archify template.

use crate::{Glow, Rgba, RoleColors, Theme, Tokens, VisualPreset};

/// Raw CSS values of one palette. Role entries are `(fill, stroke)` in legend order:
/// frontend, backend, database, cloud, security, messagebus, external.
struct Raw {
    bg: &'static str,
    grid: &'static str,
    text: &'static str,
    muted: &'static str,
    dim: &'static str,
    faint: &'static str,
    panel: &'static str,
    panel_border: &'static str,
    lane_fill: &'static str,
    lane_stroke: &'static str,
    arrow: &'static str,
    arrow_emphasis: &'static str,
    mask: &'static str,
    roles: [(&'static str, &'static str); 7],
}

const CLASSIC_DARK: Raw = Raw {
    bg: "#020617",
    grid: "#1e293b",
    text: "#ffffff",
    muted: "#94a3b8",
    dim: "#475569",
    faint: "#7d8da1",
    panel: "rgba(15,23,42,.5)",
    panel_border: "#1e293b",
    lane_fill: "rgba(15,23,42,.22)",
    lane_stroke: "#334155",
    arrow: "#64748b",
    arrow_emphasis: "#34d399",
    mask: "#0f172a",
    roles: [
        ("rgba(8,51,68,.4)", "#22d3ee"),
        ("rgba(6,78,59,.4)", "#34d399"),
        ("rgba(76,29,149,.4)", "#a78bfa"),
        ("rgba(120,53,15,.3)", "#fbbf24"),
        ("rgba(136,19,55,.4)", "#fb7185"),
        ("rgba(251,146,60,.3)", "#fb923c"),
        ("rgba(30,41,59,.5)", "#94a3b8"),
    ],
};

const CLASSIC_LIGHT: Raw = Raw {
    bg: "#f8fafc",
    grid: "#e2e8f0",
    text: "#0f172a",
    muted: "#64748b",
    dim: "#94a3b8",
    faint: "#64748b",
    panel: "#ffffff",
    panel_border: "#e2e8f0",
    lane_fill: "rgba(248,250,252,.65)",
    lane_stroke: "#cbd5e1",
    arrow: "#94a3b8",
    arrow_emphasis: "#059669",
    mask: "#ffffff",
    roles: [
        ("rgba(34,211,238,.15)", "#0891b2"),
        ("rgba(52,211,153,.18)", "#059669"),
        ("rgba(167,139,250,.2)", "#7c3aed"),
        ("rgba(251,191,36,.18)", "#d97706"),
        ("rgba(251,113,133,.15)", "#e11d48"),
        ("rgba(251,146,60,.15)", "#ea580c"),
        ("rgba(148,163,184,.18)", "#64748b"),
    ],
};

const SIGNAL_DARK: Raw = Raw {
    bg: "#030711",
    grid: "#15233a",
    text: "#f5fbff",
    muted: "#9eb0c7",
    dim: "#52667f",
    faint: "#7890ad",
    panel: "rgba(6,14,28,.78)",
    panel_border: "#1d3350",
    lane_fill: "rgba(9,22,40,.5)",
    lane_stroke: "#2c4564",
    arrow: "#7890ad",
    arrow_emphasis: "#2dd4bf",
    mask: "#07101e",
    roles: [
        ("rgba(6,182,212,.14)", "#67e8f9"),
        ("rgba(16,185,129,.14)", "#5eead4"),
        ("rgba(139,92,246,.16)", "#c4b5fd"),
        ("rgba(245,158,11,.13)", "#fcd34d"),
        ("rgba(244,63,94,.13)", "#fda4af"),
        ("rgba(249,115,22,.13)", "#fdba74"),
        ("rgba(71,85,105,.24)", "#a5b4c7"),
    ],
};

const SIGNAL_LIGHT: Raw = Raw {
    bg: "#f4f9fc",
    grid: "#d4e5ee",
    text: "#102638",
    muted: "#587287",
    dim: "#8aa2b4",
    faint: "#668397",
    panel: "rgba(255,255,255,.82)",
    panel_border: "#bfd5e2",
    lane_fill: "rgba(232,243,248,.62)",
    lane_stroke: "#a9c5d5",
    arrow: "#7b97aa",
    arrow_emphasis: "#0d9488",
    mask: "#ffffff",
    roles: [
        ("rgba(6,182,212,.09)", "#0789a1"),
        ("rgba(5,150,105,.09)", "#087f69"),
        ("rgba(124,58,237,.09)", "#7254c7"),
        ("rgba(217,119,6,.08)", "#b9670b"),
        ("rgba(225,29,72,.08)", "#c53a59"),
        ("rgba(234,88,12,.08)", "#c65f27"),
        ("rgba(100,116,139,.1)", "#607a8c"),
    ],
};

const BLUEPRINT_DARK: Raw = Raw {
    bg: "#06131f",
    grid: "#17425a",
    text: "#e3f6ff",
    muted: "#91b8ca",
    dim: "#557e92",
    faint: "#739caf",
    panel: "rgba(7,27,43,.9)",
    panel_border: "#27627f",
    lane_fill: "rgba(10,43,66,.36)",
    lane_stroke: "#34799a",
    arrow: "#78a3b7",
    arrow_emphasis: "#64dfc1",
    mask: "#0a2031",
    roles: [
        ("rgba(26,157,193,.14)", "#66d9ef"),
        ("rgba(31,155,124,.13)", "#69dfbd"),
        ("rgba(120,105,196,.14)", "#b4a8ff"),
        ("rgba(197,145,43,.13)", "#ffd166"),
        ("rgba(199,76,104,.13)", "#ff8da1"),
        ("rgba(207,112,49,.13)", "#ffad66"),
        ("rgba(84,119,139,.18)", "#a7cad9"),
    ],
};

const BLUEPRINT_LIGHT: Raw = Raw {
    bg: "#edf7fa",
    grid: "#b5d5e1",
    text: "#123344",
    muted: "#4e7486",
    dim: "#86a6b4",
    faint: "#668c9d",
    panel: "rgba(249,253,255,.94)",
    panel_border: "#78aabd",
    lane_fill: "rgba(210,232,240,.42)",
    lane_stroke: "#83b2c4",
    arrow: "#6d93a5",
    arrow_emphasis: "#087f69",
    mask: "#f9fdff",
    roles: [
        ("rgba(8,145,178,.08)", "#087f9c"),
        ("rgba(5,128,101,.08)", "#08755f"),
        ("rgba(100,75,180,.08)", "#6757a8"),
        ("rgba(181,120,15,.08)", "#a86609"),
        ("rgba(190,48,80,.07)", "#b32f50"),
        ("rgba(196,81,22,.07)", "#b65120"),
        ("rgba(79,112,128,.08)", "#506f7e"),
    ],
};

const EDITORIAL_DARK: Raw = Raw {
    bg: "#181611",
    grid: "#39342a",
    text: "#f4eddf",
    muted: "#b9ae9b",
    dim: "#776e60",
    faint: "#9d917e",
    panel: "rgba(35,31,24,.96)",
    panel_border: "#625a4a",
    lane_fill: "rgba(52,46,35,.46)",
    lane_stroke: "#726957",
    arrow: "#948978",
    arrow_emphasis: "#dd6b3d",
    mask: "#231f18",
    roles: [
        ("rgba(43,133,142,.16)", "#7fc6c7"),
        ("rgba(63,132,92,.16)", "#8fc29e"),
        ("rgba(117,91,141,.17)", "#c0a4d0"),
        ("rgba(170,119,49,.16)", "#d8ad68"),
        ("rgba(157,69,65,.17)", "#df9085"),
        ("rgba(174,79,42,.16)", "#df946f"),
        ("rgba(126,115,96,.18)", "#b8ad99"),
    ],
};

const EDITORIAL_LIGHT: Raw = Raw {
    bg: "#f2eee5",
    grid: "#d8d0c2",
    text: "#242018",
    muted: "#6f6658",
    dim: "#a09788",
    faint: "#817767",
    panel: "rgba(251,248,241,.97)",
    panel_border: "#c4b9a6",
    lane_fill: "rgba(229,221,207,.42)",
    lane_stroke: "#b8aa94",
    arrow: "#8a806f",
    arrow_emphasis: "#bb4c23",
    mask: "#fbf8f1",
    roles: [
        ("rgba(31,117,126,.09)", "#287e84"),
        ("rgba(42,119,75,.09)", "#397b53"),
        ("rgba(105,75,130,.09)", "#765d86"),
        ("rgba(157,103,31,.1)", "#9a671f"),
        ("rgba(153,58,52,.09)", "#9e463f"),
        ("rgba(182,70,27,.09)", "#ad4b25"),
        ("rgba(101,91,75,.09)", "#746b5e"),
    ],
};

fn raw(preset: VisualPreset, theme: Theme) -> &'static Raw {
    match (preset, theme) {
        (VisualPreset::Classic, Theme::Dark) => &CLASSIC_DARK,
        (VisualPreset::Classic, Theme::Light) => &CLASSIC_LIGHT,
        (VisualPreset::SignalFlow, Theme::Dark) => &SIGNAL_DARK,
        (VisualPreset::SignalFlow, Theme::Light) => &SIGNAL_LIGHT,
        (VisualPreset::Blueprint, Theme::Dark) => &BLUEPRINT_DARK,
        (VisualPreset::Blueprint, Theme::Light) => &BLUEPRINT_LIGHT,
        (VisualPreset::Editorial, Theme::Dark) => &EDITORIAL_DARK,
        (VisualPreset::Editorial, Theme::Light) => &EDITORIAL_LIGHT,
    }
}

pub(crate) fn build(preset: VisualPreset, theme: Theme) -> Tokens {
    let r = raw(preset, theme);
    let p = Rgba::parse;
    let roles = r.roles.map(|(f, s)| RoleColors {
        fill: p(f),
        stroke: p(s),
    });
    let cloud = roles[3];
    let (grid_dash, grid_opacity, region_dash, glow): (&[f32], f32, &[f32], Glow) = match preset {
        VisualPreset::Classic | VisualPreset::SignalFlow => {
            (&[], 1.0, &[8.0, 4.0], Glow::Bloom(7.0))
        }
        VisualPreset::Blueprint => (&[1.0, 3.0], 0.78, &[12.0, 4.0, 2.0, 4.0], Glow::Bloom(3.0)),
        VisualPreset::Editorial => (
            &[1.0, 4.0],
            0.48,
            &[9.0, 4.0],
            Glow::Paper { dy: 2.0, blur: 2.0 },
        ),
    };
    // classic/signal regions are a faint amber wash; editorial mixes the cloud fill at 48%.
    let region_fill = match preset {
        VisualPreset::Editorial => cloud.fill.scale_alpha(0.48),
        _ => Rgba::new(251, 191, 36, 13),
    };
    let wash = (preset == VisualPreset::SignalFlow).then(|| match theme {
        Theme::Dark => (Rgba::new(34, 211, 238, 36), Rgba::new(139, 92, 246, 31)),
        Theme::Light => (Rgba::new(6, 182, 212, 31), Rgba::new(139, 92, 246, 20)),
    });
    Tokens {
        preset,
        theme,
        bg: p(r.bg),
        grid: p(r.grid),
        text: p(r.text),
        muted: p(r.muted),
        dim: p(r.dim),
        faint: p(r.faint),
        panel: p(r.panel),
        panel_border: p(r.panel_border),
        lane_fill: p(r.lane_fill),
        lane_stroke: p(r.lane_stroke),
        arrow: p(r.arrow),
        arrow_emphasis: p(r.arrow_emphasis),
        mask: p(r.mask),
        roles,
        node_radius: 6.0,
        region_radius: 12.0,
        grid_dash,
        grid_opacity,
        region_dash,
        region_fill,
        region_stroke: cloud.stroke,
        glow,
        beam_halo: matches!(preset, VisualPreset::Classic | VisualPreset::SignalFlow),
        square_caps: preset == VisualPreset::Blueprint,
        wash,
        serif_titles: preset == VisualPreset::Editorial,
    }
}
