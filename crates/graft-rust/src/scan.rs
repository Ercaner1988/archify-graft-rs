//! Item scanner for Rust source: functions with their body span, `impl` owners, types,
//! test scopes and code-line count. Brace-aware (strings and comments are masked
//! first), but deliberately not a parser: it reads item headers, not expressions.

use crate::calls::{artifact_name, body_facts, BodyFacts};
use crate::mask::Masked;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Fn,
    Struct,
    Enum,
    Trait,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub kind: ItemKind,
    pub name: String,
    /// `impl` or `trait` type a function belongs to.
    pub owner: Option<String>,
    /// 1-based line of the header and of the closing brace (same line for bodiless).
    pub line: usize,
    pub end_line: usize,
    /// The header's first source line, trimmed.
    pub sig: String,
    pub is_pub: bool,
    /// Inside `#[cfg(test)]`, marked `#[test]`, or in a test file.
    pub in_test: bool,
    pub facts: BodyFacts,
}

#[derive(Debug, Default)]
pub struct Scan {
    pub items: Vec<Item>,
    /// Non-blank, non-comment lines outside test code (the sandik-siniri metric).
    pub code_lines: usize,
}

struct Header {
    kind: HeaderKind,
    name: String,
    is_pub: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum HeaderKind {
    Item(ItemKind),
    Impl,
    Mod,
}

enum ScopeKind {
    Fn { item: usize, body: usize },
    Impl(String),
    Trait(String),
    Other,
}

struct Scope {
    kind: ScopeKind,
    test: bool,
}

#[derive(Default, Clone, Copy)]
struct Attrs {
    test: bool,
    cfg_test: bool,
}

struct Pending {
    header: Header,
    head_start: usize,
    line: usize,
    attrs: Attrs,
}

fn is_id(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80
}

fn word<'a>(s: &'a str, w: &str) -> Option<&'a str> {
    let rest = s.strip_prefix(w)?;
    (rest.bytes().next().is_none_or(|c| !is_id(c))).then_some(rest.trim_start())
}

fn ident(s: &str) -> &str {
    let s = s.strip_prefix("r#").unwrap_or(s);
    let end = s.bytes().position(|c| !is_id(c)).unwrap_or(s.len());
    &s[..end]
}

fn skip_parens(s: &str) -> &str {
    let mut depth = 0;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return s[i + 1..].trim_start();
                }
            }
            _ => {}
        }
    }
    ""
}

fn parse_header(rest: &str) -> Option<Header> {
    let mut s = rest;
    let mut is_pub = false;
    if let Some(r) = word(s, "pub") {
        is_pub = true;
        s = if r.starts_with('(') {
            skip_parens(r)
        } else {
            r
        };
    }
    loop {
        let before = s;
        for q in ["default", "const", "async", "unsafe", "extern", "safe"] {
            if let Some(r) = word(s, q) {
                s = r;
                if q == "extern" && s.starts_with('"') {
                    s = s[1..]
                        .trim_start_matches(' ')
                        .trim_start_matches('"')
                        .trim_start();
                }
            }
        }
        if s == before {
            break;
        }
    }
    let named = |kind, r: &str| {
        let name = ident(r);
        (!name.is_empty()).then(|| Header {
            kind,
            name: name.to_string(),
            is_pub,
        })
    };
    if let Some(r) = word(s, "fn") {
        return named(HeaderKind::Item(ItemKind::Fn), r);
    }
    if let Some(r) = word(s, "struct").or_else(|| word(s, "union")) {
        return named(HeaderKind::Item(ItemKind::Struct), r);
    }
    if let Some(r) = word(s, "enum") {
        return named(HeaderKind::Item(ItemKind::Enum), r);
    }
    if let Some(r) = word(s, "trait") {
        return named(HeaderKind::Item(ItemKind::Trait), r);
    }
    if let Some(r) = word(s, "mod") {
        return named(HeaderKind::Mod, r);
    }
    if word(s, "impl").is_some() || s.starts_with("impl<") {
        return Some(Header {
            kind: HeaderKind::Impl,
            name: String::new(),
            is_pub,
        });
    }
    None
}

/// Self type of an `impl` header text: `impl<T> Tr for Foo<T> where ..` -> `Foo`.
fn impl_owner(head: &str) -> String {
    let head = head.replace(['\n', '\r'], " ");
    let mut s = head.trim_start();
    s = s.strip_prefix("impl").unwrap_or(s).trim_start();
    if s.starts_with('<') {
        let mut depth = 0;
        for (i, c) in s.char_indices() {
            match c {
                '<' => depth += 1,
                '>' if !s[..i].ends_with('-') => {
                    depth -= 1;
                    if depth == 0 {
                        s = s[i + 1..].trim_start();
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    let s = s.split(" where ").next().unwrap_or(s);
    let ty = s.split(" for ").last().unwrap_or(s);
    let ty = ty
        .trim_start_matches(|c: char| matches!(c, '&' | '\'' | '!') || c.is_whitespace())
        .trim_start_matches("mut ")
        .trim_start_matches("dyn ");
    let ty = if let Some(r) = ty.strip_prefix('\'') {
        r.split_once(' ').map_or("", |(_, t)| t)
    } else {
        ty
    };
    let path = ty.split(['<', ' ', '{']).next().unwrap_or("");
    path.rsplit("::").next().unwrap_or("").to_string()
}

fn attr_flags(text: &str, flags: &mut Attrs) {
    let t: String = text.split_whitespace().collect();
    if t.contains("cfg(test)") || (t.starts_with("#[cfg(all(") && t.contains("test,")) {
        flags.cfg_test = true;
    }
    if t == "#[test]" || t.ends_with("::test]") || t.starts_with("#[test(") || t == "#[bench]" {
        flags.test = true;
    }
}

/// `const NAME: &str = "file.bin";` -> `NAME -> file.bin`.
fn const_artifacts(m: &Masked) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for lit in &m.literals {
        let Some(art) = artifact_name(&lit.text) else {
            continue;
        };
        let before = &m.text[..lit.offset];
        let start = before.rfind([';', '{', '}']).map_or(0, |p| p + 1);
        let stmt = before[start..].trim();
        let Some(rest) = word(stmt, "pub")
            .map(|r| {
                if r.starts_with('(') {
                    skip_parens(r)
                } else {
                    r
                }
            })
            .or(Some(stmt))
            .and_then(|s| word(s, "const").or_else(|| word(s, "static")))
        else {
            continue;
        };
        let name = ident(rest);
        if !name.is_empty() {
            out.insert(name.to_string(), art);
        }
    }
    out
}

/// `file_is_test`: the whole file is test code (`tests/`, `benches/`, `tests.rs`).
pub fn scan(src: &str, m: &Masked, file_is_test: bool) -> Scan {
    let text = m.text.as_str();
    let b = text.as_bytes();
    let n = b.len();
    let consts = const_artifacts(m);
    let raw_lines: Vec<&str> = src.lines().collect();
    let lits: Vec<(usize, &str)> = m
        .literals
        .iter()
        .map(|l| (l.offset, l.text.as_str()))
        .collect();

    let mut out = Scan::default();
    let mut scopes: Vec<Scope> = Vec::new();
    let mut pending: Option<Pending> = None;
    let mut attrs = Attrs::default();
    let (mut line, mut paren, mut stmt_start) = (1usize, 0usize, true);
    let (mut line_code, mut line_test_start) = (false, false);
    let in_test_now = |scopes: &[Scope], attrs: Attrs| {
        file_is_test || scopes.last().is_some_and(|s| s.test) || attrs.cfg_test || attrs.test
    };
    let mut i = 0;
    while i < n {
        let c = b[i];
        if c == b'\n' {
            let test_end = in_test_now(&scopes, attrs);
            if line_code && !(line_test_start || test_end) {
                out.code_lines += 1;
            }
            line += 1;
            line_code = false;
            stmt_start = true;
            i += 1;
            continue;
        }
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if !line_code {
            line_code = true;
            line_test_start = in_test_now(&scopes, attrs);
        }
        if stmt_start {
            stmt_start = false;
            if c == b'#' && matches!(b.get(i + 1), Some(b'[' | b'!')) {
                let mut depth = 0;
                let mut j = i;
                while j < n {
                    match b[j] {
                        b'[' => depth += 1,
                        b']' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        b'\n' => line += 1,
                        _ => {}
                    }
                    j += 1;
                }
                attr_flags(&text[i..(j + 1).min(n)], &mut attrs);
                i = (j + 1).min(n);
                stmt_start = true;
                continue;
            }
            if pending.is_none() && (c.is_ascii_alphabetic() || c == b'_') {
                match parse_header(&text[i..]) {
                    Some(h) => {
                        if let HeaderKind::Item(
                            k @ (ItemKind::Struct | ItemKind::Enum | ItemKind::Trait),
                        ) = h.kind
                        {
                            out.items.push(Item {
                                kind: k,
                                name: h.name.clone(),
                                owner: None,
                                line,
                                end_line: line,
                                sig: raw_lines
                                    .get(line - 1)
                                    .map_or(String::new(), |l| l.trim().to_string()),
                                is_pub: h.is_pub,
                                in_test: in_test_now(&scopes, attrs),
                                facts: BodyFacts::default(),
                            });
                        }
                        pending = Some(Pending {
                            header: h,
                            head_start: i,
                            line,
                            attrs,
                        });
                        attrs = Attrs::default();
                    }
                    None => attrs = Attrs::default(),
                }
            } else if pending.is_none() {
                attrs = Attrs::default();
            }
        }
        match c {
            b'{' if paren == 0 => {
                let test = in_test_now(&scopes, pending.as_ref().map_or(attrs, |p| p.attrs));
                let kind = match pending.take() {
                    Some(p) => match p.header.kind {
                        HeaderKind::Item(ItemKind::Fn) => {
                            let owner = match scopes.last().map(|s| &s.kind) {
                                Some(ScopeKind::Impl(t) | ScopeKind::Trait(t)) => Some(t.clone()),
                                _ => None,
                            };
                            out.items.push(Item {
                                kind: ItemKind::Fn,
                                name: p.header.name,
                                owner,
                                line: p.line,
                                end_line: p.line,
                                sig: raw_lines
                                    .get(p.line - 1)
                                    .map_or(String::new(), |l| l.trim().to_string()),
                                is_pub: p.header.is_pub,
                                in_test: test || p.attrs.test || p.attrs.cfg_test,
                                facts: BodyFacts::default(),
                            });
                            ScopeKind::Fn {
                                item: out.items.len() - 1,
                                body: i + 1,
                            }
                        }
                        HeaderKind::Impl => ScopeKind::Impl(impl_owner(&text[p.head_start..i])),
                        HeaderKind::Item(ItemKind::Trait) => ScopeKind::Trait(p.header.name),
                        _ => ScopeKind::Other,
                    },
                    None => ScopeKind::Other,
                };
                scopes.push(Scope { kind, test });
                stmt_start = true;
            }
            b'{' => {
                scopes.push(Scope {
                    kind: ScopeKind::Other,
                    test: scopes.last().is_some_and(|s| s.test),
                });
                stmt_start = true;
            }
            b'}' => {
                if let Some(Scope {
                    kind: ScopeKind::Fn { item, body },
                    ..
                }) = scopes.pop()
                {
                    let it = &mut out.items[item];
                    it.end_line = line;
                    let inner: Vec<(usize, &str)> = lits
                        .iter()
                        .filter(|(o, _)| *o >= body && *o < i)
                        .copied()
                        .collect();
                    if !it.in_test {
                        it.facts = body_facts(&text[body..i], &inner, &consts);
                    }
                }
                stmt_start = true;
            }
            b';' => {
                if paren == 0 {
                    pending = None;
                    attrs = Attrs::default();
                }
                stmt_start = true;
            }
            b'(' | b'[' => paren += 1,
            b')' | b']' => paren = paren.saturating_sub(1),
            _ => {}
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(src: &str) -> Scan {
        use crate::mask::mask;
        scan(src, &mask(src), false)
    }

    fn names(s: &Scan) -> Vec<String> {
        s.items
            .iter()
            .map(|i| match &i.owner {
                Some(o) => format!("{o}::{}", i.name),
                None => i.name.clone(),
            })
            .collect()
    }

    #[test]
    fn qualifiers_visibility_and_impl_owners_are_understood() {
        let s = run("pub(crate) async fn a() {}\npub async fn b() {}\nconst fn c() {}\nstruct Foo;\nimpl<T> Tr for Foo<T> where T: X {\n    pub fn new() -> Self { Foo }\n    fn hidden(&self) {}\n}\nfn d() {}\n");
        assert_eq!(
            names(&s),
            ["a", "b", "c", "Foo", "Foo::new", "Foo::hidden", "d"]
        );
        assert!(s.items[0].is_pub && !s.items[2].is_pub);
        assert_eq!(s.items[4].line, 6);
    }

    #[test]
    fn a_function_after_an_impl_is_a_function_not_a_method() {
        let s = run("struct A;\nimpl A {\n    fn m(&self) {}\n}\nfn free() {}\n");
        assert_eq!(s.items[1].owner.as_deref(), Some("A"));
        assert_eq!(s.items[2].owner, None);
    }

    #[test]
    fn test_modules_and_test_attributes_mark_items() {
        let s = run("fn real() { helper(); }\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() { real(); }\n    fn aux() {}\n}\n#[test]\nfn lone() {}\nfn after() {}\n");
        let t: Vec<(&str, bool)> = s
            .items
            .iter()
            .map(|i| (i.name.as_str(), i.in_test))
            .collect();
        assert_eq!(
            t,
            [
                ("real", false),
                ("t", true),
                ("aux", true),
                ("lone", true),
                ("after", false)
            ]
        );
    }

    #[test]
    fn braces_in_strings_do_not_break_scopes_and_multiline_signatures_work() {
        let s = run("fn a(\n    x: i32,\n) -> i32 {\n    let s = \"}}\";\n    b(x)\n}\nfn b(x: i32) -> i32 { x }\n");
        assert_eq!(names(&s), ["a", "b"]);
        assert_eq!((s.items[0].line, s.items[0].end_line), (1, 6));
        assert_eq!(s.items[0].facts.calls, ["b"]);
    }

    #[test]
    fn code_lines_skip_blanks_comments_and_test_code() {
        let s = run("// c\nfn a() {\n\n    x();\n}\n#[cfg(test)]\nmod tests {\n    fn t() {}\n}\n");
        assert_eq!(s.code_lines, 3);
    }

    #[test]
    fn data_files_and_io_verbs_reach_the_function_facts() {
        let s = run("const ENV: &str = \"envanter.bin\";\nfn yaz() { std::fs::write(dir().join(ENV), b); }\nfn oku() { let p = \"x/durumlar.json\"; std::fs::read(p); }\n");
        let yaz = &s.items[0];
        assert_eq!(
            (yaz.facts.writes, yaz.facts.artifacts.clone()),
            (true, vec!["envanter.bin".to_string()])
        );
        let oku = &s.items[1];
        assert!(oku.facts.reads);
        assert_eq!(oku.facts.artifacts, ["durumlar.json"]);
    }
}
