//! Import extraction and resolution. `extract` pulls raw specs out of one file's full
//! text (Rust `a::b::c` paths, JS/TS relative imports, Python imports); `resolve` turns
//! them into file -> file `Imports` edges once the whole file set is known. std,
//! third-party packages and anything that is not a file of this graph are dropped.

use graft_cargo::{join_norm, Manifest};
use graft_model::{CodeGraph, EdgeRelation, EdgeV1, NodeKind};
use std::collections::{HashMap, HashSet};
use std::path::Path;

const MAX_SPECS: usize = 400;

/// Raw import specs of one file: `rs:crate::a::b`, `js:./x`, `py:.x`. `masked` is the
/// Rust source with strings and comments blanked (paths in docs are not imports).
pub fn extract(path: &str, content: &str, masked: Option<&str>) -> Vec<String> {
    let ext = Path::new(path).extension().and_then(|e| e.to_str());
    let specs: Vec<String> = match ext {
        Some("rs") => tagged("rs", rust_chains(masked.unwrap_or(content))),
        Some("ts" | "js" | "tsx" | "jsx" | "mjs" | "cjs" | "mts" | "cts") => {
            tagged("js", js_specs(content))
        }
        Some("py") => tagged("py", py_specs(content)),
        _ => Vec::new(),
    };
    let mut seen = HashSet::new();
    specs
        .into_iter()
        .filter(|s| seen.insert(s.clone()))
        .take(MAX_SPECS)
        .collect()
}

fn tagged(tag: &str, specs: Vec<String>) -> Vec<String> {
    specs.into_iter().map(|s| format!("{tag}:{s}")).collect()
}

fn is_id(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Paths inside a `{a, b::c, d::{e, f}}` group, each prefixed with `prefix`.
fn expand_group(prefix: &str, group: &str, out: &mut Vec<String>) {
    let (mut depth, mut start) = (0usize, 0usize);
    let mut items = Vec::new();
    for (i, c) in group.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                items.push(&group[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    items.push(&group[start..]);
    for item in items {
        let item = item.split(" as ").next().unwrap_or("").trim();
        if item.is_empty() || item == "*" {
            continue;
        }
        if let Some((head, rest)) = item.split_once("::{") {
            let head = head.trim();
            let inner = rest.trim_end_matches('}');
            let next = if head.is_empty() {
                prefix.to_string()
            } else {
                format!("{prefix}::{head}")
            };
            out.push(next.clone());
            expand_group(&next, inner, out);
        } else if item != "self" {
            out.push(format!(
                "{prefix}::{}",
                item.split_whitespace().collect::<String>()
            ));
        }
    }
}

/// Every maximal `ident::ident::...` chain (at least two segments, or one segment
/// followed by a `::{..}` group) in `src`. Covers `use` statements and inline paths such
/// as `crate::goz::open(..)` alike; a brace group gives the prefix and each member.
fn rust_chains(src: &str) -> Vec<String> {
    let b = src.as_bytes();
    let (mut out, mut i) = (Vec::new(), 0);
    // `use krate;` and `use krate as alias;` have no `::` but name a crate.
    for (at, _) in src.match_indices("use ") {
        if at > 0 && is_id(b[at - 1]) {
            continue;
        }
        let rest = &src[at + 4..];
        let end = rest.bytes().position(|c| !is_id(c)).unwrap_or(rest.len());
        let after = rest[end..].trim_start();
        if end > 0 && (after.starts_with(';') || after.starts_with("as ")) {
            out.push(rest[..end].to_string());
        }
    }
    while let Some(off) = src[i..].find("::") {
        let at = i + off;
        let mut start = at;
        loop {
            let mut k = start;
            while k > 0 && is_id(b[k - 1]) {
                k -= 1;
            }
            if k == start {
                break;
            }
            start = k;
            if start >= 2 && &b[start - 2..start] == b"::" {
                start -= 2;
            } else {
                break;
            }
        }
        if src[start..].starts_with("::") {
            start += 2;
        }
        let mut end = at;
        while src[end..].starts_with("::") {
            let id_start = end + 2;
            let mut k = id_start;
            while k < b.len() && is_id(b[k]) {
                k += 1;
            }
            if k == id_start {
                break;
            }
            end = k;
        }
        let mut next_i = end.max(at + 2);
        let grouped = src[end..].starts_with("::{");
        if end > start && (src[start..end].contains("::") || grouped) {
            let chain = src[start..end].to_string();
            if grouped {
                let open = end + 2;
                let mut depth = 0;
                let close = src[open..]
                    .char_indices()
                    .find_map(|(j, c)| {
                        match c {
                            '{' => depth += 1,
                            '}' => depth -= 1,
                            _ => {}
                        }
                        (depth == 0).then_some(open + j)
                    })
                    .unwrap_or(src.len());
                out.push(chain.clone());
                expand_group(&chain, &src[open + 1..close], &mut out);
                next_i = close.max(next_i);
            } else {
                out.push(chain);
            }
        }
        i = next_i;
    }
    out
}

/// Relative specs only (`./x`, `../y`); bare names are third-party packages.
fn js_specs(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    for marker in ["from ", "require(", "import(", "import "] {
        for (i, _) in src.match_indices(marker) {
            let rest = src[i + marker.len()..].trim_start();
            let Some(q) = rest
                .chars()
                .next()
                .filter(|c| matches!(c, '"' | '\'' | '`'))
            else {
                continue;
            };
            if let Some(end) = rest[1..].find(q) {
                let spec = &rest[1..1 + end];
                if spec.starts_with('.') {
                    out.push(spec.to_string());
                }
            }
        }
    }
    out
}

fn py_specs(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in src.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("from ") {
            if let Some((module, _)) = rest.split_once(" import ") {
                out.push(module.trim().to_string());
            }
        } else if let Some(rest) = t.strip_prefix("import ") {
            for part in rest.split(',') {
                if let Some(module) = part.split_whitespace().next() {
                    out.push(module.to_string());
                }
            }
        }
    }
    out
}

/// `(crate root, module path)` of a Rust file below a `src/` folder; `lib.rs` and
/// `main.rs` are the crate root (empty module path), `x/mod.rs` is module `x`.
pub(crate) fn rust_module(path: &str) -> Option<(String, String)> {
    let (root, rel) = match path.rfind("/src/") {
        Some(i) => (&path[..i], &path[i + 5..]),
        None => ("", path.strip_prefix("src/")?),
    };
    let rel = rel.strip_suffix(".rs")?;
    let rel = rel.strip_suffix("/mod").unwrap_or(rel);
    let rel = if matches!(rel, "lib" | "main") {
        ""
    } else {
        rel
    };
    Some((root.to_string(), rel.to_string()))
}

/// Rust modules of the graph: which file is `crate::a::b` of which crate.
pub(crate) struct RustIndex {
    by_key: HashMap<(String, String), String>,
    /// Crate name as written in source (`-` as `_`) -> crate roots with that name.
    crates: HashMap<String, Vec<String>>,
}

impl RustIndex {
    pub(crate) fn new(graph: &CodeGraph) -> Self {
        let mut idx = RustIndex {
            by_key: HashMap::new(),
            crates: HashMap::new(),
        };
        let mut files: Vec<(&str, String)> = graph
            .nodes
            .iter()
            .filter(|n| n.kind == NodeKind::File)
            .map(|n| (n.id.as_str(), n.path.replace('\\', "/")))
            .collect();
        // lib.rs wins over main.rs for the crate root.
        files.sort_by_key(|(_, p)| p.ends_with("/main.rs"));
        for (id, path) in &files {
            if let Some((root, rel)) = rust_module(path) {
                let folder = root.rsplit('/').next().unwrap_or("").replace('-', "_");
                idx.add_crate(folder, &root);
                idx.by_key
                    .entry((root, rel))
                    .or_insert_with(|| (*id).to_string());
            }
        }
        // Real package names from Cargo.toml beat the folder-name guess.
        for node in graph.nodes.iter().filter(|n| n.kind == NodeKind::Crate) {
            let Some(specs) = graph.imports.get(&format!("file:{}", node.path)) else {
                continue;
            };
            let path = node.path.replace('\\', "/");
            let dir = path.rsplit_once('/').map_or("", |(d, _)| d);
            if let Some(name) = Manifest::from_specs(specs).code_name() {
                idx.add_crate(name, dir);
            }
        }
        idx
    }

    fn add_crate(&mut self, name: String, root: &str) {
        let roots = self.crates.entry(name).or_default();
        if !roots.iter().any(|r| r == root) {
            roots.push(root.to_string());
        }
    }

    /// Workspace crate by name. Package names often carry a prefix the folder lacks
    /// (`pasli_cekirdek` lives in `cekirdek/`), so a unique `_<folder>` suffix counts.
    fn crate_root(&self, name: &str) -> Option<&str> {
        if let Some(roots) = self.crates.get(name) {
            return (roots.len() == 1).then(|| roots[0].as_str());
        }
        let mut hits =
            (self.crates.iter()).filter(|(k, _)| !k.is_empty() && name.ends_with(&format!("_{k}")));
        match (hits.next(), hits.next()) {
            (Some((_, roots)), None) if roots.len() == 1 => Some(roots[0].as_str()),
            _ => None,
        }
    }

    /// Longest module prefix of `segs` that is a file of the crate at `root`; with no
    /// such prefix, the crate root itself (`lib.rs`/`main.rs`).
    fn find(&self, root: &str, segs: &[&str]) -> Option<String> {
        (0..=segs.len()).rev().find_map(|n| {
            let key = (root.to_string(), segs[..n].join("/"));
            self.by_key.get(&key).cloned()
        })
    }

    /// File of `chain` (`crate::a::b::item`, `self::x`, `super::y`, `krate::z`) as seen
    /// from `from_path`: the deepest module file it names.
    pub(crate) fn resolve(&self, from_path: &str, chain: &str) -> Option<String> {
        let segs: Vec<&str> = chain.split("::").collect();
        self.resolve_segs(from_path, &segs)
    }

    pub(crate) fn resolve_segs(&self, from_path: &str, segs: &[&str]) -> Option<String> {
        let (root, own) = rust_module(from_path)?;
        let own: Vec<&str> = own.split('/').filter(|s| !s.is_empty()).collect();
        let (root, rest): (String, Vec<&str>) = match *segs.first()? {
            "crate" => (root, segs[1..].to_vec()),
            "self" => (root, [own.as_slice(), &segs[1..]].concat()),
            "super" => {
                let ups = segs.iter().take_while(|s| **s == "super").count();
                let keep = own.len().saturating_sub(ups);
                (root, [&own[..keep], &segs[ups..]].concat())
            }
            name => (self.crate_root(name)?.to_string(), segs[1..].to_vec()),
        };
        self.find(&root, &rest)
    }
}

fn dir_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(d, _)| d)
}

fn lookup(paths: &HashMap<&str, &str>, candidates: &[String]) -> Option<String> {
    candidates
        .iter()
        .find_map(|c| paths.get(c.as_str()).map(|id| id.to_string()))
}

fn resolve_js(from: &str, spec: &str, paths: &HashMap<&str, &str>) -> Option<String> {
    let base = join_norm(dir_of(from), spec);
    let base = if from.starts_with('/') && !base.starts_with('/') {
        format!("/{base}")
    } else {
        base
    };
    let candidates: Vec<String> = [
        "",
        ".ts",
        ".tsx",
        ".js",
        ".jsx",
        ".mjs",
        ".cjs",
        ".mts",
        ".cts",
        "/index.ts",
        "/index.tsx",
        "/index.js",
        "/index.mjs",
        "/index.cjs",
    ]
    .iter()
    .map(|ext| format!("{base}{ext}"))
    .collect();
    lookup(paths, &candidates)
}

fn resolve_py(from: &str, spec: &str, paths: &HashMap<&str, &str>) -> Option<String> {
    let dots = spec.chars().take_while(|c| *c == '.').count();
    let module = spec[dots..].replace('.', "/");
    if dots > 0 {
        let mut dir = dir_of(from).to_string();
        for _ in 1..dots {
            dir = dir_of(&dir).to_string();
        }
        let candidates = if module.is_empty() {
            vec![format!("{dir}/__init__.py")]
        } else {
            vec![
                format!("{dir}/{module}.py"),
                format!("{dir}/{module}/__init__.py"),
            ]
        };
        return lookup(paths, &candidates);
    }
    let suffixes = [format!("/{module}.py"), format!("/{module}/__init__.py")];
    let mut hits = paths
        .iter()
        .filter(|(p, _)| suffixes.iter().any(|s| p.ends_with(s.as_str())));
    match (hits.next(), hits.next()) {
        (Some((_, id)), None) => Some(id.to_string()),
        _ => None,
    }
}

/// Rebuilds every `Imports` edge from the raw specs in `graph.imports`. Idempotent:
/// running it again on the same graph yields the same edges.
pub fn resolve(graph: &mut CodeGraph) {
    graph.edges.retain(|e| e.relation != EdgeRelation::Imports);
    let files: Vec<(String, String)> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::File)
        .map(|n| (n.id.clone(), n.path.replace('\\', "/")))
        .collect();
    let paths: HashMap<&str, &str> = files
        .iter()
        .map(|(id, p)| (p.as_str(), id.as_str()))
        .collect();
    let path_of: HashMap<&str, &str> = files
        .iter()
        .map(|(id, p)| (id.as_str(), p.as_str()))
        .collect();
    let rust = RustIndex::new(graph);

    let mut seen = HashSet::new();
    let mut edges = Vec::new();
    for (file_id, specs) in &graph.imports {
        let Some(&from) = path_of.get(file_id.as_str()) else {
            continue;
        };
        for spec in specs {
            let target = match spec.split_once(':') {
                Some(("rs", s)) => rust.resolve(from, s),
                Some(("js", s)) => resolve_js(from, s, &paths),
                Some(("py", s)) => resolve_py(from, s, &paths),
                _ => None,
            };
            if let Some(target) = target {
                if &target != file_id && seen.insert((file_id.clone(), target.clone())) {
                    edges.push(EdgeV1 {
                        source: file_id.clone(),
                        target,
                        relation: EdgeRelation::Imports,
                        confidence: 1.0,
                    });
                }
            }
        }
    }
    graph.edges.extend(edges);
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_model::NodeV1;

    fn file(path: &str) -> NodeV1 {
        NodeV1 {
            id: format!("file:{path}"),
            path: path.into(),
            name: path.rsplit('/').next().unwrap().into(),
            kind: NodeKind::File,
            span: None,
            search_body: String::new(),
            file_residual: String::new(),
        }
    }

    fn graph(files: &[&str], imports: &[(&str, &[&str])]) -> CodeGraph {
        let mut g = CodeGraph::new();
        for f in files {
            g.add_node(file(f));
        }
        for (from, specs) in imports {
            g.imports.insert(
                format!("file:{from}"),
                specs.iter().map(|s| s.to_string()).collect(),
            );
        }
        g
    }

    fn links(g: &CodeGraph) -> Vec<(String, String)> {
        let short = |s: &str| {
            s.trim_start_matches("file:")
                .rsplit('/')
                .take(2)
                .collect::<Vec<_>>()
                .join("<")
        };
        let mut v: Vec<_> = g
            .edges
            .iter()
            .map(|e| (short(&e.source), short(&e.target)))
            .collect();
        v.sort();
        v
    }

    #[test]
    fn rust_paths_are_found_in_use_statements_and_inline() {
        let src = "use crate::goz::{a, b};\nuse super::*;\nfn f() { crate::kopru::x(); std::fs::read(p); Vec::<u8>::new(); }";
        let chains = rust_chains(src);
        assert!(chains.contains(&"crate::goz".to_string()));
        assert!(chains.contains(&"crate::goz::a".to_string()));
        assert!(chains.contains(&"crate::kopru::x".to_string()));
        assert!(chains.contains(&"std::fs::read".to_string()));
        assert!(!chains.iter().any(|c| c == "super" || c.is_empty()));
    }

    #[test]
    fn brace_groups_give_the_crate_root_and_every_member() {
        let src0 = "use kok;
use kok2 as k;";
        assert_eq!(rust_chains(src0), ["kok", "kok2"]);
        let src = "use fihrist_core::{Kayit, hata::{Hata, Sonuc as S}, self};\nuse graft_model::{CodeGraph};";
        let chains = rust_chains(src);
        for c in [
            "fihrist_core",
            "fihrist_core::Kayit",
            "fihrist_core::hata",
            "fihrist_core::hata::Hata",
            "fihrist_core::hata::Sonuc",
            "graft_model",
            "graft_model::CodeGraph",
        ] {
            assert!(chains.contains(&c.to_string()), "{c} in {chains:?}");
        }
    }

    #[test]
    fn rust_imports_resolve_across_modules_and_workspace_crates() {
        let files = [
            "/w/repo/cekirdek/src/lib.rs",
            "/w/repo/cekirdek/src/goz.rs",
            "/w/repo/cekirdek/src/uret/mod.rs",
            "/w/repo/cekirdek/src/uret/olcum.rs",
            "/w/repo/cli/src/main.rs",
        ];
        let mut g = graph(
            &files,
            &[
                (
                    "/w/repo/cekirdek/src/uret/olcum.rs",
                    &[
                        "rs:crate::goz::agac_tara_goz",
                        "rs:super::hesapla",
                        "rs:std::fs::read",
                    ],
                ),
                (
                    "/w/repo/cli/src/main.rs",
                    &["rs:pasli_cekirdek::uret::olcum::f"],
                ),
            ],
        );
        resolve(&mut g);
        assert_eq!(
            links(&g),
            [
                ("main.rs<src".to_string(), "olcum.rs<uret".to_string()),
                ("olcum.rs<uret".to_string(), "goz.rs<src".to_string()),
                ("olcum.rs<uret".to_string(), "mod.rs<uret".to_string()),
            ]
        );
    }

    #[test]
    fn a_path_naming_only_the_crate_root_links_to_lib_rs() {
        let files = [
            "/w/r/core/src/lib.rs",
            "/w/r/core/src/a.rs",
            "/w/r/app/src/main.rs",
        ];
        let mut g = graph(
            &files,
            &[("/w/r/app/src/main.rs", &["rs:core::Thing", "rs:core"])],
        );
        resolve(&mut g);
        assert_eq!(
            links(&g),
            [("main.rs<src".to_string(), "lib.rs<src".to_string())]
        );
    }

    #[test]
    fn package_names_from_cargo_beat_the_folder_guess() {
        let mut g = graph(
            &["/w/r/ic/src/lib.rs", "/w/r/app/src/main.rs"],
            &[("/w/r/app/src/main.rs", &["rs:dis_ad::X"])],
        );
        let mut manifest = file("/w/r/ic/Cargo.toml");
        manifest.kind = NodeKind::Crate;
        manifest.name = "dis-ad".into();
        g.add_node(manifest);
        g.imports.insert(
            "file:/w/r/ic/Cargo.toml".into(),
            vec!["cargo:pkg|dis-ad".into()],
        );
        resolve(&mut g);
        assert_eq!(links(&g).len(), 1);
    }

    #[test]
    fn js_and_python_relative_imports_resolve() {
        let files = [
            "/w/app/src/a.ts",
            "/w/app/src/lib/b.ts",
            "/w/app/src/lib/index.ts",
            "/w/py/pkg/a.py",
            "/w/py/pkg/b.py",
            "/w/py/pkg/sub/__init__.py",
        ];
        let mut g = graph(
            &files,
            &[
                (
                    "/w/app/src/a.ts",
                    &["js:./lib/b", "js:./lib", "js:../missing"],
                ),
                ("/w/py/pkg/a.py", &["py:.b", "py:.sub", "py:pkg.b", "py:os"]),
            ],
        );
        resolve(&mut g);
        assert_eq!(g.edges.len(), 4, "{:?}", links(&g));
        assert!(g.edges.iter().all(|e| e.relation == EdgeRelation::Imports));
    }

    #[test]
    fn resolve_is_idempotent_and_forgets_removed_files() {
        let files = ["/w/r/c/src/lib.rs", "/w/r/c/src/a.rs", "/w/r/c/src/b.rs"];
        let mut g = graph(&files, &[("/w/r/c/src/a.rs", &["rs:crate::b::x"])]);
        resolve(&mut g);
        resolve(&mut g);
        assert_eq!(g.edges.len(), 1);
        g.imports.clear();
        resolve(&mut g);
        assert!(g.edges.is_empty());
    }
}
