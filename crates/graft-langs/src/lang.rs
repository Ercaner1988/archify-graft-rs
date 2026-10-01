//! The languages this crate reads, and the masking that blanks comments and strings so
//! braces, parentheses and call-looking words inside them cannot fool the scanners.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    /// JavaScript and TypeScript (all extensions).
    Js,
    Py,
    Go,
    Java,
    /// C and C++.
    C,
}

impl Lang {
    pub fn of_path(path: &str) -> Option<Lang> {
        let ext = path.rsplit_once('.')?.1;
        Some(match ext {
            "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts" => Lang::Js,
            "py" => Lang::Py,
            "go" => Lang::Go,
            "java" => Lang::Java,
            "c" | "h" | "cpp" | "hpp" | "cc" | "cxx" | "hh" => Lang::C,
            _ => return None,
        })
    }

    pub fn tag(self) -> &'static str {
        match self {
            Lang::Js => "js",
            Lang::Py => "py",
            Lang::Go => "go",
            Lang::Java => "java",
            Lang::C => "c",
        }
    }
}

fn blank(out: &mut [u8], from: usize, to: usize) {
    for b in &mut out[from..to] {
        if *b != b'\n' && *b != b'\r' {
            *b = b' ';
        }
    }
}

/// Same length and same newlines as `src`; comment and string contents become spaces.
/// An unclosed `"` or `'` ends at the line break, so one stray quote (a regex literal, an
/// apostrophe in a preprocessor line) cannot swallow the rest of the file.
pub fn mask(lang: Lang, src: &str) -> String {
    let b = src.as_bytes();
    let n = b.len();
    let mut out = b.to_vec();
    let mut i = 0;
    while i < n {
        let c = b[i];
        let next = b.get(i + 1).copied();
        if (lang == Lang::Py && c == b'#') || (lang != Lang::Py && c == b'/' && next == Some(b'/'))
        {
            let s = i;
            while i < n && b[i] != b'\n' {
                i += 1;
            }
            blank(&mut out, s, i);
        } else if lang != Lang::Py && c == b'/' && next == Some(b'*') {
            let s = i;
            i += 2;
            while i < n && !(b[i] == b'*' && b.get(i + 1) == Some(&b'/')) {
                i += 1;
            }
            i = (i + 2).min(n);
            blank(&mut out, s, i);
        } else if lang == Lang::Py && (c == b'"' || c == b'\'') && b[i..].starts_with(&[c, c, c]) {
            let s = i;
            i += 3;
            while i < n && !b[i..].starts_with(&[c, c, c]) {
                i += if b[i] == b'\\' { 2 } else { 1 };
            }
            i = (i + 3).min(n);
            blank(&mut out, s + 3, i.saturating_sub(3).max(s + 3));
        } else if c == b'`' && matches!(lang, Lang::Js | Lang::Go) {
            let s = i + 1;
            i += 1;
            while i < n && b[i] != b'`' {
                i += if b[i] == b'\\' && lang == Lang::Js {
                    2
                } else {
                    1
                };
            }
            let e = i.min(n);
            blank(&mut out, s, e);
            i = (e + 1).min(n);
        } else if c == b'"' || c == b'\'' {
            let s = i + 1;
            i += 1;
            while i < n && b[i] != c && b[i] != b'\n' {
                i += if b[i] == b'\\' { 2 } else { 1 };
            }
            let e = i.min(n);
            blank(&mut out, s, e);
            i = if e < n && b[e] == c { e + 1 } else { e };
        } else {
            i += 1;
        }
    }
    String::from_utf8(out).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_and_strings_vanish_but_offsets_survive() {
        let src = "a(); // b()\n/* c() */ d(\"e()\", 'f()');\n`g()`";
        let m = mask(Lang::Js, src);
        assert_eq!(m.len(), src.len());
        assert_eq!(m.lines().count(), 3);
        assert!(m.contains("a();") && m.contains("d("));
        for gone in ["b()", "c()", "e()", "f()", "g()"] {
            assert!(!m.contains(gone), "{gone} in {m}");
        }
    }

    #[test]
    fn python_comments_and_docstrings_are_blanked_and_stray_quotes_stop_at_the_line() {
        let m = mask(
            Lang::Py,
            "x = f()  # g()\n\"\"\"doc h()\nmore i()\"\"\"\ny = j()\n",
        );
        assert!(m.contains("f()") && m.contains("j()"));
        assert!(!m.contains("g()") && !m.contains("h()") && !m.contains("i()"));
        let c = mask(Lang::C, "#error don't\nint main() { return k(); }\n");
        assert!(c.contains("k()"), "{c}");
    }

    #[test]
    fn language_is_chosen_by_extension() {
        assert_eq!(Lang::of_path("a/b.tsx"), Some(Lang::Js));
        assert_eq!(Lang::of_path("a/b.hpp"), Some(Lang::C));
        assert_eq!(Lang::of_path("a/b.rs"), None);
    }
}
