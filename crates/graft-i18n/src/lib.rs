//! graft-i18n: Trilingual Subword Tokenizer and Normalizer (Turkish, Arabic, English).

use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedLanguage {
    Turkish,
    Arabic,
    English,
}

pub struct TrilingualTokenizer;

impl TrilingualTokenizer {
    /// Tokenizes input text according to Arabic, Turkish, and English morphological rules.
    pub fn tokenize(text: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let normalized: String = text.nfc().collect();

        for raw_word in normalized.split(|c: char| {
            c.is_whitespace()
                || c == '.'
                || c == '_'
                || c == '-'
                || c == '/'
                || c == '\\'
                || c == ':'
                || c == '('
                || c == ')'
                || c == '['
                || c == ']'
                || c == '{'
                || c == '}'
                || c == '<'
                || c == '>'
                || c == '"'
                || c == '\''
                || c == ';'
                || c == ','
        }) {
            if raw_word.is_empty() {
                continue;
            }

            let subwords = split_camel_case(raw_word);
            for subword in subwords {
                let cleaned = normalize_trilingual(&subword);
                if !cleaned.is_empty() && cleaned.len() > 1 {
                    tokens.push(cleaned);
                }
            }
        }

        tokens
    }
}

fn split_camel_case(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut prev_is_lower = false;

    for c in s.chars() {
        if c.is_uppercase() && prev_is_lower {
            if !current.is_empty() {
                parts.push(current);
                current = String::new();
            }
            prev_is_lower = false;
        } else if c.is_lowercase() {
            prev_is_lower = true;
        }
        current.push(c);
    }

    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

/// NFC + the one Turkish-safe fold for comparing names: `İ`, `I`, `ı` all become `i`,
/// everything else is lowercased char by char. Never use bare `str::to_lowercase` on
/// Turkish text (`I` -> `i` is wrong there, `İ` -> `i̇` is two chars).
pub fn fold_tr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.nfc() {
        if matches!(c, 'İ' | 'I' | 'ı') {
            out.push('i');
        } else {
            out.extend(c.to_lowercase());
        }
    }
    out
}

pub fn normalize_trilingual(s: &str) -> String {
    let mut out = String::with_capacity(s.len());

    for c in s.chars() {
        // Arabic Tashkeel / Harakat (Fatha, Damma, Kasra, Shadda, Sukun, Tanwin) -> Strip
        if ('\u{064B}'..='\u{0652}').contains(&c) || c == '\u{0670}' {
            continue;
        }

        // Arabic Alef normalization: أ, إ, آ -> ا
        if c == '\u{0622}' || c == '\u{0623}' || c == '\u{0625}' {
            out.push('\u{0627}');
            continue;
        }
        // Arabic Taa Marbuta: ة -> ه
        if c == '\u{0629}' {
            out.push('\u{0647}');
            continue;
        }
        // Arabic Alif Maqsura: ى -> ي
        if c == '\u{0649}' {
            out.push('\u{064A}');
            continue;
        }

        // Turkish Locale-Safe Case Folding. One dotted `i` for İ, I and ı: folding `I`
        // to `ı` broke English (`Input` -> `ınput`) and the query side folds the same way,
        // so `ışık`, `Işık` and `ISIK` still meet.
        if matches!(c, 'İ' | 'I' | 'ı') {
            out.push('i');
            continue;
        }

        // Standard Unicode lowercase
        for lc in c.to_lowercase() {
            out.push(lc);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turkish_dotless_i_safety() {
        let tokens = TrilingualTokenizer::tokenize("İşlemIşığı ayrıştırıcı");
        assert!(tokens.contains(&"işlem".to_string()));
        assert!(tokens.contains(&"işiği".to_string()));
        // Query typed with a dotless ı meets the same token.
        assert_eq!(normalize_trilingual("ışığı"), normalize_trilingual("IŞIĞI"));
    }

    #[test]
    fn english_capital_i_is_not_turned_dotless() {
        let tokens = TrilingualTokenizer::tokenize("Input Index ID");
        assert!(tokens.contains(&"input".to_string()));
        assert!(tokens.contains(&"index".to_string()));
    }

    #[test]
    fn fold_tr_maps_all_three_i_forms_to_one() {
        assert_eq!(fold_tr("ISIK İşık ışık Gui"), "isik işik işik gui");
    }

    #[test]
    fn test_arabic_normalization() {
        let tokens = TrilingualTokenizer::tokenize("العَرَبِيَّة أَحْمَد");
        assert!(tokens.contains(&"العربيه".to_string()));
        assert!(tokens.contains(&"احمد".to_string()));
    }

    #[test]
    fn test_camel_case_splitting() {
        let tokens = TrilingualTokenizer::tokenize("parseASTNodeAndEmit");
        assert!(tokens.contains(&"parse".to_string()));
    }
}
