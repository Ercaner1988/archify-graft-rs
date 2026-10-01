//! Declarations with body spans: functions, methods, classes, enums, interfaces. One
//! brace-aware scanner serves JS/TS, Go, Java and C/C++ (only the header shapes differ);
//! Python is read by indentation.

use crate::calls::call_tokens;
use crate::lang::Lang;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Fn,
    Class,
    Enum,
    Interface,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub kind: Kind,
    pub name: String,
    /// Class / receiver type a function belongs to.
    pub owner: Option<String>,
    pub line: usize,
    pub end_line: usize,
    pub sig: String,
    pub is_pub: bool,
    pub calls: Vec<String>,
}

struct Header {
    kind: Kind,
    name: String,
    owner: Option<String>,
    is_pub: bool,
    /// Go struct / interface and C++ namespaces open scopes that are not classes.
    plain_scope: bool,
    /// Offset (in the header slice) just after `=>` of an arrow function.
    arrow: Option<usize>,
}

fn is_id(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$' || b >= 0x80
}

fn word<'a>(s: &'a str, w: &str) -> Option<&'a str> {
    let r = s.strip_prefix(w)?;
    r.bytes()
        .next()
        .is_none_or(|c| !is_id(c))
        .then_some(r.trim_start())
}

fn ident(s: &str) -> &str {
    let end = s.bytes().position(|c| !is_id(c)).unwrap_or(s.len());
    &s[..end]
}

fn upper(s: &str) -> bool {
    s.chars().next().is_some_and(char::is_uppercase)
}

fn hdr(kind: Kind, name: &str, owner: Option<&str>, is_pub: bool) -> Option<Header> {
    (!name.is_empty()).then(|| Header {
        kind,
        name: name.to_string(),
        owner: owner.map(str::to_string),
        is_pub,
        plain_scope: false,
        arrow: None,
    })
}

const NOT_FN: &[&str] = &[
    "if",
    "for",
    "while",
    "switch",
    "catch",
    "return",
    "else",
    "do",
    "new",
    "throw",
    "try",
    "constructor_call",
    "synchronized",
    "sizeof",
    "case",
    "goto",
    "typedef",
    "using",
    "template",
    "delete",
    "defined",
    "super",
    "this",
    "elif",
    "with",
    "assert",
];

fn js_header(s: &str, class: Option<&str>) -> Option<Header> {
    let mut t = s.trim_start();
    let mut export = false;
    loop {
        let before = t;
        for p in [
            "export",
            "default",
            "declare",
            "async",
            "static",
            "public",
            "private",
            "protected",
            "readonly",
            "abstract",
            "override",
        ] {
            if let Some(r) = word(t, p) {
                export |= p == "export";
                t = r;
            }
        }
        for p in ["get", "set"] {
            // `get foo()` is an accessor, `get(` is a method called get.
            if let Some(r) = t.strip_prefix(p).and_then(|r| r.strip_prefix(' ')) {
                if r.trim_start().bytes().next().is_some_and(is_id) {
                    t = r.trim_start();
                }
            }
        }
        if t == before {
            break;
        }
    }
    let t = t.strip_prefix('*').unwrap_or(t).trim_start();
    if let Some(r) = word(t, "function") {
        let r = r.strip_prefix('*').unwrap_or(r).trim_start();
        return hdr(Kind::Fn, ident(r), None, export);
    }
    if let Some(r) = word(t, "class") {
        return hdr(Kind::Class, ident(r), None, export);
    }
    if let Some(r) = word(t, "interface") {
        return hdr(Kind::Interface, ident(r), None, export);
    }
    if let Some(r) = word(t, "enum").or_else(|| word(t, "const").and_then(|r| word(r, "enum"))) {
        return hdr(Kind::Enum, ident(r), None, export);
    }
    let decl = word(t, "const")
        .or_else(|| word(t, "let"))
        .or_else(|| word(t, "var"));
    let (name_src, is_decl) = match decl {
        Some(r) => (r, true),
        None => (t, false),
    };
    let name = ident(name_src);
    if name.is_empty() || NOT_FN.contains(&name) {
        return None;
    }
    let after = name_src[name.len()..].trim_start();
    if (is_decl || class.is_some()) && (after.starts_with('=') || after.starts_with(':')) {
        if let Some(eq) = after.find('=') {
            let rhs = after[eq + 1..].trim_start();
            let rhs = word(rhs, "async").unwrap_or(rhs);
            if rhs.starts_with("function") {
                return hdr(Kind::Fn, name, class, export);
            }
            if let Some(p) = after.find("=>") {
                let mut h = hdr(Kind::Fn, name, class, export)?;
                h.arrow = Some(s.len() - after.len() + p + 2);
                return Some(h);
            }
            return None;
        }
    }
    if class.is_some() && (after.starts_with('(') || after.starts_with('<')) && !is_decl {
        return hdr(
            Kind::Fn,
            name,
            class,
            !s.trim_start().starts_with("private"),
        );
    }
    None
}

fn go_header(s: &str) -> Option<Header> {
    let t = s.trim_start();
    if let Some(r) = word(t, "func") {
        let (owner, r) = if let Some(rest) = r.strip_prefix('(') {
            let close = rest.find(')')?;
            let ty = rest[..close]
                .split_whitespace()
                .last()?
                .trim_start_matches('*');
            (Some(ident(ty).to_string()), rest[close + 1..].trim_start())
        } else {
            (None, r)
        };
        let name = ident(r);
        return hdr(Kind::Fn, name, owner.as_deref(), upper(name));
    }
    let r = word(t, "type")?;
    let name = ident(r);
    let rest = r[name.len()..].trim_start();
    let kind = if word(rest, "struct").is_some() {
        Kind::Class
    } else if word(rest, "interface").is_some() {
        Kind::Interface
    } else {
        return None;
    };
    let mut h = hdr(kind, name, None, upper(name))?;
    h.plain_scope = true;
    Some(h)
}

fn java_header(s: &str, class: Option<&str>) -> Option<Header> {
    let mut t = s.trim_start();
    let mut public = false;
    loop {
        let before = t;
        if t.starts_with('@') && !t.starts_with("@interface") {
            let after = ident(&t[1..]);
            t = t[1 + after.len()..].trim_start();
            if t.starts_with('(') {
                let close = t.find(')').map_or(t.len(), |c| c + 1);
                t = t[close..].trim_start();
            }
        }
        for p in [
            "public",
            "private",
            "protected",
            "static",
            "final",
            "abstract",
            "synchronized",
            "native",
            "default",
            "sealed",
            "strictfp",
            "transient",
            "volatile",
            "non-sealed",
        ] {
            if let Some(r) = word(t, p) {
                public |= p == "public";
                t = r;
            }
        }
        if t == before {
            break;
        }
    }
    for (kw, kind) in [
        ("class", Kind::Class),
        ("interface", Kind::Interface),
        ("@interface", Kind::Interface),
        ("enum", Kind::Enum),
        ("record", Kind::Class),
    ] {
        if let Some(r) = word(t, kw) {
            return hdr(kind, ident(r), None, public);
        }
    }
    let cls = class?;
    let paren = t.find('(')?;
    let before = t[..paren].trim_end();
    if before.contains(['=', '.', '"'])
        || before.starts_with("return")
        || before.starts_with("new ")
    {
        return None;
    }
    let before = before.trim_end_matches('>');
    let name = before
        .rsplit(|c: char| c.is_whitespace() || c == '>')
        .next()
        .unwrap_or("");
    let tokens = before.split_whitespace().count();
    if name.is_empty() || NOT_FN.contains(&name) || !name.bytes().all(is_id) {
        return None;
    }
    (tokens >= 2 || name == cls)
        .then(|| hdr(Kind::Fn, name, Some(cls), public))
        .flatten()
}

fn c_header(s: &str, class: Option<&str>) -> Option<Header> {
    let t = s.trim_start();
    let first = ident(t);
    if NOT_FN.contains(&first) && first != "template" {
        return None;
    }
    for (kw, kind) in [
        ("struct", Kind::Class),
        ("class", Kind::Class),
        ("union", Kind::Class),
        ("enum", Kind::Enum),
    ] {
        if let Some(mut r) = word(t, kw) {
            r = word(r, "class").or_else(|| word(r, "struct")).unwrap_or(r);
            let name = ident(r);
            let rest = r[name.len()..].trim_start();
            let defines = rest.starts_with('{') || rest.starts_with(':') || rest.is_empty();
            if defines && !name.is_empty() && !t.contains('(') {
                return hdr(kind, name, None, true);
            }
        }
    }
    let line = t.trim_end();
    if line.ends_with(';') || line.starts_with("template") {
        return None;
    }
    let paren = t.find('(')?;
    let before = t[..paren].trim_end();
    if before.contains(['=', '.', '"']) || before.contains("->") {
        return None;
    }
    let name_full = before
        .rsplit(|c: char| c.is_whitespace() || c == '*' || c == '&')
        .next()
        .unwrap_or("");
    let (owner, name) = match name_full.rsplit_once("::") {
        Some((o, n)) => (Some(o.rsplit("::").next().unwrap_or(o)), n),
        None => (class, name_full),
    };
    let tokens = before.split_whitespace().count();
    let name = name.trim_start_matches('~');
    if name.is_empty() || NOT_FN.contains(&name) || !name.bytes().all(is_id) {
        return None;
    }
    let has_type = tokens >= 2 || before.contains(['*', '&']);
    let ctor = owner.is_some_and(|o| o == name) || name_full.contains("::");
    (has_type || ctor).then(|| Header {
        kind: Kind::Fn,
        name: name.to_string(),
        owner: owner.map(str::to_string),
        is_pub: !before.starts_with("static"),
        plain_scope: false,
        arrow: None,
    })
}

enum Scope {
    Fn { item: usize, body: usize },
    Class(String),
    Other,
}

struct Frame {
    scope: Scope,
    in_fn: bool,
}

struct Pending {
    h: Header,
    line: usize,
}

fn sig_of(lines: &[&str], line: usize) -> String {
    let s = lines.get(line - 1).map_or("", |l| l.trim());
    s.chars().take(200).collect()
}

fn scan_brace(lang: Lang, src: &str, m: &str) -> Vec<Item> {
    let b = m.as_bytes();
    let n = b.len();
    let lines: Vec<&str> = src.lines().collect();
    let mut items: Vec<Item> = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    let mut pending: Option<Pending> = None;
    let (mut line, mut paren, mut stmt) = (1usize, 0usize, true);
    let mut i = 0;
    while i < n {
        let c = b[i];
        if c == b'\n' {
            line += 1;
            stmt = true;
            i += 1;
            continue;
        }
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if lang == Lang::C && stmt && c == b'#' {
            // Preprocessor line, with backslash continuations.
            while i < n && !(b[i] == b'\n' && b[i - 1] != b'\\') {
                if b[i] == b'\n' {
                    line += 1;
                }
                i += 1;
            }
            continue;
        }
        let in_fn = stack.last().is_some_and(|f| f.in_fn);
        if stmt {
            stmt = false;
            if pending.as_ref().is_some_and(|p| line > p.line + 40) {
                pending = None;
            }
            if pending.is_none() && (lang == Lang::Js || !in_fn) {
                let end = m[i..].find('\n').map_or(n, |e| i + e);
                let class = match stack.last() {
                    Some(Frame {
                        scope: Scope::Class(c),
                        in_fn: false,
                    }) => Some(c.as_str()),
                    _ => None,
                };
                let slice = &m[i..end];
                let h = match lang {
                    Lang::Js => js_header(slice, class),
                    Lang::Go => go_header(slice),
                    Lang::Java => java_header(slice, class),
                    Lang::C => c_header(slice, class),
                    _ => None,
                };
                if let Some(mut h) = h {
                    if let Some(a) = h.arrow.take() {
                        // Expression body on the same line: `const f = (x) => g(x);`
                        let rest = slice[a..].trim_start();
                        if !rest.starts_with('{') {
                            items.push(Item {
                                kind: Kind::Fn,
                                name: h.name.clone(),
                                owner: h.owner.clone(),
                                line,
                                end_line: line,
                                sig: sig_of(&lines, line),
                                is_pub: h.is_pub,
                                calls: call_tokens(lang, rest),
                            });
                            i = end;
                            continue;
                        }
                    }
                    pending = Some(Pending { h, line });
                }
            }
        }
        match c {
            b'{' if paren == 0 => {
                let inherited = stack.last().is_some_and(|f| f.in_fn);
                let (scope, in_fn) = match pending.take() {
                    Some(Pending { h, line: hl }) => match h.kind {
                        Kind::Fn => {
                            items.push(Item {
                                kind: Kind::Fn,
                                name: h.name,
                                owner: h.owner,
                                line: hl,
                                end_line: hl,
                                sig: sig_of(&lines, hl),
                                is_pub: h.is_pub,
                                calls: Vec::new(),
                            });
                            (
                                Scope::Fn {
                                    item: items.len() - 1,
                                    body: i + 1,
                                },
                                true,
                            )
                        }
                        k => {
                            items.push(Item {
                                kind: k,
                                name: h.name.clone(),
                                owner: None,
                                line: hl,
                                end_line: hl,
                                sig: sig_of(&lines, hl),
                                is_pub: h.is_pub,
                                calls: Vec::new(),
                            });
                            let scope = if h.plain_scope {
                                Scope::Other
                            } else {
                                Scope::Class(h.name)
                            };
                            (scope, false)
                        }
                    },
                    None => (Scope::Other, inherited),
                };
                stack.push(Frame { scope, in_fn });
                stmt = true;
            }
            b'{' => {
                let in_fn = stack.last().is_some_and(|f| f.in_fn);
                stack.push(Frame {
                    scope: Scope::Other,
                    in_fn,
                });
                stmt = true;
            }
            b'}' => {
                if let Some(Frame {
                    scope: Scope::Fn { item, body },
                    ..
                }) = stack.pop()
                {
                    items[item].end_line = line;
                    items[item].calls = call_tokens(lang, &m[body..i]);
                }
                stmt = true;
            }
            b';' => {
                if paren == 0 {
                    pending = None;
                }
                stmt = true;
            }
            b'(' | b'[' => paren += 1,
            b')' | b']' => paren = paren.saturating_sub(1),
            _ => {}
        }
        i += 1;
    }
    items
}

fn indent_of(line: &str) -> usize {
    line.chars()
        .take_while(|c| c.is_whitespace())
        .map(|c| if c == '\t' { 4 } else { 1 })
        .sum()
}

fn scan_py(src: &str, m: &str) -> Vec<Item> {
    let raw: Vec<&str> = src.lines().collect();
    let lines: Vec<&str> = m.lines().collect();
    let mut items = Vec::new();
    // (indent, class name) of the classes we are inside.
    let mut classes: Vec<(usize, String)> = Vec::new();
    let mut li = 0;
    while li < lines.len() {
        let l = lines[li];
        let t = l.trim_start();
        let ind = indent_of(l);
        if t.is_empty() {
            li += 1;
            continue;
        }
        while classes.last().is_some_and(|(i, _)| ind <= *i) {
            classes.pop();
        }
        let def = word(t, "def").or_else(|| word(t, "async").and_then(|r| word(r, "def")));
        if let Some(r) = def {
            let name = ident(r);
            // Header runs until the line that closes the parentheses and ends with `:`.
            let mut end = li;
            let mut depth: i32 = 0;
            loop {
                for c in lines[end].chars() {
                    depth += i32::from(c == '(') - i32::from(c == ')');
                }
                if depth <= 0 || end + 1 >= lines.len() {
                    break;
                }
                end += 1;
            }
            let mut last = end;
            let mut k = end + 1;
            while k < lines.len() {
                if lines[k].trim().is_empty() {
                    k += 1;
                    continue;
                }
                if indent_of(lines[k]) <= ind {
                    break;
                }
                last = k;
                k += 1;
            }
            let body = lines[end + 1..=last.max(end)].join("\n");
            let owner = classes
                .last()
                .filter(|(ci, _)| *ci < ind)
                .map(|c| c.1.clone());
            items.push(Item {
                kind: Kind::Fn,
                name: name.to_string(),
                owner,
                line: li + 1,
                end_line: last + 1,
                sig: raw
                    .get(li)
                    .map_or(String::new(), |s| s.trim().chars().take(200).collect()),
                is_pub: !name.starts_with('_'),
                calls: call_tokens(Lang::Py, &body),
            });
            li += 1;
            continue;
        }
        if let Some(r) = word(t, "class") {
            let name = ident(r);
            items.push(Item {
                kind: Kind::Class,
                name: name.to_string(),
                owner: None,
                line: li + 1,
                end_line: li + 1,
                sig: raw
                    .get(li)
                    .map_or(String::new(), |s| s.trim().chars().take(200).collect()),
                is_pub: !name.starts_with('_'),
                calls: Vec::new(),
            });
            classes.push((ind, name.to_string()));
        }
        li += 1;
    }
    items
}

pub fn scan(lang: Lang, src: &str, masked: &str) -> Vec<Item> {
    if lang == Lang::Py {
        scan_py(src, masked)
    } else {
        scan_brace(lang, src, masked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::mask;

    fn run(lang: Lang, src: &str) -> Vec<Item> {
        scan(lang, src, &mask(lang, src))
    }

    fn names(v: &[Item]) -> Vec<String> {
        v.iter()
            .map(|i| match &i.owner {
                Some(o) => format!("{o}.{}", i.name),
                None => i.name.clone(),
            })
            .collect()
    }

    #[test]
    fn js_functions_arrows_classes_and_methods_with_bodies() {
        let src = "export async function load(p) { return read(p); }\nconst parse = (x) => tokenize(x);\nexport const run = async (a) => {\n  go(a);\n};\nclass Box extends Base {\n  constructor(a) { super(a); }\n  open() { this.load(); }\n  static of(x) { return new Box(x); }\n}\nfunction later() {}\n";
        let v = run(Lang::Js, src);
        assert_eq!(
            names(&v),
            [
                "load",
                "parse",
                "run",
                "Box",
                "Box.constructor",
                "Box.open",
                "Box.of",
                "later"
            ]
        );
        assert_eq!(v[0].calls, ["read"]);
        assert_eq!(v[1].calls, ["tokenize"]);
        assert_eq!(v[2].calls, ["go"]);
        assert_eq!(v[5].calls, [".load"]);
        assert!(v[3].kind == Kind::Class && v[7].owner.is_none());
        assert!(v[0].is_pub && !v[7].is_pub);
    }

    #[test]
    fn python_defs_use_indentation_for_bodies_and_owners() {
        let src = "def top(a):\n    return helper(a)\n\nclass Svc:\n    def run(self,\n            x):\n        self.step(x)\n        out = other(x)\n\n    def _hidden(self):\n        pass\n\ndef after():\n    pass\n";
        let v = run(Lang::Py, src);
        assert_eq!(names(&v), ["top", "Svc", "Svc.run", "Svc._hidden", "after"]);
        assert_eq!(v[0].calls, ["helper"]);
        assert_eq!(v[2].calls, [".step", "other"]);
        assert!(!v[3].is_pub && v[4].owner.is_none());
    }

    #[test]
    fn go_methods_get_their_receiver_type_and_exported_names_are_public() {
        let src = "package p\ntype Server struct {\n  n int\n}\ntype Runner interface {\n  Run()\n}\nfunc (s *Server) Start() error {\n  return boot(s)\n}\nfunc helper(a int) int { return a }\n";
        let v = run(Lang::Go, src);
        assert_eq!(names(&v), ["Server", "Runner", "Server.Start", "helper"]);
        assert_eq!(v[2].calls, ["boot"]);
        assert!(v[2].is_pub && !v[3].is_pub);
    }

    #[test]
    fn java_methods_constructors_and_annotations_are_found_inside_classes_only() {
        let src = "package a;\npublic class Svc {\n  private int n = compute();\n  public Svc(int n) { init(n); }\n  @Override\n  public String name() { return fmt(n); }\n  static <T> List<T> all() { return load(); }\n  void run() {\n    if (n > 0) { work(); }\n    new Thread(() -> { tick(); }).start();\n  }\n}\n";
        let v = run(Lang::Java, src);
        assert_eq!(
            names(&v),
            ["Svc", "Svc.Svc", "Svc.name", "Svc.all", "Svc.run"]
        );
        assert_eq!(v[1].calls, ["init"]);
        assert!(
            v[4].calls.contains(&"work".to_string()) && v[4].calls.contains(&"tick".to_string())
        );
    }

    #[test]
    fn c_and_cpp_functions_prototypes_macros_and_class_scope() {
        let src = "#include <stdio.h>\n#define LOG(x) \\\n  printf(x)\nint proto(int a);\nstatic int add(int a, int b) {\n  return helper(a) + b;\n}\nvoid Widget::draw(int x)\n{\n  paint(x);\n}\nclass Box {\n public:\n  int area() const { return calc(w); }\n};\nint main(void) { add(1, 2); }\n";
        let v = run(Lang::C, src);
        assert_eq!(names(&v), ["add", "Widget.draw", "Box", "Box.area", "main"]);
        assert_eq!(v[0].calls, ["helper"]);
        assert_eq!(v[1].calls, ["paint"]);
        assert!(!v[0].is_pub && v[4].is_pub);
    }
}
