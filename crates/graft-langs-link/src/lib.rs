//! Graph-wide linking for the non-Rust languages: `Imports` edges from the bindings of
//! each file and `Calls` edges from the `c:` call tokens. A call is linked only when the
//! language's own scoping rules pin it: same file, a binding the file imports, the class
//! it names, its package / directory, or (JS, Python) a long unique method name.

use graft_langs::Lang;
use graft_langs::{is_std_method, is_std_receiver};
use graft_model::{CodeGraph, EdgeRelation, EdgeV1, NodeKind};
use std::collections::{HashMap, HashSet};

struct Binding {
    local: String,
    orig: String,
    spec: String,
}

struct Def<'a> {
    id: &'a str,
    file: String,
    name: &'a str,
    owner: Option<&'a str>,
    kind: NodeKind,
    is_pub: bool,
}

struct Files {
    /// normalized path -> file node id
    id_of: HashMap<String, String>,
    by_dir: HashMap<String, Vec<String>>,
    by_name: HashMap<String, Vec<String>>,
    /// dotted module tails (`pkg.mod`) -> paths, for Python and Java
    dotted: HashMap<String, Vec<String>>,
}

fn dir_of(p: &str) -> &str {
    p.rsplit_once('/').map_or("", |(d, _)| d)
}

fn norm_join(dir: &str, spec: &str) -> String {
    let mut parts: Vec<&str> = dir.split('/').filter(|s| !s.is_empty()).collect();
    for seg in spec.split('/') {
        match seg {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    let j = parts.join("/");
    if dir.starts_with('/') {
        format!("/{j}")
    } else {
        j
    }
}

impl Files {
    fn new(graph: &CodeGraph) -> Files {
        let mut f = Files {
            id_of: HashMap::new(),
            by_dir: HashMap::new(),
            by_name: HashMap::new(),
            dotted: HashMap::new(),
        };
        for n in graph.nodes.iter().filter(|n| n.kind == NodeKind::File) {
            let p = n.path.replace('\\', "/");
            if Lang::of_path(&p).is_none() {
                continue;
            }
            f.by_dir
                .entry(dir_of(&p).to_string())
                .or_default()
                .push(p.clone());
            f.by_name
                .entry(p.rsplit('/').next().unwrap_or("").to_string())
                .or_default()
                .push(p.clone());
            let stem = p.rsplit_once('.').map_or(p.as_str(), |s| s.0);
            let stem = stem.strip_suffix("/__init__").unwrap_or(stem);
            let parts: Vec<&str> = stem.split('/').collect();
            for k in 1..=parts.len().min(5) {
                f.dotted
                    .entry(parts[parts.len() - k..].join("."))
                    .or_default()
                    .push(p.clone());
            }
            f.id_of.insert(p, n.id.clone());
        }
        f
    }

    /// Files a binding spec names (empty = outside the project).
    fn resolve(&self, lang: Lang, from: &str, spec: &str) -> Vec<String> {
        let exists = |p: &str| self.id_of.contains_key(p).then(|| p.to_string());
        match lang {
            Lang::Js => {
                if !spec.starts_with('.') {
                    return Vec::new();
                }
                let base = norm_join(dir_of(from), spec);
                let trimmed = ["js", "jsx", "mjs", "cjs"]
                    .iter()
                    .find_map(|e| base.strip_suffix(&format!(".{e}")))
                    .unwrap_or(&base);
                for b in [base.as_str(), trimmed] {
                    for ext in [
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
                    ] {
                        if let Some(p) = exists(&format!("{b}{ext}")) {
                            return vec![p];
                        }
                    }
                }
                Vec::new()
            }
            Lang::Py => {
                let dots = spec.chars().take_while(|c| *c == '.').count();
                let module = spec[dots..].replace('.', "/");
                if dots > 0 {
                    let mut dir = dir_of(from).to_string();
                    for _ in 1..dots {
                        dir = dir_of(&dir).to_string();
                    }
                    let cands = if module.is_empty() {
                        vec![format!("{dir}/__init__.py")]
                    } else {
                        vec![
                            format!("{dir}/{module}.py"),
                            format!("{dir}/{module}/__init__.py"),
                        ]
                    };
                    return cands.iter().find_map(|c| exists(c)).into_iter().collect();
                }
                match self.dotted.get(spec) {
                    Some(v) if v.iter().all(|p| p.ends_with(".py")) && v.len() <= 2 => v.clone(),
                    _ => Vec::new(),
                }
            }
            Lang::Go => {
                let parts: Vec<&str> = spec.split('/').collect();
                for k in (1..=parts.len().min(3)).rev() {
                    let tail = format!("/{}", parts[parts.len() - k..].join("/"));
                    let hits: Vec<&String> =
                        self.by_dir.keys().filter(|d| d.ends_with(&tail)).collect();
                    if hits.len() == 1 && (k > 1 || parts.len() == 1 || !parts[0].contains('.')) {
                        return self.by_dir[hits[0]].clone();
                    }
                    if !hits.is_empty() {
                        break;
                    }
                }
                Vec::new()
            }
            Lang::Java => {
                let path = spec.replace('.', "/");
                let last = spec.rsplit('.').next().unwrap_or(spec);
                let file = self
                    .by_name
                    .get(&format!("{last}.java"))
                    .map(|v| {
                        v.iter()
                            .filter(|p| p.ends_with(&format!("{path}.java")))
                            .cloned()
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if !file.is_empty() {
                    return file;
                }
                self.by_dir
                    .iter()
                    .filter(|(d, _)| d.ends_with(&format!("/{path}")))
                    .flat_map(|(_, v)| v.clone())
                    .collect()
            }
            Lang::C => {
                let name = spec.rsplit('/').next().unwrap_or(spec);
                let near = norm_join(dir_of(from), spec);
                let mut hits: Vec<String> = exists(&near).into_iter().collect();
                if hits.is_empty() {
                    hits = self
                        .by_name
                        .get(name)
                        .map(|v| {
                            v.iter()
                                .filter(|p| p.ends_with(&format!("/{spec}")))
                                .cloned()
                                .collect()
                        })
                        .unwrap_or_default();
                }
                // A header stands for the sources next to it.
                let mut all = hits.clone();
                for h in &hits {
                    if let Some((stem, ext)) = h.rsplit_once('.') {
                        if matches!(ext, "h" | "hpp" | "hh") {
                            for e in ["c", "cpp", "cc", "cxx"] {
                                all.extend(exists(&format!("{stem}.{e}")));
                            }
                        }
                    }
                }
                all
            }
        }
    }
}

fn unique<'a, 'b>(mut it: impl Iterator<Item = &'b Def<'a>>) -> Option<&'b Def<'a>>
where
    'a: 'b,
{
    match (it.next(), it.next()) {
        (Some(d), None) => Some(d),
        _ => None,
    }
}

fn parse_bindings(facts: Option<&Vec<String>>) -> Vec<Binding> {
    facts
        .into_iter()
        .flatten()
        .filter_map(|f| {
            let mut p = f.strip_prefix("b:")?.splitn(3, '|');
            Some(Binding {
                local: p.next()?.to_string(),
                orig: p.next()?.to_string(),
                spec: p.next()?.to_string(),
            })
        })
        .collect()
}

pub fn link(graph: &mut CodeGraph) {
    let files = Files::new(graph);
    if files.id_of.is_empty() {
        return;
    }
    let mut defs: Vec<Def> = Vec::new();
    for n in graph.nodes.iter().filter(|n| {
        matches!(
            n.kind,
            NodeKind::Function
                | NodeKind::Method
                | NodeKind::Class
                | NodeKind::Interface
                | NodeKind::Enum
        )
    }) {
        let p = n.path.replace('\\', "/");
        if !files.id_of.contains_key(&p) {
            continue;
        }
        let rest =
            n.id.strip_prefix(n.path.as_str())
                .and_then(|r| r.strip_prefix(':'))
                .unwrap_or(&n.id);
        let rest = rest.split('#').next().unwrap_or(rest);
        defs.push(Def {
            id: &n.id,
            name: &n.name,
            owner: rest.rsplit_once("::").map(|(o, _)| o),
            kind: n.kind.clone(),
            is_pub: graph
                .facts
                .get(&n.id)
                .is_some_and(|f| f.iter().any(|x| x == "pub")),
            file: p,
        });
    }
    let mut by_name: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, d) in defs.iter().enumerate() {
        by_name.entry(d.name).or_default().push(i);
    }
    let cands = |name: &str| -> Vec<&Def> {
        by_name
            .get(name)
            .into_iter()
            .flatten()
            .map(|&i| &defs[i])
            .collect()
    };

    let mut fns_of: HashMap<&str, Vec<&graft_model::NodeV1>> = HashMap::new();
    for n in graph
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Function | NodeKind::Method))
    {
        fns_of.entry(n.path.as_str()).or_default().push(n);
    }
    let mut existing: HashSet<(String, String)> = graph
        .edges
        .iter()
        .filter(|e| e.relation == EdgeRelation::Imports)
        .map(|e| (e.source.clone(), e.target.clone()))
        .collect();
    let mut new_edges: Vec<EdgeV1> = Vec::new();
    let mut calls_seen: HashSet<(String, String)> = HashSet::new();

    for f in graph.nodes.iter().filter(|n| n.kind == NodeKind::File) {
        let from = f.path.replace('\\', "/");
        let Some(lang) = Lang::of_path(&from) else {
            continue;
        };
        let bindings = parse_bindings(graph.facts.get(&f.id));
        let resolved: Vec<Vec<String>> = bindings
            .iter()
            .map(|b| files.resolve(lang, &from, &b.spec))
            .collect();
        for (b, rf) in bindings.iter().zip(&resolved) {
            let _ = b;
            for t in rf.iter().filter(|t| **t != from) {
                let tid = files.id_of[t].clone();
                if existing.insert((f.id.clone(), tid.clone())) {
                    new_edges.push(EdgeV1 {
                        source: f.id.clone(),
                        target: tid,
                        relation: EdgeRelation::Imports,
                        confidence: 1.0,
                    });
                }
            }
        }
        // Same package / directory scope (Go, Java, C), plus everything imported.
        let package: Vec<String> = if matches!(lang, Lang::Go | Lang::Java | Lang::C) {
            files.by_dir.get(dir_of(&from)).cloned().unwrap_or_default()
        } else {
            Vec::new()
        };
        let mut scope: HashSet<&str> = package.iter().map(String::as_str).collect();
        for rf in &resolved {
            scope.extend(rf.iter().map(String::as_str));
        }
        scope.insert(from.as_str());
        let internal: HashSet<&str> = bindings
            .iter()
            .zip(&resolved)
            .filter(|(_, rf)| !rf.is_empty())
            .map(|(b, _)| b.local.as_str())
            .collect();
        let external: HashSet<&str> = bindings
            .iter()
            .zip(&resolved)
            .filter(|(b, rf)| {
                rf.is_empty() && !b.local.is_empty() && !internal.contains(b.local.as_str())
            })
            .map(|(b, _)| b.local.as_str())
            .collect();
        let types: HashSet<&str> = graph
            .facts
            .get(&f.id)
            .into_iter()
            .flatten()
            .filter_map(|x| x.strip_prefix("ty:"))
            .collect();
        let in_scope = |d: &Def| scope.contains(d.file.as_str());
        let same_dir = |d: &Def| dir_of(&d.file) == dir_of(&from);
        let bound_files = |local: &str| -> HashSet<&str> {
            bindings
                .iter()
                .zip(&resolved)
                .filter(|(b, _)| b.local == local)
                .flat_map(|(_, rf)| rf.iter().map(String::as_str))
                .collect()
        };
        let bound_defs = |local: &str, name: &str| -> Vec<&Def> {
            let mut out = Vec::new();
            for (b, rf) in bindings
                .iter()
                .zip(&resolved)
                .filter(|(b, _)| b.local == local)
            {
                let target = if b.orig == "default" || b.orig == "*" {
                    name
                } else {
                    b.orig.as_str()
                };
                out.extend(cands(target).into_iter().filter(|d| rf.contains(&d.file)));
            }
            out
        };

        for n in fns_of.get(f.path.as_str()).into_iter().flatten().copied() {
            let Some(facts) = graph.facts.get(&n.id) else {
                continue;
            };
            let my_owner =
                n.id.strip_prefix(n.path.as_str())
                    .and_then(|r| r.strip_prefix(':'))
                    .and_then(|r| r.split('#').next())
                    .and_then(|r| r.rsplit_once("::"))
                    .map(|(o, _)| o);
            // Typed languages: a receiver's class must at least be named in this file.
            let typed = |name: &str| -> Vec<&Def> {
                cands(name)
                    .into_iter()
                    .filter(|d| {
                        !matches!(lang, Lang::Java | Lang::C | Lang::Go)
                            || d.owner
                                .is_some_and(|o| types.contains(o) || Some(o) == my_owner)
                    })
                    .collect()
            };
            for token in facts.iter().filter_map(|x| x.strip_prefix("c:")) {
                let target: Option<(&Def, f32)> = if let Some(cls) = token.strip_prefix("new:") {
                    let class = |d: &&Def| {
                        matches!(
                            d.kind,
                            NodeKind::Class | NodeKind::Interface | NodeKind::Enum
                        )
                    };
                    let found = unique(
                        cands(cls)
                            .into_iter()
                            .filter(|d| class(d) && d.file == from),
                    )
                    .or_else(|| unique(bound_defs(cls, cls).into_iter().filter(|d| class(d))))
                    .or_else(|| unique(cands(cls).into_iter().filter(|d| class(d) && in_scope(d))))
                    .or_else(|| unique(cands(cls).into_iter().filter(|d| class(d) && d.is_pub)));
                    found.map(|c| {
                        let ctor = cands("constructor")
                            .into_iter()
                            .chain(cands("__init__"))
                            .chain(cands(cls))
                            .find(|d| {
                                d.owner == Some(cls)
                                    && d.file == c.file
                                    && matches!(d.kind, NodeKind::Method)
                            });
                        (ctor.unwrap_or(c), 0.85)
                    })
                } else if let Some((recv, name)) =
                    token.split_once('.').filter(|(r, _)| !r.is_empty())
                {
                    let bound = bound_files(recv);
                    if !bound.is_empty() {
                        unique(cands(name).into_iter().filter(|d| {
                            bound.contains(d.file.as_str()) && !matches!(d.kind, NodeKind::Class)
                        }))
                        .map(|d| (d, 0.9))
                    } else if external.contains(recv) || is_std_receiver(recv) {
                        None
                    } else if recv.chars().next().is_some_and(char::is_uppercase) {
                        unique(
                            cands(name)
                                .into_iter()
                                .filter(|d| d.owner == Some(recv) && (in_scope(d) || d.is_pub)),
                        )
                        .map(|d| (d, 0.85))
                        .or_else(|| unknown(name, None, &from, lang, &typed, &in_scope, &same_dir))
                    } else {
                        unknown(name, None, &from, lang, &typed, &in_scope, &same_dir)
                    }
                } else if let Some(name) = token.strip_prefix('~') {
                    unknown(name, None, &from, lang, &typed, &in_scope, &same_dir)
                } else if let Some(name) = token.strip_prefix('.') {
                    unknown(name, my_owner, &from, lang, &typed, &in_scope, &same_dir)
                } else if external.contains(token) {
                    // Imported from outside the project somewhere in this file (often
                    // inside a function, shadowing a same-named local definition).
                    None
                } else {
                    let free = || {
                        cands(token)
                            .into_iter()
                            .filter(|d| d.owner.is_none() && !matches!(d.kind, NodeKind::Class))
                    };
                    unique(free().filter(|d| d.file == from))
                        .or_else(|| {
                            unique(cands(token).into_iter().filter(|d| {
                                d.file == from && my_owner.is_some() && d.owner == my_owner
                            }))
                        })
                        .map(|d| (d, 0.95))
                        .or_else(|| {
                            unique(
                                bound_defs(token, token)
                                    .into_iter()
                                    .filter(|d| !matches!(d.kind, NodeKind::Class)),
                            )
                            .map(|d| (d, 0.9))
                        })
                        .or_else(|| {
                            let glob: HashSet<&str> = bindings
                                .iter()
                                .zip(&resolved)
                                .filter(|(b, _)| b.local.is_empty())
                                .flat_map(|(_, rf)| rf.iter().map(String::as_str))
                                .collect();
                            unique(free().filter(|d| glob.contains(d.file.as_str())))
                                .map(|d| (d, 0.85))
                        })
                        .or_else(|| {
                            (matches!(lang, Lang::Go | Lang::Java | Lang::C))
                                .then(|| {
                                    unique(free().filter(|d| in_scope(d) || same_dir(d)))
                                        .map(|d| (d, 0.75))
                                })
                                .flatten()
                        })
                };
                if let Some((d, confidence)) = target {
                    if d.id != n.id && calls_seen.insert((n.id.clone(), d.id.to_string())) {
                        new_edges.push(EdgeV1 {
                            source: n.id.clone(),
                            target: d.id.to_string(),
                            relation: EdgeRelation::Calls,
                            confidence,
                        });
                    }
                }
            }
        }
    }
    graph.edges.extend(new_edges);
}

/// Method call on a receiver whose type is unknown.
fn unknown<'a, 'b>(
    name: &str,
    my_owner: Option<&str>,
    from: &str,
    lang: Lang,
    cands: &dyn Fn(&str) -> Vec<&'b Def<'a>>,
    in_scope: &dyn Fn(&Def) -> bool,
    same_dir: &dyn Fn(&Def) -> bool,
) -> Option<(&'b Def<'a>, f32)> {
    // `super().__init__()` and friends go to a parent we cannot see.
    if name.starts_with("__") && name.ends_with("__") {
        return None;
    }
    let methods = || {
        cands(name)
            .into_iter()
            .filter(|d| d.owner.is_some() && matches!(d.kind, NodeKind::Method))
    };
    if my_owner.is_some() {
        if let Some(d) = unique(methods().filter(|d| d.file == from && d.owner == my_owner)) {
            return Some((d, 0.85));
        }
    }
    if is_std_method(name) || name.len() < 4 {
        return None;
    }
    unique(methods().filter(|d| d.file == from))
        .map(|d| (d, 0.7))
        .or_else(|| unique(methods().filter(|d| in_scope(d))).map(|d| (d, 0.6)))
        .or_else(|| match lang {
            Lang::Go | Lang::Java | Lang::C => {
                unique(methods().filter(|d| same_dir(d))).map(|d| (d, 0.55))
            }
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_langs::extract;

    fn graph(files: &[(&str, &str)]) -> CodeGraph {
        let mut g = CodeGraph::new();
        for (path, src) in files {
            let name = path.rsplit('/').next().unwrap();
            let mut sub = CodeGraph::new();
            sub.add_node(graft_model::NodeV1 {
                id: format!("file:{path}"),
                path: (*path).into(),
                name: name.into(),
                kind: NodeKind::File,
                span: None,
                search_body: String::new(),
                file_residual: String::new(),
            });
            extract(&mut sub, path, &format!("file:{path}"), src);
            g.absorb(sub);
        }
        link(&mut g);
        g.rebuild_index();
        g
    }

    fn calls(g: &CodeGraph) -> Vec<String> {
        let short = |s: &str| s.rsplit('/').next().unwrap().to_string();
        let mut v: Vec<String> = g
            .edges
            .iter()
            .filter(|e| e.relation == EdgeRelation::Calls)
            .map(|e| format!("{}>{}", short(&e.source), short(&e.target)))
            .collect();
        v.sort();
        v
    }

    #[test]
    fn js_calls_follow_imports_namespaces_classes_and_skip_builtins() {
        let g = graph(&[
            ("/p/a.js", "import { render } from './b';\nimport * as u from './c';\nimport Box from './d';\nfunction main() { render(1); u.merge(2); new Box(3); console.log(1); [1].map(f); process(4); }\nfunction process(x) {}\n"),
            ("/p/b.js", "export function render(x) { return 1; }\n"),
            ("/p/c.js", "export function merge(x) {}\nexport function render(x) {}\n"),
            ("/p/d.js", "export default class Box { constructor(a) { this.a = a; } }\n"),
            ("/p/e.js", "function render(x) {}\nfunction other() { render(1); }\n"),
        ]);
        assert_eq!(
            calls(&g),
            [
                "a.js:main>a.js:process",
                "a.js:main>b.js:render",
                "a.js:main>c.js:merge",
                "a.js:main>d.js:Box::constructor",
                "e.js:other>e.js:render"
            ]
        );
    }

    #[test]
    fn python_from_imports_modules_self_methods_and_builtins() {
        let g = graph(&[
            ("/p/app/main.py", "from .util import helper\nimport app.store as store\n\nclass Svc:\n    def run(self):\n        helper(1)\n        self.step()\n        store.save(2)\n        print(len([]))\n        data.append(3)\n    def step(self):\n        pass\n"),
            ("/p/app/util.py", "def helper(x):\n    return x\n"),
            ("/p/app/store.py", "def save(x):\n    pass\n"),
        ]);
        assert_eq!(
            calls(&g),
            [
                "main.py:Svc::run>main.py:Svc::step",
                "main.py:Svc::run>store.py:save",
                "main.py:Svc::run>util.py:helper"
            ]
        );
    }

    #[test]
    fn go_packages_java_imports_and_c_includes() {
        let go = graph(&[
            ("/m/cmd/main.go", "package main\nimport (\n \"fmt\"\n \"example.com/m/store\"\n)\nfunc main() {\n fmt.Println(1)\n store.Open(2)\n local()\n}\nfunc local() {}\n"),
            ("/m/store/db.go", "package store\nfunc Open(p int) {}\nfunc Println(a int) {}\n"),
        ]);
        assert_eq!(
            calls(&go),
            ["main.go:main>db.go:Open", "main.go:main>main.go:local"]
        );

        let java = graph(&[
            ("/j/a/Svc.java", "package a;\nimport b.Util;\npublic class Svc {\n  void run() { Util.help(); System.out.println(1); step(); }\n  void step() {}\n}\n"),
            ("/j/b/Util.java", "package b;\npublic class Util {\n  public static void help() {}\n}\n"),
        ]);
        assert_eq!(
            calls(&java),
            [
                "Svc.java:Svc::run>Svc.java:Svc::step",
                "Svc.java:Svc::run>Util.java:Util::help"
            ]
        );

        let c = graph(&[
            (
                "/c/main.c",
                "#include \"lib.h\"\nint main(void) { compute(1); printf(\"x\"); return 0; }\n",
            ),
            ("/c/lib.h", "int compute(int a);\n"),
            (
                "/c/lib.c",
                "#include \"lib.h\"\nint compute(int a) { return a; }\n",
            ),
        ]);
        assert_eq!(calls(&c), ["main.c:main>lib.c:compute"]);
    }

    #[test]
    fn ambiguous_names_and_unknown_externals_make_no_edge() {
        let g = graph(&[
            (
                "/p/a.js",
                "import x from 'react';\nfunction main() { x.render(); run(); }\n",
            ),
            ("/p/b.js", "function run() {}\n"),
            ("/p/c.js", "function run() {}\n"),
        ]);
        assert!(calls(&g).is_empty(), "{:?}", calls(&g));
    }
}
