//! archify-style: the single source of colour, typography and preset chrome.
//!
//! Both the egui canvas and the SVG renderer read their colours from [`Tokens`]; there is
//! no second colour table anywhere. Values are the exact tokens of the original Archify
//! template (`--bg`, `--mask`, `--frontend-fill`, ...) for the four presets and both themes.

mod color;
mod edge;
mod palette;
pub mod text;

pub use archify_ir::{SemanticRole, VisualPreset};
pub use color::Rgba;
pub use edge::{EdgeKind, EdgeStyle};

/// All seven roles in legend order.
pub const ROLE_ORDER: [SemanticRole; 7] = [
    SemanticRole::Frontend,
    SemanticRole::Backend,
    SemanticRole::Database,
    SemanticRole::Cloud,
    SemanticRole::Security,
    SemanticRole::Messagebus,
    SemanticRole::External,
];

/// Dark or light variant of a preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

impl Theme {
    pub fn toggled(self) -> Theme {
        match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        }
    }
}

/// Fill and stroke of one semantic role.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoleColors {
    pub fill: Rgba,
    pub stroke: Rgba,
}

/// How a hovered/selected box is lit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Glow {
    /// No filter at all.
    None,
    /// `drop-shadow(0 0 blur colour)` in the role colour.
    Bloom(f32),
    /// `drop-shadow(0 dy blur text@18%)`: editorial paper shadow.
    Paper { dy: f32, blur: f32 },
}

/// Every colour and chrome value a renderer needs for one preset and theme.
#[derive(Debug, Clone)]
pub struct Tokens {
    pub preset: VisualPreset,
    pub theme: Theme,
    pub bg: Rgba,
    pub grid: Rgba,
    pub text: Rgba,
    pub muted: Rgba,
    pub dim: Rgba,
    pub faint: Rgba,
    pub panel: Rgba,
    pub panel_border: Rgba,
    pub lane_fill: Rgba,
    pub lane_stroke: Rgba,
    pub arrow: Rgba,
    pub arrow_emphasis: Rgba,
    /// Opaque plate painted under every translucent box so edges never show through.
    pub mask: Rgba,
    pub roles: [RoleColors; 7],
    pub node_radius: f32,
    pub region_radius: f32,
    /// Dash pattern of the 40px background grid (empty = solid).
    pub grid_dash: &'static [f32],
    pub grid_opacity: f32,
    /// Dash pattern of region frames.
    pub region_dash: &'static [f32],
    pub region_fill: Rgba,
    pub region_stroke: Rgba,
    pub glow: Glow,
    /// Flowing beams carry a soft halo only in the glowing presets.
    pub beam_halo: bool,
    pub square_caps: bool,
    /// Faint corner washes behind the canvas (signal-flow only).
    pub wash: Option<(Rgba, Rgba)>,
    /// Editorial titles use a serif face.
    pub serif_titles: bool,
}

impl Default for Tokens {
    fn default() -> Self {
        Tokens::new(VisualPreset::Editorial, Theme::Dark)
    }
}

impl Tokens {
    pub fn new(preset: VisualPreset, theme: Theme) -> Tokens {
        palette::build(preset, theme)
    }

    pub fn role(&self, role: SemanticRole) -> RoleColors {
        self.roles[role_index(role)]
    }

    pub fn stroke(&self, role: SemanticRole) -> Rgba {
        self.role(role).stroke
    }

    /// Short label shown in the preset picker.
    pub fn preset_name(preset: VisualPreset) -> &'static str {
        match preset {
            VisualPreset::Classic => "Classic",
            VisualPreset::SignalFlow => "Signal Flow",
            VisualPreset::Blueprint => "Blueprint",
            VisualPreset::Editorial => "Editorial",
        }
    }

    pub fn is_dark(&self) -> bool {
        self.theme == Theme::Dark
    }
}

pub fn role_index(role: SemanticRole) -> usize {
    match role {
        SemanticRole::Frontend => 0,
        SemanticRole::Backend => 1,
        SemanticRole::Database => 2,
        SemanticRole::Cloud => 3,
        SemanticRole::Security => 4,
        SemanticRole::Messagebus => 5,
        SemanticRole::External => 6,
    }
}

/// Legend / tooltip name of a role for a locale (`tr`, `ar`, anything else = English).
pub fn role_name(role: SemanticRole, locale: &str) -> &'static str {
    let (tr, ar, en) = match role {
        SemanticRole::Frontend => ("Arayüz", "واجهة", "Frontend"),
        SemanticRole::Backend => ("Çekirdek", "خادم", "Backend"),
        SemanticRole::Database => ("Depolama", "قاعدة بيانات", "Database"),
        SemanticRole::Cloud => ("Bulut", "سحابة", "Cloud"),
        SemanticRole::Security => ("Güvenlik", "أمان", "Security"),
        SemanticRole::Messagebus => ("Mesaj yolu", "ناقل رسائل", "Message bus"),
        SemanticRole::External => ("Dış", "خارجي", "External"),
    };
    match locale {
        "tr" => tr,
        "ar" => ar,
        _ => en,
    }
}

/// Type scale (px) of the diagram text, taken from the reference output.
pub mod font {
    pub const NODE_LABEL: f32 = 11.0;
    pub const NODE_SUBLABEL: f32 = 9.0;
    pub const NODE_TAG: f32 = 7.0;
    pub const EDGE_LABEL: f32 = 8.0;
    pub const REGION_TITLE: f32 = 9.0;
    pub const LEGEND: f32 = 10.0;
    pub const TITLE: f32 = 20.0;
    /// Advance width of the monospace face in em.
    pub const MONO_ADVANCE: f32 = 0.6;
    pub const MONO_STACK: &str = "'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, Consolas, 'DejaVu Sans Mono', 'Liberation Mono', monospace";
    pub const SERIF_STACK: &str = "Georgia, 'Times New Roman', serif";
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRESETS: [VisualPreset; 4] = [
        VisualPreset::Classic,
        VisualPreset::SignalFlow,
        VisualPreset::Blueprint,
        VisualPreset::Editorial,
    ];

    #[test]
    fn editorial_dark_matches_the_template() {
        let t = Tokens::new(VisualPreset::Editorial, Theme::Dark);
        assert_eq!(t.bg.hex(), "#181611");
        assert_eq!(t.mask.hex(), "#231f18");
        assert_eq!(t.arrow_emphasis.hex(), "#dd6b3d");
        assert_eq!(t.stroke(SemanticRole::Database).hex(), "#c0a4d0");
        let f = t.role(SemanticRole::Frontend).fill;
        assert_eq!((f.r, f.g, f.b), (43, 133, 142));
        assert!((f.a as f32 / 255.0 - 0.16).abs() < 0.01);
    }

    #[test]
    fn classic_dark_and_light_differ_and_match_original() {
        let d = Tokens::new(VisualPreset::Classic, Theme::Dark);
        let l = Tokens::new(VisualPreset::Classic, Theme::Light);
        assert_eq!(d.bg.hex(), "#020617");
        assert_eq!(l.bg.hex(), "#f8fafc");
        assert_eq!(d.stroke(SemanticRole::Frontend).hex(), "#22d3ee");
        assert_eq!(l.stroke(SemanticRole::Frontend).hex(), "#0891b2");
    }

    #[test]
    fn every_preset_theme_has_distinct_role_strokes_and_opaque_mask() {
        for p in PRESETS {
            for th in [Theme::Dark, Theme::Light] {
                let t = Tokens::new(p, th);
                assert_eq!(t.mask.a, 255);
                assert_eq!(t.bg.a, 255);
                let mut seen = std::collections::HashSet::new();
                for r in ROLE_ORDER {
                    assert!(
                        seen.insert(t.stroke(r).hex()),
                        "{p:?}/{th:?} duplicate stroke"
                    );
                }
            }
        }
    }
}
