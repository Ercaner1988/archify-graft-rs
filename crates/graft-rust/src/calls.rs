//! What a function body does: which functions it calls, which data files it names,
//! whether it reads or writes. Works on masked text (no strings, no comments).

use std::collections::{BTreeSet, HashMap};

const KEYWORDS: &[&str] = &[
    "if", "while", "for", "match", "return", "fn", "loop", "let", "in", "as", "where", "impl",
    "move", "else", "unsafe", "async", "await", "dyn", "mut", "ref", "use", "pub", "box",
];
const READ_VERBS: &[&str] = &[
    "fs::read(",
    "fs::read_to_string(",
    "read_to_string(",
    "File::open(",
    "Mmap::map",
    "read_file(",
    "load_mmap(",
];
const WRITE_VERBS: &[&str] = &[
    "fs::write(",
    "File::create(",
    "create_new(",
    "OpenOptions",
    ".write_all(",
    "fs::copy(",
    "fs::rename(",
];
const MAX_CALLS: usize = 160;

#[derive(Debug, Default, Clone)]
pub struct BodyFacts {
    /// `name`, `.method`, `a::b::name` tokens of call sites (deduplicated, in order).
    pub calls: Vec<String>,
    /// Data files named by a literal or by a `const` that holds one.
    pub artifacts: Vec<String>,
    pub reads: bool,
    pub writes: bool,
}

fn is_id(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80
}

fn skip_ws(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && b[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

/// Skips a balanced `<...>` starting at `i` (which must be `<`).
fn skip_angle(b: &[u8], mut i: usize) -> usize {
    let mut depth = 0;
    while i < b.len() {
        match b[i] {
            b'<' => depth += 1,
            b'>' => {
                depth -= 1;
                if depth == 0 {
                    return i + 1;
                }
            }
            b';' | b'{' | b'}' => return i,
            _ => {}
        }
        i += 1;
    }
    i
}

/// Reads `a::b::<T>::c` from `i`; returns the segments and the index after them.
fn read_path(text: &str, mut i: usize) -> (Vec<&str>, usize) {
    let b = text.as_bytes();
    let mut segs = Vec::new();
    loop {
        let s = i;
        while i < b.len() && is_id(b[i]) {
            i += 1;
        }
        if i == s {
            break;
        }
        segs.push(&text[s..i]);
        let mut j = skip_ws(b, i);
        if !text[j..].starts_with("::") {
            break;
        }
        j = skip_ws(b, j + 2);
        if b.get(j) == Some(&b'<') {
            j = skip_ws(b, skip_angle(b, j));
            if !text[j..].starts_with("::") {
                i = j;
                break;
            }
            j = skip_ws(b, j + 2);
        }
        i = j;
    }
    (segs, i)
}

/// `artifact` file name of a string literal (`"x/envanter.bin"` -> `envanter.bin`).
pub fn artifact_name(lit: &str) -> Option<String> {
    const EXT: &[&str] = &[
        "bin", "db", "sqlite", "sqlite3", "toml", "json", "ron", "csv", "tsv", "log", "idx", "dat",
        "cache", "yaml", "yml", "svg", "html", "ndjson", "jsonl", "parquet",
    ];
    const SKIP: &[&str] = &["Cargo.toml", "package.json", "tsconfig.json"];
    let base = lit.rsplit(['/', '\\']).next()?.trim();
    let (stem, ext) = base.rsplit_once('.')?;
    let clean = base
        .bytes()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'));
    (clean
        && (2..=48).contains(&base.len())
        && !stem.is_empty()
        && EXT.contains(&ext)
        && !SKIP.contains(&base))
    .then(|| base.to_string())
}

/// `consts`: names of `const`/`static` items that hold a data file name.
pub fn body_facts(
    body: &str,
    literals: &[(usize, &str)],
    consts: &HashMap<String, String>,
) -> BodyFacts {
    let b = body.as_bytes();
    let mut facts = BodyFacts {
        reads: READ_VERBS.iter().any(|v| body.contains(v)),
        writes: WRITE_VERBS.iter().any(|v| body.contains(v)),
        ..BodyFacts::default()
    };
    let mut arts: BTreeSet<String> = literals
        .iter()
        .filter_map(|(_, t)| artifact_name(t))
        .collect();
    let mut seen = BTreeSet::new();
    let mut i = 0;
    while i < b.len() {
        if !(b[i].is_ascii_alphabetic() || b[i] == b'_' || b[i] >= 0x80)
            || (i > 0 && is_id(b[i - 1]))
        {
            i += 1;
            continue;
        }
        let method = {
            let mut k = i;
            while k > 0 && b[k - 1].is_ascii_whitespace() {
                k -= 1;
            }
            k > 0 && b[k - 1] == b'.' && !(k > 1 && b[k - 2] == b'.')
        };
        let (segs, end) = read_path(body, i);
        let after = skip_ws(b, end);
        let last = segs.last().copied().unwrap_or("");
        if let Some(art) = consts.get(last) {
            arts.insert(art.clone());
        }
        let is_call = b.get(after) == Some(&b'(');
        let lower_last = last.chars().next().is_some_and(|c| !c.is_uppercase());
        if is_call && lower_last && !KEYWORDS.contains(&last) && facts.calls.len() < MAX_CALLS {
            let token = if method {
                format!(".{last}")
            } else {
                segs.join("::")
            };
            if seen.insert(token.clone()) {
                facts.calls.push(token);
            }
        }
        i = if end > i { end.max(i + 1) } else { i + 1 };
        // Arguments still need scanning: continue right after the path.
    }
    facts.artifacts = arts.into_iter().collect();
    facts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(body: &str) -> BodyFacts {
        body_facts(body, &[], &HashMap::new())
    }

    #[test]
    fn plain_method_and_path_calls_are_tokens_but_constructors_are_not() {
        let f = facts("let x = oku(a); self.ekle(b); crate::goz::ac(c); Vec::<u8>::new(); Some(d); Komut::Ara(e); if (y) {} println!(\"{}\", z);");
        for t in ["oku", ".ekle", "crate::goz::ac", "Vec::new"] {
            assert!(f.calls.contains(&t.to_string()), "{t} in {:?}", f.calls);
        }
        for t in ["Some", "Komut::Ara", "if", "println"] {
            assert!(!f.calls.contains(&t.to_string()), "{t} in {:?}", f.calls);
        }
    }

    #[test]
    fn calls_inside_arguments_are_found_and_turkish_names_work() {
        let f = facts("dugum_ciz(ui, hesapla(x)); çağır();");
        assert!(f.calls.contains(&"hesapla".to_string()));
        assert!(f.calls.contains(&"çağır".to_string()));
    }

    #[test]
    fn io_verbs_and_artifact_names_are_detected() {
        let f = body_facts(
            "std::fs::write(&p, b)?; ",
            &[(3, "%LOCALAPPDATA%/x/envanter.bin"), (9, "hello world")],
            &HashMap::new(),
        );
        assert!(f.writes && !f.reads);
        assert_eq!(f.artifacts, ["envanter.bin"]);
    }

    #[test]
    fn a_const_that_holds_a_file_name_counts_when_used() {
        let consts = HashMap::from([("ENVANTER".to_string(), "envanter.bin".to_string())]);
        let f = body_facts("fs::read(dir.join(ENVANTER))", &[], &consts);
        assert!(f.reads);
        assert_eq!(f.artifacts, ["envanter.bin"]);
    }

    #[test]
    fn artifact_names_reject_sources_and_manifests() {
        assert_eq!(
            artifact_name("a/b/durumlar.json").as_deref(),
            Some("durumlar.json")
        );
        assert_eq!(artifact_name("Cargo.toml"), None);
        assert_eq!(artifact_name("main.rs"), None);
        assert_eq!(artifact_name("not a file.bin"), None);
    }
}
