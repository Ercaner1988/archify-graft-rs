//! Blanks comments, string and char literals out of Rust source, byte for byte: the
//! result has the same length and the same newlines as the input, so offsets and line
//! numbers stay valid, and brace/paren counting can no longer be fooled by `"{"`.

/// A string literal found while masking.
#[derive(Debug, Clone)]
pub struct Literal {
    /// Byte offset of the opening quote in the source.
    pub offset: usize,
    /// Raw text between the quotes (escapes untouched).
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct Masked {
    pub text: String,
    pub literals: Vec<Literal>,
}

fn is_id(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn blank(out: &mut [u8], from: usize, to: usize) {
    for b in &mut out[from..to] {
        if *b != b'\n' && *b != b'\r' {
            *b = b' ';
        }
    }
}

/// Length in bytes of the UTF-8 char that starts with `first`.
fn utf8_len(first: u8) -> usize {
    match first {
        0xF0..=0xFF => 4,
        0xE0..=0xEF => 3,
        0xC0..=0xDF => 2,
        _ => 1,
    }
}

/// `r"`, `r#"`, `br##"`: returns `(index of the opening quote, number of #)`.
fn raw_string_at(b: &[u8], i: usize) -> Option<(usize, usize)> {
    let prev_ok = |p: usize| p == 0 || !is_id(b[p - 1]);
    let start_ok = prev_ok(i) || (b[i - 1] == b'b' && prev_ok(i - 1));
    if b[i] != b'r' || !start_ok {
        return None;
    }
    let mut j = i + 1;
    while b.get(j) == Some(&b'#') {
        j += 1;
    }
    (b.get(j) == Some(&b'"')).then_some((j, j - i - 1))
}

pub fn mask(src: &str) -> Masked {
    let b = src.as_bytes();
    let n = b.len();
    let mut out = b.to_vec();
    let mut literals = Vec::new();
    let mut i = 0;
    while i < n {
        match b[i] {
            b'/' if b.get(i + 1) == Some(&b'/') => {
                let s = i;
                while i < n && b[i] != b'\n' {
                    i += 1;
                }
                blank(&mut out, s, i);
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                let s = i;
                let mut depth = 0;
                while i < n {
                    if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
                        depth += 1;
                        i += 2;
                    } else if b[i] == b'*' && b.get(i + 1) == Some(&b'/') {
                        depth -= 1;
                        i += 2;
                        if depth == 0 {
                            break;
                        }
                    } else {
                        i += 1;
                    }
                }
                blank(&mut out, s, i.min(n));
            }
            b'r' if raw_string_at(b, i).is_some() => {
                let (quote, hashes) = raw_string_at(b, i).unwrap_or((i, 0));
                let closer: Vec<u8> = std::iter::once(b'"')
                    .chain(std::iter::repeat_n(b'#', hashes))
                    .collect();
                let body = quote + 1;
                let end = b[body..]
                    .windows(closer.len())
                    .position(|w| w == closer.as_slice())
                    .map_or(n, |p| body + p);
                literals.push(Literal {
                    offset: quote,
                    text: src[body..end].to_string(),
                });
                blank(&mut out, body, end);
                i = (end + closer.len()).min(n);
            }
            b'"' => {
                let body = i + 1;
                let mut j = body;
                while j < n && b[j] != b'"' {
                    j += if b[j] == b'\\' { 2 } else { 1 };
                }
                let end = j.min(n);
                literals.push(Literal {
                    offset: i,
                    text: src[body..end].to_string(),
                });
                blank(&mut out, body, end);
                i = (end + 1).min(n);
            }
            b'\'' => {
                // Char literal, or a lifetime (`'a`), which is left alone.
                let close = if b.get(i + 1) == Some(&b'\\') {
                    (i + 3..n).find(|&j| b[j] == b'\'')
                } else {
                    let next = i + 1 + b.get(i + 1).map_or(1, |&c| utf8_len(c));
                    (b.get(next) == Some(&b'\'')).then_some(next)
                };
                match close {
                    Some(c) => {
                        blank(&mut out, i + 1, c);
                        i = c + 1;
                    }
                    None => i += 1,
                }
            }
            _ => i += 1,
        }
    }
    Masked {
        // Only ASCII bytes were overwritten or whole multi-byte runs blanked to spaces.
        text: String::from_utf8(out).unwrap_or_default(),
        literals,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_strings_and_chars_are_blanked_but_offsets_survive() {
        let src = "let a = \"{ x }\"; // tail {\nlet c = '{'; /* { */ let l: &'a str;";
        let m = mask(src);
        assert_eq!(m.text.len(), src.len());
        assert_eq!(m.text.matches('\n').count(), 1);
        assert!(!m.text.contains('{'), "{}", m.text);
        assert!(
            m.text.contains("&'a str"),
            "lifetime must survive: {}",
            m.text
        );
        assert_eq!(m.literals.len(), 1);
        assert_eq!(m.literals[0].text, "{ x }");
    }

    #[test]
    fn raw_strings_escapes_and_non_ascii_are_safe() {
        let src = "let p = r#\"a\"b.bin\"#; let q = \"ç\\\"ğ.json\"; fn çağır() {}";
        let m = mask(src);
        assert_eq!(m.text.len(), src.len());
        assert!(m.text.contains("fn çağır() {}"));
        assert_eq!(m.literals[0].text, "a\"b.bin");
        assert_eq!(m.literals[1].text, "ç\\\"ğ.json");
    }

    #[test]
    fn nested_block_comments_end_where_they_should() {
        let m = mask("a /* x /* y */ z */ b");
        assert_eq!(m.text.trim().replace(' ', ""), "ab");
    }
}
