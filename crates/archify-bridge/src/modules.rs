//! Collapses the file/symbol graph to crate/module level: region = crate or
//! top-level folder, component = module, connection = aggregated dependency count.

use graft_model::{CodeGraph, EdgeRelation, EdgeV1, NodeKind, NodeV1};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Files below these folders never enter the architecture.
const SKIP_DIRS: &[&str] = &[
    "tests",
    "test",
    "benches",
    "examples",
    "target",
    "node_modules",
    "docs",
    "fixtures",
    "dist",
    "build",
];
/// Container folders such as `crates/<name>/...`: the region is the name below them.
const CONTAINER_DIRS: &[&str] = &[
    "crates", "packages", "apps", "libs", "services", "modules", "cmd",
];
/// Source roots that are not a module name themselves.
const SOURCE_DIRS: &[&str] = &["src", "lib", "app", "source"];

pub struct ModuleInfo {
    pub region: String,
    pub name: String,
    pub files: usize,
    pub symbols: usize,
}

pub struct Dependency {
    pub from: String,
    pub to: String,
    pub count: u32,
    /// Most frequent relation between the two modules, used for the edge label.
    pub dominant: EdgeRelation,
}

pub struct ModuleSummary {
    /// Key: `region/module`.
    pub modules: BTreeMap<String, ModuleInfo>,
    /// Heaviest first; ties broken by name so the output is deterministic.
    pub dependencies: Vec<Dependency>,
}

fn split(path: &str) -> Vec<&str> {
    path.split(['/', '\\'])
        .filter(|p| !p.is_empty() && *p != ".")
        .collect()
}

/// Number of leading directory segments shared by every file.
fn common_prefix_len(files: &[Vec<&str>]) -> usize {
    let Some(first) = files.first() else {
        return 0;
    };
    let mut n = first.len().saturating_sub(1);
    for f in &files[1..] {
        n = n.min(f.len().saturating_sub(1));
        n = (0..n).take_while(|&i| first[i] == f[i]).count();
    }
    // `repo/src` as the shared prefix would turn every module into a region.
    if n > 0 && SOURCE_DIRS.contains(&first[n - 1]) {
        n -= 1;
    }
    n
}

/// `(region, module)` of a source file, or `None` when it is not architecture.
fn locate(parts: &[&str], prefix: usize, root: &str) -> Option<(String, String)> {
    let (file, dirs) = parts.get(prefix..)?.split_last()?;
    if dirs
        .iter()
        .any(|d| SKIP_DIRS.contains(&d.to_lowercase().as_str()))
    {
        return None;
    }
    let (region, rest): (&str, &[&str]) = match dirs {
        [] => (root, &[]),
        [c, name, rest @ ..] if CONTAINER_DIRS.contains(c) => (name, rest),
        [s, rest @ ..] if SOURCE_DIRS.contains(s) => (root, rest),
        [name, rest @ ..] => (name, rest),
    };
    let rest = match rest {
        [s, tail @ ..] if SOURCE_DIRS.contains(s) => tail,
        r => r,
    };
    let stem = file.rsplit_once('.').map_or(*file, |(s, _)| s);
    let module = rest.first().copied().unwrap_or(stem);
    Some((region.to_string(), module.to_string()))
}

fn relation_rank(r: &EdgeRelation) -> u8 {
    match r {
        EdgeRelation::Calls => 5,
        EdgeRelation::Imports => 4,
        EdgeRelation::Implements => 3,
        EdgeRelation::Extends => 2,
        EdgeRelation::References => 1,
        EdgeRelation::Contains => 0,
    }
}

pub fn summarize(graph: &CodeGraph) -> ModuleSummary {
    let file_parts: Vec<Vec<&str>> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::File)
        .map(|n| split(&n.path))
        .collect();
    let prefix = common_prefix_len(&file_parts);
    let root = match file_parts.first() {
        Some(f) if prefix > 0 => f[prefix - 1],
        _ => "root",
    };

    let mut key_of: HashMap<&str, String> = HashMap::new();
    let mut modules: BTreeMap<String, ModuleInfo> = BTreeMap::new();
    for node in &graph.nodes {
        if node.path.ends_with(".json") {
            continue;
        }
        let Some((region, name)) = locate(&split(&node.path), prefix, root) else {
            continue;
        };
        let key = format!("{region}/{name}");
        let info = modules.entry(key.clone()).or_insert_with(|| ModuleInfo {
            region,
            name,
            files: 0,
            symbols: 0,
        });
        match node.kind {
            NodeKind::File => info.files += 1,
            NodeKind::Module => {}
            _ => info.symbols += 1,
        }
        key_of.insert(node.id.as_str(), key);
    }

    // The call resolver links a call to EVERY function of that name in the whole
    // graph, so `new(`/`oku(` reach every crate. A name defined in more than one
    // module is ambiguous: it says nothing about architecture and is not counted.
    let by_id: HashMap<&str, &NodeV1> = graph.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut definers: HashMap<&str, HashSet<&str>> = HashMap::new();
    for n in &graph.nodes {
        if matches!(n.kind, NodeKind::Function | NodeKind::Method) {
            if let Some(key) = key_of.get(n.id.as_str()) {
                definers.entry(n.name.as_str()).or_default().insert(key);
            }
        }
    }
    // Like a compiler would: a name defined in the caller's own module is an internal
    // call; otherwise it resolves within the caller's crate if exactly one module
    // there defines it. Anything still ambiguous is dropped.
    let ambiguous_call = |e: &EdgeV1, from: &str, to: &str| {
        if e.relation != EdgeRelation::Calls {
            return false;
        }
        let Some(defs) = by_id
            .get(e.target.as_str())
            .and_then(|t| definers.get(t.name.as_str()))
            .filter(|d| d.len() > 1)
        else {
            return false;
        };
        let own_region = modules[from].region.as_str();
        let mut local = defs.iter().filter(|m| modules[**m].region == own_region);
        let sole_local = match (local.next(), local.next()) {
            (Some(only), None) => Some(*only),
            _ => None,
        };
        defs.contains(from) || sole_local != Some(to)
    };

    let mut agg: HashMap<(String, String), HashMap<EdgeRelation, u32>> = HashMap::new();
    for e in &graph.edges {
        if e.relation == EdgeRelation::Contains {
            continue;
        }
        let (Some(a), Some(b)) = (key_of.get(e.source.as_str()), key_of.get(e.target.as_str()))
        else {
            continue;
        };
        if a != b && !ambiguous_call(e, a, b) {
            *agg.entry((a.clone(), b.clone()))
                .or_default()
                .entry(e.relation.clone())
                .or_insert(0) += 1;
        }
    }
    let mut dependencies: Vec<Dependency> = agg
        .into_iter()
        .map(|((from, to), rels)| {
            let count = rels.values().sum();
            let dominant = rels
                .into_iter()
                .max_by_key(|(r, c)| (*c, relation_rank(r)))
                .map_or(EdgeRelation::References, |(r, _)| r);
            Dependency {
                from,
                to,
                count,
                dominant,
            }
        })
        .collect();
    dependencies.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.from.cmp(&b.from))
            .then_with(|| a.to.cmp(&b.to))
    });
    ModuleSummary {
        modules,
        dependencies,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, path: &str, kind: NodeKind) -> NodeV1 {
        NodeV1 {
            id: id.into(),
            path: path.into(),
            name: id.into(),
            kind,
            span: None,
            search_body: String::new(),
            file_residual: String::new(),
        }
    }

    fn edge(s: &str, t: &str, relation: EdgeRelation) -> EdgeV1 {
        EdgeV1 {
            source: s.into(),
            target: t.into(),
            relation,
            confidence: 1.0,
        }
    }

    fn workspace() -> CodeGraph {
        let mut g = CodeGraph::new();
        let nodes = [
            ("f1", "/w/repo/crates/core/src/db.rs", NodeKind::File),
            ("f2", "/w/repo/crates/core/src/net/http.rs", NodeKind::File),
            ("f3", "/w/repo/crates/cli/src/main.rs", NodeKind::File),
            ("f4", "/w/repo/crates/core/tests/it.rs", NodeKind::File),
            ("f5", "/w/repo/crates/core/config.json", NodeKind::File),
            ("s1", "/w/repo/crates/core/src/db.rs", NodeKind::Function),
            ("s2", "/w/repo/crates/cli/src/main.rs", NodeKind::Function),
            ("s3", "/w/repo/crates/core/src/net/http.rs", NodeKind::Class),
        ];
        for (id, path, kind) in nodes {
            g.add_node(node(id, path, kind));
        }
        g.add_edge(edge("s2", "s1", EdgeRelation::Calls));
        g.add_edge(edge("s2", "s3", EdgeRelation::Calls));
        g.add_edge(edge("f3", "f2", EdgeRelation::Imports));
        g.add_edge(edge("f1", "s1", EdgeRelation::Contains));
        g.add_edge(edge("s1", "f1", EdgeRelation::Calls)); // same module: ignored
        g
    }

    #[test]
    fn workspace_crates_become_regions_and_folders_become_modules() {
        let s = summarize(&workspace());
        let keys: Vec<&str> = s.modules.keys().map(String::as_str).collect();
        assert_eq!(keys, ["cli/main", "core/db", "core/net"]);
        assert_eq!(s.modules["core/db"].region, "core");
        assert_eq!(s.modules["core/net"].name, "net");
        assert_eq!(
            (s.modules["core/db"].files, s.modules["core/db"].symbols),
            (1, 1)
        );
    }

    #[test]
    fn tests_dirs_and_json_files_do_not_enter_the_architecture() {
        let s = summarize(&workspace());
        assert!(s
            .modules
            .keys()
            .all(|k| !k.contains("it") && !k.contains("config")));
    }

    #[test]
    fn dependencies_are_aggregated_and_ignore_contains_and_self_edges() {
        let s = summarize(&workspace());
        assert_eq!(s.dependencies.len(), 2);
        let d = &s.dependencies[0];
        assert_eq!((d.from.as_str(), d.to.as_str()), ("cli/main", "core/net"));
        assert_eq!(d.count, 2);
        assert_eq!(d.dominant, EdgeRelation::Calls);
        assert_eq!(s.dependencies[1].count, 1);
    }

    #[test]
    fn calls_to_names_defined_in_several_modules_do_not_create_links() {
        let mut g = CodeGraph::new();
        let files = [("a", "x"), ("b", "y"), ("c", "z"), ("d", "u")];
        for (krate, m) in files {
            let path = format!("/w/repo/crates/{krate}/src/{m}.rs");
            g.add_node(node(&format!("file-{krate}"), &path, NodeKind::File));
            let name = match krate {
                "a" => "caller",
                "d" => "unique_fn",
                _ => "new",
            };
            let mut f = node(&format!("fn-{krate}"), &path, NodeKind::Function);
            f.name = name.into();
            g.add_node(f);
        }
        g.add_edge(edge("fn-a", "fn-b", EdgeRelation::Calls)); // `new` is defined twice
        g.add_edge(edge("fn-a", "fn-c", EdgeRelation::Calls));
        g.add_edge(edge("fn-a", "fn-d", EdgeRelation::Calls)); // unique: kept
        let s = summarize(&g);
        assert_eq!(s.dependencies.len(), 1);
        assert_eq!(s.dependencies[0].to, "d/u");
    }

    /// `(crate, module, function)` triples -> one file per module, one function each;
    /// function ids are `crate/module:function`.
    fn graph_of(funcs: &[(&str, &str, &str)]) -> CodeGraph {
        let mut g = CodeGraph::new();
        let mut seen = HashSet::new();
        for (krate, module, func) in funcs {
            let path = format!("/w/repo/crates/{krate}/src/{module}.rs");
            if seen.insert(path.clone()) {
                g.add_node(node(
                    &format!("file:{krate}/{module}"),
                    &path,
                    NodeKind::File,
                ));
            }
            let mut f = node(
                &format!("{krate}/{module}:{func}"),
                &path,
                NodeKind::Function,
            );
            f.name = (*func).into();
            g.add_node(f);
        }
        g
    }

    #[test]
    fn an_ambiguous_name_resolves_inside_the_callers_own_crate() {
        let mut g = graph_of(&[
            ("a", "x", "run"),
            ("a", "y", "helper"),
            ("b", "z", "helper"),
        ]);
        g.add_edge(edge("a/x:run", "a/y:helper", EdgeRelation::Calls));
        g.add_edge(edge("a/x:run", "b/z:helper", EdgeRelation::Calls));
        let s = summarize(&g);
        assert_eq!(s.dependencies.len(), 1);
        assert_eq!(
            (
                s.dependencies[0].from.as_str(),
                s.dependencies[0].to.as_str()
            ),
            ("a/x", "a/y")
        );
    }

    #[test]
    fn a_name_the_callers_own_module_defines_is_an_internal_call() {
        let mut g = graph_of(&[
            ("a", "x", "run"),
            ("a", "x", "helper"),
            ("a", "y", "helper"),
        ]);
        g.add_edge(edge("a/x:run", "a/y:helper", EdgeRelation::Calls));
        assert!(summarize(&g).dependencies.is_empty());
    }

    #[test]
    fn single_crate_src_layout_uses_the_repo_folder_as_region() {
        let mut g = CodeGraph::new();
        g.add_node(node("a", "C:\\w\\repo\\src\\a.rs", NodeKind::File));
        g.add_node(node("c", "C:\\w\\repo\\src\\b\\c.rs", NodeKind::File));
        let s = summarize(&g);
        let keys: Vec<&str> = s.modules.keys().map(String::as_str).collect();
        assert_eq!(keys, ["repo/a", "repo/b"]);
    }

    #[test]
    fn empty_graph_summarizes_to_nothing() {
        let s = summarize(&CodeGraph::new());
        assert!(s.modules.is_empty() && s.dependencies.is_empty());
    }
}
