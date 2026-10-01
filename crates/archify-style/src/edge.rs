//! Edge variants (`Connection.line_style`) and their strokes.

use crate::{Rgba, SemanticRole, Tokens};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EdgeKind {
    #[default]
    Default,
    Emphasis,
    Security,
    Dashed,
    Dotted,
}

impl EdgeKind {
    /// Maps the free-form `line_style` string of the IR. Unknown values are plain edges.
    /// `"animated"` is a plain edge that flows (see [`EdgeKind::is_animated`]).
    pub fn from_line_style(s: &str) -> EdgeKind {
        let is = |k: &str| s.eq_ignore_ascii_case(k);
        if is("strong") || is("emphasis") {
            EdgeKind::Emphasis
        } else if is("security") {
            EdgeKind::Security
        } else if is("dashed") || is("async") {
            EdgeKind::Dashed
        } else if is("dotted") {
            EdgeKind::Dotted
        } else {
            EdgeKind::Default
        }
    }

    pub fn is_animated(line_style: &str) -> bool {
        line_style.eq_ignore_ascii_case("animated")
    }

    /// Stable marker/css class suffix.
    pub fn class(self) -> &'static str {
        match self {
            EdgeKind::Default => "default",
            EdgeKind::Emphasis => "emphasis",
            EdgeKind::Security => "security",
            EdgeKind::Dashed => "dashed",
            EdgeKind::Dotted => "dotted",
        }
    }
}

/// Resolved stroke of one edge variant for a preset/theme.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeStyle {
    pub color: Rgba,
    pub label_color: Rgba,
    pub width: f32,
    pub dash: &'static [f32],
}

impl Tokens {
    pub fn edge_style(&self, kind: EdgeKind) -> EdgeStyle {
        match kind {
            EdgeKind::Default => EdgeStyle {
                color: self.arrow,
                label_color: self.muted,
                width: 1.5,
                dash: &[],
            },
            EdgeKind::Emphasis => EdgeStyle {
                color: self.arrow_emphasis,
                label_color: self.stroke(SemanticRole::Backend),
                width: 1.8,
                dash: &[],
            },
            EdgeKind::Security => EdgeStyle {
                color: self.stroke(SemanticRole::Security),
                label_color: self.stroke(SemanticRole::Security),
                width: 1.5,
                dash: &[5.0, 5.0],
            },
            EdgeKind::Dashed => EdgeStyle {
                color: self.stroke(SemanticRole::Database),
                label_color: self.stroke(SemanticRole::Messagebus),
                width: 1.5,
                dash: &[4.0, 4.0],
            },
            EdgeKind::Dotted => EdgeStyle {
                color: self.arrow,
                label_color: self.muted,
                width: 1.4,
                dash: &[1.5, 4.0],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Theme, VisualPreset};

    #[test]
    fn line_style_mapping() {
        assert_eq!(EdgeKind::from_line_style(""), EdgeKind::Default);
        assert_eq!(EdgeKind::from_line_style("strong"), EdgeKind::Emphasis);
        assert_eq!(EdgeKind::from_line_style("DASHED"), EdgeKind::Dashed);
        assert!(EdgeKind::is_animated("animated"));
    }

    #[test]
    fn emphasis_is_thicker_and_uses_the_accent() {
        let t = Tokens::new(VisualPreset::Classic, Theme::Dark);
        let e = t.edge_style(EdgeKind::Emphasis);
        assert_eq!(e.color.hex(), "#34d399");
        assert!(e.width > t.edge_style(EdgeKind::Default).width);
    }
}
