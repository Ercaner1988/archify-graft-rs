//! What a file brings into scope: `import`, `require`, `from x import y`, Go imports,
//! Java imports, `#include`. Each binding is a fact `b:<local>|<original>|<spec>` on the
//! file; `original` is `*` for a whole module / namespace, `local` is empty for a glob.

use crate::lang::Lang;

fn quoted(s: &str) -> Option<&str> {
    let q = s.find(['"', '\'', '`'])?;
    let quote = s[q..].chars().next()?;
    let rest = &s[q + 1..];
    Some(&rest[..rest.find(quote)?])
}

fn b(local: &str, orig: &str, spec: &str) -> String {
    format!("b:{local}|{orig}|{spec}")
}

/// `{a, b as c, type D}` -> `(local, original)` pairs.
fn named(list: &str) -> Vec<(String, String)> {
    list.trim_matches(|c: char| c == '{' || c == '}' || c.is_whitespace())
        .split(',')
        .filter_map(|p| {
            let p = p.trim().trim_start_matches("type ").trim();
            if p.is_empty() {
                return None;
            }
            let mut it = p.split(" as ");
            let orig = it.next()?.trim();
            let local = it.next().map_or(orig, str::trim);
            Some((local.to_string(), orig.to_string()))
        })
        .collect()
}

fn js(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let text: Vec<&str> = src.lines().collect();
    let mut i = 0;
    while i < text.len() {
        let t = text[i].trim();
        if t.starts_with("import ")
            || t.starts_with("export ") && t.contains(" from ")
            || t.starts_with("export {")
        {
            // Join a multi-line `import {\n a,\n b\n} from 'x'`.
            let mut stmt = t.to_string();
            while !stmt.contains(" from ")
                && !stmt.contains("import '")
                && !stmt.contains("import \"")
                && i + 1 < text.len()
                && stmt.len() < 2000
            {
                i += 1;
                stmt.push(' ');
                stmt.push_str(text[i].trim());
                if text[i].contains(" from ") {
                    break;
                }
            }
            if let Some((clause, tail)) = stmt.split_once(" from ") {
                if let Some(spec) = quoted(tail) {
                    let clause = clause
                        .trim_start_matches("import ")
                        .trim_start_matches("export ")
                        .trim_start_matches("type ")
                        .trim();
                    if clause.starts_with('*') {
                        let ns = clause.split(" as ").nth(1).unwrap_or("").trim();
                        out.push(b(ns, "*", spec));
                    } else {
                        let (default, braces) = match clause.find('{') {
                            Some(p) => (clause[..p].trim().trim_end_matches(','), &clause[p..]),
                            None => (clause, ""),
                        };
                        if !default.is_empty() {
                            out.push(b(default, "default", spec));
                        }
                        for (local, orig) in named(braces) {
                            out.push(b(&local, &orig, spec));
                        }
                    }
                }
            }
        } else if let Some(p) = t.find("require(") {
            if let Some(spec) = quoted(&t[p..]) {
                let lhs = t[..p]
                    .trim_start_matches("export ")
                    .trim_start_matches("const ")
                    .trim_start_matches("let ")
                    .trim_start_matches("var ")
                    .trim_end_matches(['=', ' ']);
                if lhs.starts_with('{') {
                    for (local, orig) in named(&lhs.replace(':', " as ")) {
                        out.push(b(&local, &orig, spec));
                    }
                } else if !lhs.is_empty()
                    && lhs
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'$')
                {
                    out.push(b(lhs, "*", spec));
                }
            }
        }
        i += 1;
    }
    out
}

fn py(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let text: Vec<&str> = src.lines().collect();
    let mut i = 0;
    while i < text.len() {
        let t = text[i].trim();
        if let Some(rest) = t.strip_prefix("from ") {
            if let Some((module, names)) = rest.split_once(" import ") {
                let mut names = names.to_string();
                while names.contains('(') && !names.contains(')') && i + 1 < text.len() {
                    i += 1;
                    names.push(' ');
                    names.push_str(text[i].trim());
                }
                let module = module.trim();
                let names = names
                    .split('#')
                    .next()
                    .unwrap_or("")
                    .replace(['(', ')'], "");
                if names.trim() == "*" {
                    out.push(b("", "*", module));
                }
                for (local, orig) in named(&names).into_iter().filter(|(l, _)| l != "*") {
                    out.push(b(&local, &orig, module));
                    // `from pkg import mod`: `mod` may be a module file.
                    let sep = if module.ends_with('.') { "" } else { "." };
                    out.push(b(&local, "*", &format!("{module}{sep}{orig}")));
                }
            }
        } else if let Some(rest) = t.strip_prefix("import ") {
            for part in rest.split(',') {
                let mut it = part.trim().split(" as ");
                let module = it.next().unwrap_or("").trim();
                if module.is_empty() {
                    continue;
                }
                let local = it
                    .next()
                    .map_or_else(|| module.split('.').next().unwrap_or(module), str::trim);
                out.push(b(local, "*", module));
            }
        }
        i += 1;
    }
    out
}

fn go(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_block = false;
    for line in src.lines() {
        let t = line.trim();
        let stmt = if in_block {
            if t.starts_with(')') {
                in_block = false;
                continue;
            }
            t
        } else if let Some(r) = t.strip_prefix("import") {
            let r = r.trim();
            if r.starts_with('(') {
                in_block = !r.contains(')');
                continue;
            }
            r
        } else {
            continue;
        };
        if let Some(spec) = quoted(stmt) {
            let alias = stmt
                .split_whitespace()
                .next()
                .filter(|a| !a.starts_with('"') && !a.starts_with('`'));
            let last = spec.rsplit('/').next().unwrap_or(spec);
            let last = if last.len() > 1
                && last.starts_with('v')
                && last[1..].bytes().all(|c| c.is_ascii_digit())
            {
                spec.rsplit('/').nth(1).unwrap_or(last)
            } else {
                last
            };
            match alias {
                Some("_") | Some(".") => {}
                Some(a) => out.push(b(a, "*", spec)),
                None => out.push(b(last, "*", spec)),
            }
        }
    }
    out
}

fn java(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in src.lines() {
        let Some(rest) = line.trim().strip_prefix("import ") else {
            continue;
        };
        let rest = rest.trim_end_matches(';').trim();
        let (is_static, path) = match rest.strip_prefix("static ") {
            Some(p) => (true, p.trim()),
            None => (false, rest),
        };
        let Some((head, last)) = path.rsplit_once('.') else {
            continue;
        };
        if last == "*" {
            out.push(b("", "*", head));
        } else if is_static {
            out.push(b(last, last, head));
        } else {
            out.push(b(last, last, path));
        }
    }
    out
}

fn c(src: &str) -> Vec<String> {
    src.lines()
        .filter_map(|l| {
            let r = l
                .trim()
                .strip_prefix('#')?
                .trim_start()
                .strip_prefix("include")?;
            let r = r.trim();
            let spec = r.get(1..r.len().checked_sub(1)?)?;
            matches!(r.as_bytes().first(), Some(b'"' | b'<')).then(|| b("", "*", spec))
        })
        .collect()
}

pub fn extract(lang: Lang, src: &str) -> Vec<String> {
    let mut v = match lang {
        Lang::Js => js(src),
        Lang::Py => py(src),
        Lang::Go => go(src),
        Lang::Java => java(src),
        Lang::C => c(src),
    };
    v.sort();
    v.dedup();
    v.truncate(300);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has(v: &[String], s: &str) -> bool {
        v.iter().any(|x| x == s)
    }

    #[test]
    fn js_imports_and_requires() {
        let v = extract(
            Lang::Js,
            "import Foo, { a, b as c } from './x';\nimport * as ns from \"../y\";\nimport {\n  d,\n  e\n} from './z';\nconst { f, g: h } = require('./w');\nconst k = require('./v');\nimport 'side';\n",
        );
        for want in [
            "b:Foo|default|./x",
            "b:a|a|./x",
            "b:c|b|./x",
            "b:ns|*|../y",
            "b:d|d|./z",
            "b:e|e|./z",
            "b:f|f|./w",
            "b:h|g|./w",
            "b:k|*|./v",
        ] {
            assert!(has(&v, want), "{want} in {v:?}");
        }
    }

    #[test]
    fn python_go_java_and_c_imports() {
        let py = extract(Lang::Py, "import os, pkg.mod as m\nfrom .util import helper as h, other\nfrom . import sibling\nfrom lib import (\n  a,\n  b,\n)\nfrom z import *\n");
        for want in [
            "b:m|*|pkg.mod",
            "b:h|helper|.util",
            "b:other|other|.util",
            "b:sibling|*|.sibling",
            "b:a|a|lib",
            "b:b|b|lib",
            "b:|*|z",
        ] {
            assert!(has(&py, want), "{want} in {py:?}");
        }
        let go = extract(Lang::Go, "package p\nimport \"fmt\"\nimport (\n  \"github.com/x/y/store\"\n  u \"github.com/x/util/v2\"\n  _ \"embed\"\n)\n");
        for want in [
            "b:fmt|*|fmt",
            "b:store|*|github.com/x/y/store",
            "b:u|*|github.com/x/util/v2",
        ] {
            assert!(has(&go, want), "{want} in {go:?}");
        }
        assert!(!go.iter().any(|x| x.contains("embed")));
        let java = extract(
            Lang::Java,
            "import a.b.Cls;\nimport static a.b.Util.helper;\nimport a.c.*;\n",
        );
        for want in ["b:Cls|Cls|a.b.Cls", "b:helper|helper|a.b.Util", "b:|*|a.c"] {
            assert!(has(&java, want), "{want} in {java:?}");
        }
        let c = extract(Lang::C, "#include \"x.h\"\n#  include <sys/y.h>\n");
        assert!(has(&c, "b:|*|x.h") && has(&c, "b:|*|sys/y.h"));
    }
}
