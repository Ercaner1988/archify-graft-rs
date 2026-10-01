//! Turkish-safe text folding for the search box.
//!
//! NFC first, then `I`, `İ`, `ı`, `i` all fold to `i` (the search is deliberately forgiving),
//! everything else is lowercased char by char. Bare `str::to_lowercase` is never used: it
//! turns `İ` into `i̇` (two scalars) and `I` into `i`, which breaks Turkish matching.

use unicode_normalization::UnicodeNormalization;

pub fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.nfc() {
        match c {
            'I' | 'İ' | 'ı' | 'i' => out.push('i'),
            c if c.is_ascii() => out.push(c.to_ascii_lowercase()),
            c => out.extend(c.to_lowercase()),
        }
    }
    out
}

/// True when every whitespace-separated word of `query` occurs in `haystack` (both folded).
pub fn matches(haystack: &str, query: &str) -> bool {
    let h = fold(haystack);
    fold(query).split_whitespace().all(|w| h.contains(w))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dotted_and_dotless_i_fold_together() {
        assert!(matches("İstanbul Çekirdek", "istanbul"));
        assert!(matches("ISPARTA", "ısparta"));
        assert!(matches("kırmızı", "KIRMIZI"));
        assert_eq!(fold("İ"), "i");
    }

    #[test]
    fn decomposed_input_matches_composed_text() {
        assert!(matches("şifre", "s\u{0327}ifre"));
        assert!(matches("pasli-çekirdek", "ÇEKİRDEK"));
        assert!(!matches("pano", "kasa"));
    }

    #[test]
    fn every_word_must_match() {
        assert!(matches("envanter oku", "oku env"));
        assert!(!matches("envanter oku", "oku yaz"));
    }
}
