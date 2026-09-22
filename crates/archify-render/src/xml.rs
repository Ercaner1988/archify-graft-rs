//! Text helpers for SVG output.

/// Escapes text for XML content and attribute values. Module and file names can
/// legally contain `&` or `<`; unescaped they would break the whole document.
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// Cuts `s` to at most `max` characters, ending in `…` when something was dropped.
pub fn fit(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esc_covers_the_five_xml_specials() {
        assert_eq!(esc(r#"a&b<c>"d""#), "a&amp;b&lt;c&gt;&quot;d&quot;");
        assert_eq!(esc("çağırır"), "çağırır");
    }

    #[test]
    fn fit_truncates_by_characters_not_bytes() {
        assert_eq!(fit("kısa", 10), "kısa");
        assert_eq!(fit("çağırıcıdeğişken", 6), "çağır…");
    }
}
