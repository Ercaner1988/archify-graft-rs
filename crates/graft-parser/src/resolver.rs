//! Call resolver: links the call sites recorded in `CodeGraph::facts` (`c:<token>`) to
//! function definitions. A call is linked only when the language rules pin it down:
//! same file, a file the caller imports, a named module path, an `impl` owner, or a
//! unique definition inside the caller's own crate. A bare name never reaches across
//! crates on its own, and std method names (`push`, `path`, `len`) are not calls into
//! the project.

use crate::imports::RustIndex;
use graft_model::{CodeGraph, EdgeRelation, EdgeV1, NodeKind};
use std::collections::{HashMap, HashSet};

/// Method names so common in std and derives that `.name(` says nothing about the
/// project's own functions.
const STD_METHODS: &[&str] = &[
    "new",
    "default",
    "from",
    "into",
    "clone",
    "len",
    "push",
    "pop",
    "get",
    "insert",
    "remove",
    "contains",
    "iter",
    "next",
    "map",
    "unwrap",
    "expect",
    "is_empty",
    "fmt",
    "as_str",
    "to_string",
    "eq",
    "cmp",
    "partial_cmp",
    "path",
    "id",
    "name",
    "join",
    "send",
    "recv",
    "lock",
    "read",
    "write",
    "open",
    "close",
    "run",
    "build",
    "parse",
    "take",
    "set",
    "add",
    "sub",
    "drop",
    "deref",
    "hash",
    "borrow",
    "as_ref",
    "as_mut",
    "to_owned",
    "extend",
    "clear",
    "first",
    "last",
    "find",
    "filter",
    "collect",
    "flush",
    "kind",
    "key",
    "value",
    "keys",
    "values",
    "or_default",
    "entry",
    "and_then",
    "ok",
    "err",
    "min",
    "max",
    "sort",
    "rev",
];

struct Def<'a> {
    id: &'a str,
    file: String,
    owner: Option<&'a str>,
    is_pub: bool,
    krate: Option<usize>,
}

/// Directory -> crate index; a file belongs to the deepest crate above it.
pub(crate) struct CrateMap {
    by_dir: HashMap<String, usize>,
}

impl CrateMap {
    pub(crate) fn new(graph: &CodeGraph) -> Self {
        let by_dir = graph
            .nodes
            .iter()
            .filter(|n| n.kind == NodeKind::Crate)
            .enumerate()
            .map(|(i, n)| {
                let p = n.path.replace('\\', "/");
                (
                    p.rsplit_once('/')
                        .map_or(String::new(), |(d, _)| d.to_string()),
                    i,
                )
            })
            .collect();
        CrateMap { by_dir }
    }

    pub(crate) fn owner(&self, path: &str) -> Option<usize> {
        let mut d = path;
        while let Some((parent, _)) = d.rsplit_once('/') {
            if let Some(&i) = self.by_dir.get(parent) {
                return Some(i);
            }
            d = parent;
        }
        None
    }
}

fn base_name(name: &str) -> &str {
    name.split('<').next().unwrap_or(name)
}

fn module_name(path: &str) -> &str {
    let stem = path
        .rsplit('/')
        .next()
        .unwrap_or("")
        .trim_end_matches(".rs");
    if stem == "mod" {
        path.rsplit('/').nth(1).unwrap_or("")
    } else {
        stem
    }
}

fn unique<'a>(cands: impl Iterator<Item = &'a Def<'a>>) -> Option<&'a Def<'a>> {
    let mut it = cands;
    match (it.next(), it.next()) {
        (Some(d), None) => Some(d),
        _ => None,
    }
}

pub struct CallResolver;

impl CallResolver {
    /// Rebuilds every `Calls` edge from the `c:` facts. Idempotent.
    pub fn resolve_calls(graph: &mut CodeGraph) {
        graph.edges.retain(|e| e.relation != EdgeRelation::Calls);
        let crates = CrateMap::new(graph);
        let rust = RustIndex::new(graph);

        let mut defs: HashMap<&str, Vec<Def>> = HashMap::new();
        for n in graph
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Function | NodeKind::Method))
        {
            let path = n.path.replace('\\', "/");
            let rest =
                n.id.strip_prefix(n.path.as_str())
                    .and_then(|r| r.strip_prefix(':'))
                    .unwrap_or(&n.id);
            let rest = rest.split('#').next().unwrap_or(rest);
            defs.entry(base_name(&n.name)).or_default().push(Def {
                id: &n.id,
                owner: rest.rsplit_once("::").map(|(o, _)| o),
                is_pub: graph
                    .facts
                    .get(&n.id)
                    .is_some_and(|f| f.iter().any(|x| x == "pub")),
                krate: crates.owner(&path),
                file: path,
            });
        }
        let id_of_path: HashMap<String, &str> = graph
            .nodes
            .iter()
            .filter(|n| n.kind == NodeKind::File)
            .map(|n| (n.path.replace('\\', "/"), n.id.as_str()))
            .collect();
        let mut imported: HashMap<&str, HashSet<&str>> = HashMap::new();
        for e in graph
            .edges
            .iter()
            .filter(|e| e.relation == EdgeRelation::Imports)
        {
            imported
                .entry(e.source.as_str())
                .or_default()
                .insert(e.target.as_str());
        }
        let mut deps_of: HashMap<usize, HashSet<usize>> = HashMap::new();
        {
            let crate_ix: HashMap<&str, usize> = graph
                .nodes
                .iter()
                .filter(|n| n.kind == NodeKind::Crate)
                .enumerate()
                .map(|(i, n)| (n.id.as_str(), i))
                .collect();
            for e in graph
                .edges
                .iter()
                .filter(|e| e.relation == EdgeRelation::DependsOn)
            {
                if let (Some(&a), Some(&b)) = (
                    crate_ix.get(e.source.as_str()),
                    crate_ix.get(e.target.as_str()),
                ) {
                    deps_of.entry(a).or_default().insert(b);
                }
            }
        }

        let mut seen: HashSet<(&str, &str)> = HashSet::new();
        let mut new_edges = Vec::new();
        for f in graph
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Function | NodeKind::Method))
        {
            let Some(facts) = graph.facts.get(&f.id) else {
                continue;
            };
            let path = f.path.replace('\\', "/");
            let file_id = format!("file:{}", f.path);
            let empty = HashSet::new();
            let imports = imported.get(file_id.as_str()).unwrap_or(&empty);
            let me = crates.owner(&path);
            let my_owner =
                f.id.strip_prefix(f.path.as_str())
                    .and_then(|r| r.strip_prefix(':'))
                    .and_then(|r| r.rsplit_once("::"))
                    .map(|(o, _)| o);
            let in_scope = |d: &Def| {
                d.file == path
                    || id_of_path
                        .get(&d.file)
                        .is_some_and(|id| imports.contains(id))
            };
            let same_crate = |d: &Def| d.krate == me;

            for token in facts.iter().filter_map(|x| x.strip_prefix("c:")) {
                let target = if let Some(name) = token.strip_prefix('.') {
                    let Some(cands) = defs.get(name) else {
                        continue;
                    };
                    let methods = || cands.iter().filter(|d| d.owner.is_some());
                    let own = methods()
                        .filter(|d| d.file == path && d.owner == my_owner && my_owner.is_some());
                    if let Some(d) = unique(own) {
                        Some((d, 0.85))
                    } else if STD_METHODS.contains(&name) {
                        None
                    } else if let Some(d) = unique(methods().filter(|d| d.file == path)) {
                        Some((d, 0.7))
                    } else if let Some(d) = unique(methods().filter(|d| in_scope(d))) {
                        Some((d, 0.65))
                    } else {
                        unique(methods().filter(|d| same_crate(d))).map(|d| (d, 0.55))
                    }
                } else if token.contains("::") {
                    let segs: Vec<&str> = token.split("::").collect();
                    let (name, qual) = (segs[segs.len() - 1], &segs[..segs.len() - 1]);
                    let Some(cands) = defs.get(name) else {
                        continue;
                    };
                    let last = qual[qual.len() - 1];
                    if last == "Self" || last.chars().next().is_some_and(char::is_uppercase) {
                        let owner = if last == "Self" { my_owner } else { Some(last) };
                        let typed = || {
                            cands
                                .iter()
                                .filter(|d| d.owner.is_some() && d.owner == owner)
                        };
                        unique(typed().filter(|d| d.file == path))
                            .or_else(|| unique(typed().filter(|d| in_scope(d))))
                            .or_else(|| unique(typed().filter(|d| same_crate(d))))
                            .map(|d| (d, 0.85))
                    } else {
                        let file = rust.resolve_segs(&path, qual).or_else(|| {
                            imports
                                .iter()
                                .find(|t| module_name(t.trim_start_matches("file:")) == qual[0])
                                .map(|t| t.to_string())
                        });
                        file.and_then(|fid| {
                            let p = fid.trim_start_matches("file:").replace('\\', "/");
                            unique(cands.iter().filter(|d| d.file == p && d.owner.is_none()))
                                .map(|d| (d, 0.95))
                        })
                    }
                } else {
                    let Some(cands) = defs.get(token) else {
                        continue;
                    };
                    let free = || cands.iter().filter(|d| d.owner.is_none());
                    unique(free().filter(|d| d.file == path))
                        .map(|d| (d, 0.95))
                        .or_else(|| unique(free().filter(|d| in_scope(d))).map(|d| (d, 0.9)))
                        .or_else(|| unique(free().filter(|d| same_crate(d))).map(|d| (d, 0.75)))
                        .or_else(|| {
                            let deps = me.and_then(|m| deps_of.get(&m))?;
                            unique(
                                free().filter(|d| {
                                    d.is_pub && d.krate.is_some_and(|k| deps.contains(&k))
                                }),
                            )
                            .map(|d| (d, 0.6))
                        })
                };
                if let Some((d, confidence)) = target {
                    if d.id != f.id && seen.insert((f.id.as_str(), d.id)) {
                        new_edges.push(EdgeV1 {
                            source: f.id.clone(),
                            target: d.id.to_string(),
                            relation: EdgeRelation::Calls,
                            confidence,
                        });
                    }
                }
            }
        }
        graph.edges.extend(new_edges);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_model::NodeV1;

    fn function(
        path: &str,
        name: &str,
        owner: Option<&str>,
        calls: &[&str],
    ) -> (NodeV1, Vec<String>) {
        let id = match owner {
            Some(o) => format!("{path}:{o}::{name}"),
            None => format!("{path}:{name}"),
        };
        let kind = if owner.is_some() {
            NodeKind::Method
        } else {
            NodeKind::Function
        };
        let node = NodeV1 {
            id,
            path: path.to_string(),
            name: name.to_string(),
            kind,
            span: None,
            search_body: String::new(),
            file_residual: String::new(),
        };
        (node, calls.iter().map(|c| format!("c:{c}")).collect())
    }

    fn file(path: &str) -> NodeV1 {
        NodeV1 {
            id: format!("file:{path}"),
            path: path.to_string(),
            name: path.rsplit('/').next().unwrap().to_string(),
            kind: NodeKind::File,
            span: None,
            search_body: String::new(),
            file_residual: String::new(),
        }
    }

    fn graph(files: &[&str], fns: Vec<(NodeV1, Vec<String>)>) -> CodeGraph {
        let mut g = CodeGraph::new();
        for f in files {
            g.add_node(file(f));
        }
        for (n, facts) in fns {
            g.facts.insert(n.id.clone(), facts);
            g.add_node(n);
        }
        g
    }

    fn targets(g: &CodeGraph) -> Vec<String> {
        let mut v: Vec<String> = g
            .edges
            .iter()
            .filter(|e| e.relation == EdgeRelation::Calls)
            .map(|e| {
                format!(
                    "{}->{}",
                    e.source.rsplit('/').next().unwrap(),
                    e.target.rsplit('/').next().unwrap()
                )
            })
            .collect();
        v.sort();
        v
    }

    #[test]
    fn a_unique_bare_name_in_the_same_crate_is_linked() {
        let mut g = graph(
            &["/w/c/src/a.rs", "/w/c/src/b.rs"],
            vec![
                function("/w/c/src/a.rs", "run", None, &["calc"]),
                function("/w/c/src/b.rs", "calc", None, &[]),
            ],
        );
        CallResolver::resolve_calls(&mut g);
        assert_eq!(targets(&g), ["a.rs:run->b.rs:calc"]);
    }

    #[test]
    fn std_method_names_and_type_names_are_not_calls_into_the_project() {
        let mut g = graph(
            &["/w/c/src/a.rs", "/w/c/src/b.rs"],
            vec![
                function(
                    "/w/c/src/a.rs",
                    "run",
                    None,
                    &[".path", ".push", "Path::new", "Some"],
                ),
                function("/w/c/src/b.rs", "path", Some("Tez"), &[]),
                function("/w/c/src/b.rs", "push", Some("Tez"), &[]),
            ],
        );
        CallResolver::resolve_calls(&mut g);
        assert!(targets(&g).is_empty(), "{:?}", targets(&g));
    }

    #[test]
    fn an_ambiguous_name_is_not_guessed() {
        let mut g = graph(
            &["/w/c/src/a.rs", "/w/c/src/b.rs", "/w/c/src/d.rs"],
            vec![
                function("/w/c/src/a.rs", "run", None, &["oku"]),
                function("/w/c/src/b.rs", "oku", None, &[]),
                function("/w/c/src/d.rs", "oku", None, &[]),
            ],
        );
        CallResolver::resolve_calls(&mut g);
        assert!(targets(&g).is_empty());
    }

    #[test]
    fn module_paths_self_and_type_paths_pick_the_right_definition() {
        let mut g = graph(
            &[
                "/w/c/src/lib.rs",
                "/w/c/src/a.rs",
                "/w/c/src/goz.rs",
                "/w/c/src/tez.rs",
            ],
            vec![
                function(
                    "/w/c/src/a.rs",
                    "run",
                    None,
                    &["crate::goz::ac", "Tez::yeni"],
                ),
                function("/w/c/src/a.rs", "yardim", Some("Tez"), &["Self::yeni"]),
                function("/w/c/src/a.rs", "yeni", Some("Tez"), &[]),
                function("/w/c/src/goz.rs", "ac", None, &[]),
                function("/w/c/src/tez.rs", "ac", None, &[]),
            ],
        );
        CallResolver::resolve_calls(&mut g);
        assert_eq!(
            targets(&g),
            [
                "a.rs:Tez::yardim->a.rs:Tez::yeni",
                "a.rs:run->a.rs:Tez::yeni",
                "a.rs:run->goz.rs:ac"
            ]
        );
    }

    #[test]
    fn resolving_a_large_graph_is_linear() {
        let mut g = CodeGraph::new();
        for i in 0..20_000 {
            let next = (i + 1) % 20_000;
            let path = format!("/w/c/src/f{i}.rs");
            g.add_node(file(&path));
            let (n, facts) = function(&path, &format!("fn_{i}"), None, &[&format!("fn_{next}")]);
            g.facts.insert(n.id.clone(), facts);
            g.add_node(n);
        }
        let start = std::time::Instant::now();
        CallResolver::resolve_calls(&mut g);
        assert_eq!(g.edges.len(), 20_000);
        assert!(start.elapsed().as_secs() < 10, "resolver is not linear");
    }
}
