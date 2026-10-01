//! Text metrics and fitting shared by the canvas and the SVG renderer.
//!
//! The diagram face is monospace, so a string's width is `units * 0.6 * font_px`. Fullwidth
//! and CJK characters count two units (as in the original `textUnits`).

use crate::font::MONO_ADVANCE;

/// Width units of one char: 2 for CJK / fullwidth / emoji, 0 for combining marks, else 1.
pub fn char_units(c: char) -> f32 {
    let u = c as u32;
    match u {
        0x0300..=0x036F | 0x064B..=0x065F | 0x200B..=0x200F => 0.0,
        0x1100..=0x115F
        | 0x2E80..=0xA4CF
        | 0xAC00..=0xD7A3
        | 0xF900..=0xFAFF
        | 0xFE30..=0xFE6F
        | 0xFF00..=0xFF60
        | 0xFFE0..=0xFFE6
        | 0x1F300..=0x1FAFF => 2.0,
        _ => 1.0,
    }
}

pub fn units(s: &str) -> f32 {
    s.chars().map(char_units).sum()
}

/// Rendered width in px of `s` at `font_px`.
pub fn width(s: &str, font_px: f32) -> f32 {
    units(s) * MONO_ADVANCE * font_px
}

/// Edge-label plate: `max(30, units * 4.8 + 10)` wide.
pub fn edge_label_width(s: &str) -> f32 {
    (units(s) * 4.8 + 10.0).max(30.0)
}

pub const EDGE_LABEL_HEIGHT: f32 = 14.0;

/// Result of fitting a string into a width budget.
#[derive(Debug, Clone, PartialEq)]
pub struct Fitted {
    pub text: String,
    pub font: f32,
    pub truncated: bool,
}

/// Fits `s` into `max_px`: first shrinks the font down to `min_font` (0.1px steps, like the
/// original `fittedNodeFontSize`), then truncates. Names with an extension keep it and lose
/// the middle (`00ebbd47…bbc.json`), everything else ends in an ellipsis.
pub fn fit(s: &str, max_px: f32, font: f32, min_font: f32) -> Fitted {
    let u = units(s);
    if u == 0.0 || u * MONO_ADVANCE * font <= max_px {
        return Fitted {
            text: s.to_string(),
            font,
            truncated: false,
        };
    }
    let shrunk = (((max_px / (u * MONO_ADVANCE)) * 10.0).floor() / 10.0).max(min_font);
    if shrunk >= min_font && u * MONO_ADVANCE * shrunk <= max_px {
        return Fitted {
            text: s.to_string(),
            font: shrunk.min(font),
            truncated: false,
        };
    }
    let font = min_font.min(font);
    let budget = (max_px / (MONO_ADVANCE * font)).floor().max(2.0);
    Fitted {
        text: truncate_units(s, budget),
        font,
        truncated: true,
    }
}

/// Truncates to at most `budget` width units, keeping a short extension tail.
pub fn truncate_units(s: &str, budget: f32) -> String {
    if units(s) <= budget {
        return s.to_string();
    }
    let chars: Vec<char> = s.chars().collect();
    let tail_len = extension_len(&chars);
    let (head_budget, tail): (f32, &[char]) = if tail_len > 0 && (tail_len as f32) + 4.0 <= budget {
        (
            budget - tail_len as f32 - 1.0,
            &chars[chars.len() - tail_len..],
        )
    } else {
        (budget - 1.0, &[])
    };
    let mut out = String::new();
    let mut used = 0.0;
    for &c in &chars {
        let w = char_units(c);
        if used + w > head_budget {
            break;
        }
        used += w;
        out.push(c);
    }
    out.push('…');
    out.extend(tail);
    out
}

/// Length (chars) of a trailing `.ext` of 1..=6 alphanumerics, plus 3 chars of name before
/// it so the tail reads like `bbc.json`; 0 when the string has no extension.
fn extension_len(chars: &[char]) -> usize {
    let dot = match chars.iter().rposition(|&c| c == '.') {
        Some(i) if i > 0 => i,
        _ => return 0,
    };
    let ext = chars.len() - dot - 1;
    if (1..=6).contains(&ext) && chars[dot + 1..].iter().all(|c| c.is_ascii_alphanumeric()) {
        (ext + 1 + 3).min(chars.len() - 1)
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cjk_counts_double() {
        assert_eq!(units("ab"), 2.0);
        assert_eq!(units("中文"), 4.0);
        assert_eq!(units("e\u{0301}"), 1.0);
    }

    #[test]
    fn short_text_untouched_and_long_shrinks_then_truncates() {
        let a = fit("Pano", 150.0, 11.0, 8.0);
        assert!(!a.truncated && a.font == 11.0);
        // 24 chars at 11px = 158px: shrinks to ~10.4 without truncation
        let b = fit("abcdefghijklmnopqrstuvwx", 150.0, 11.0, 8.0);
        assert!(!b.truncated && b.font < 11.0 && b.font >= 8.0);
        let c = fit(&"x".repeat(80), 150.0, 11.0, 8.0);
        assert!(c.truncated && width(&c.text, c.font) <= 150.0 + 0.01);
    }

    #[test]
    fn extension_survives_truncation() {
        let name = "00ebbd47387f9a1c2e3d4b5a69788796a5b4c3d2e1f00112233445566778899aabbc.json";
        let f = fit(name, 150.0, 11.0, 8.0);
        assert!(f.truncated);
        assert!(f.text.ends_with("bbc.json"), "{}", f.text);
        assert!(f.text.contains('…'));
        assert!(width(&f.text, f.font) <= 150.0 + 0.01);
    }

    #[test]
    fn turkish_text_is_not_split_mid_char() {
        let f = fit("çağırıcıdeğişkenuzunad", 60.0, 8.0, 8.0);
        assert!(f.truncated);
        assert!(f.text.chars().all(|c| c != '\u{fffd}'));
    }

    #[test]
    fn edge_label_plate_follows_the_reference_formula() {
        assert!((edge_label_width("envanter oku") - 67.6).abs() < 0.01);
        assert_eq!(edge_label_width("ab"), 30.0);
    }
}
