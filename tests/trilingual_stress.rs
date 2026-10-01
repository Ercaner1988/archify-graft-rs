//! Integration test: Trilingual stress testing for Turkish, Arabic, and English.
//! Validates Unicode edge cases, diacritics stripping, case folding, and subword splitting.

use graft_i18n::{normalize_trilingual, TrilingualTokenizer};

#[test]
fn test_turkish_advanced_morphology_and_locale_safety() {
    // Test dotted/dotless I across varying cases
    let input = "İSTANBUL ılık ışık İpek İŞLEM";
    let tokens = TrilingualTokenizer::tokenize(input);

    // All three i forms (İ, I, ı) fold to one dotted i, so English and Turkish both match
    assert!(tokens.contains(&"istanbul".to_string()));
    assert!(tokens.contains(&"ilik".to_string()));
    assert!(tokens.contains(&"işik".to_string()));
    assert!(tokens.contains(&"ipek".to_string()));
    assert!(tokens.contains(&"işlem".to_string()));

    // Test Turkish specific suffixes and characters (ç, ğ, ö, ş, ü)
    let compound = "GeliştiriciArayüzü_Bileşeni";
    let compound_tokens = TrilingualTokenizer::tokenize(compound);
    assert!(compound_tokens.contains(&"geliştirici".to_string()));
    assert!(compound_tokens.contains(&"arayüzü".to_string()));
    assert!(compound_tokens.contains(&"bileşeni".to_string()));
}

#[test]
fn test_arabic_diacritics_and_alef_variations() {
    // Multi-harakat sentence: "بِسْمِ اللَّهِ الرَّحْمَٰنِ الرَّحِيمِ"
    let bismillah = "بِسْمِ اللَّهِ الرَّحْمَٰنِ الرَّحِيمِ";
    let tokens = TrilingualTokenizer::tokenize(bismillah);

    assert!(tokens.contains(&"بسم".to_string()));
    assert!(tokens.contains(&"الله".to_string()));
    assert!(tokens.contains(&"الرحمن".to_string()));
    assert!(tokens.contains(&"الرحيم".to_string()));

    // Alef variants normalization (أ, إ, آ -> ا) and Taa Marbuta (ة -> ه)
    let variants = "أُسْتَاذٌ إِبْرَاهِيمُ آيَةٌ مَدِينَةٌ";
    let normalized = normalize_trilingual(variants);
    assert!(normalized.contains("استاذ"));
    assert!(normalized.contains("ابراهيم"));
    assert!(normalized.contains("ايه"));
    assert!(normalized.contains("مدينه"));
}

#[test]
fn test_english_acronyms_and_identifier_boundaries() {
    let code_line = "parseXMLHTTPRequestAsync(newIOErrorPayload)";
    let tokens = TrilingualTokenizer::tokenize(code_line);

    assert!(tokens.contains(&"parse".to_string()));
    assert!(
        tokens.contains(&"xmlhttprequest".to_string())
            || tokens.contains(&"xml".to_string())
            || tokens.contains(&"http".to_string())
    );
    assert!(tokens.contains(&"async".to_string()));
}
