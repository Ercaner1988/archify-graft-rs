//! Level 1 of the diagram: one box per crate. Boxes, required/optional dependencies and
//! how heavily each one is used come from `Cargo.toml` and the `use` paths of the code,
//! not from guessing which folders look like modules.

use crate::labels::role_for_crate;
use archify_ir::SemanticRole;
use graft_cargo::{resolve, Manifest, Target};
use graft_model::{CodeGraph, EdgeRelation, NodeKind};
use std::collections::{HashMap, HashSet};

pub struct CrateBox {
    /// Package name for crates of the indexed tree, `ext:<repo>/<name>` for siblings.
    pub id: String,
    pub label: String,
    /// Repository / workspace the box is drawn in.
    pub group: String,
    pub external: bool,
    /// Code lines outside test code (the sandik-siniri metric), 0 for external crates.
    pub lines: usize,
    pub bin: bool,
    pub lib: bool,
    pub role: SemanticRole,
    /// Most used public functions, for the tour text.
    pub api: Vec<String>,
    /// Normalized directory of the crate (empty for external ones).
    pub dir: String,
}

pub struct CrateEdge {
    pub from: usize,
    pub to: usize,
    /// Number of `use` paths in `from`'s code that name `to`.
    pub uses: u32,
    pub optional: bool,
}

pub struct CrateSummary {
    pub boxes: Vec<CrateBox>,
    pub edges: Vec<CrateEdge>,
}

pub fn norm(path: &str) -> String {
    path.replace('\\', "/")
}

/// Tests, benches and examples are not part of the product.
pub fn is_test_path(path: &str) -> bool {
    path.split('/')
        .any(|s| matches!(s, "tests" | "benches" | "examples" | "fixtures"))
}

fn loc_of(graph: &CodeGraph, file_id: &str) -> usize {
    graph.facts.get(file_id).map_or(0, |f| {
        f.iter()
            .filter_map(|x| x.strip_prefix("loc:")?.parse::<usize>().ok())
            .sum()
    })
}

/// `None` when the graph has no `Cargo.toml` crates (the caller falls back to modules).
pub fn summarize(graph: &CodeGraph) -> Option<CrateSummary> {
    let manifests: Vec<(String, Manifest)> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::File && n.name == "Cargo.toml")
        .filter(|n| !is_test_path(&norm(&n.path)))
        .filter_map(|n| {
            let specs = graph.imports.get(&n.id)?;
            Some((n.path.clone(), Manifest::from_specs(specs)))
        })
        .collect();
    let resolved = resolve(manifests);
    if resolved.crates.is_empty() {
        return None;
    }

    // Deepest crate directory first, so a nested crate owns its own files.
    let mut owners: Vec<(&str, usize)> = resolved
        .crates
        .iter()
        .enumerate()
        .map(|(i, c)| (c.dir.as_str(), i))
        .collect();
    owners.sort_by_key(|(d, _)| std::cmp::Reverse(d.len()));
    let owner_of = |path: &str| {
        owners
            .iter()
            .find(|(d, _)| path.strip_prefix(d).is_some_and(|r| r.starts_with('/')))
            .map(|&(_, i)| i)
    };

    let paths: HashSet<String> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::File)
        .map(|n| norm(&n.path))
        .collect();
    let n_local = resolved.crates.len();
    let mut lines = vec![0usize; n_local];
    let mut file_owner: HashMap<&str, usize> = HashMap::new();
    for n in graph.nodes.iter().filter(|n| n.kind == NodeKind::File) {
        let p = norm(&n.path);
        if is_test_path(&p) {
            continue;
        }
        if let Some(i) = owner_of(&p) {
            file_owner.insert(n.id.as_str(), i);
            if p.ends_with(".rs") {
                lines[i] += loc_of(graph, &n.id);
            }
        }
    }

    // Public functions ranked by callers in other crates.
    let crate_of_path: HashMap<&str, Option<usize>> = graph
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Function | NodeKind::Method))
        .map(|n| {
            (
                n.id.as_str(),
                file_owner.get(format!("file:{}", n.path).as_str()).copied(),
            )
        })
        .collect();
    let mut outside: HashMap<&str, u32> = HashMap::new();
    for e in graph
        .edges
        .iter()
        .filter(|e| e.relation == EdgeRelation::Calls)
    {
        let (a, b) = (
            crate_of_path.get(e.source.as_str()),
            crate_of_path.get(e.target.as_str()),
        );
        if let (Some(Some(a)), Some(Some(b))) = (a, b) {
            if a != b {
                *outside.entry(e.target.as_str()).or_default() += 1;
            }
        }
    }
    let mut api: Vec<Vec<(u32, &str)>> = vec![Vec::new(); n_local];
    for n in graph.nodes.iter().filter(|n| n.kind == NodeKind::Function) {
        let is_pub = graph
            .facts
            .get(&n.id)
            .is_some_and(|f| f.iter().any(|x| x == "pub"));
        if let (true, Some(Some(i))) = (is_pub, crate_of_path.get(n.id.as_str())) {
            api[*i].push((
                outside.get(n.id.as_str()).copied().unwrap_or(0),
                n.name.as_str(),
            ));
        }
    }

    let mut boxes: Vec<CrateBox> = Vec::new();
    let has_dependents: HashSet<usize> = resolved
        .links
        .iter()
        .filter_map(|l| match l.to {
            Target::Local(i) => Some(i),
            Target::External { .. } => None,
        })
        .collect();
    for (i, c) in resolved.crates.iter().enumerate() {
        let name = c.manifest.package.clone().unwrap_or_default();
        let src = format!("{}/src/", c.dir);
        let bin = !c.manifest.bins.is_empty()
            || paths.contains(&format!("{src}main.rs"))
            || paths.iter().any(|p| p.starts_with(&format!("{src}bin/")));
        let lib = paths.contains(&format!("{src}lib.rs"));
        let deps: Vec<&str> = c.manifest.deps.iter().map(|d| d.name.as_str()).collect();
        let mut pub_api = std::mem::take(&mut api[i]);
        pub_api.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(b.1)));
        pub_api.dedup_by(|a, b| a.1 == b.1);
        let group = c.workspace_dir.rsplit('/').next().unwrap_or("").to_string();
        boxes.push(CrateBox {
            id: name.clone(),
            role: role_for_crate(&name, &deps, bin && !lib, has_dependents.contains(&i)),
            label: name,
            group,
            external: false,
            lines: lines[i],
            bin,
            lib,
            api: pub_api
                .iter()
                .take(4)
                .map(|(_, n)| (*n).to_string())
                .collect(),
            dir: c.dir.clone(),
        });
    }

    let mut ext_ix: HashMap<String, usize> = HashMap::new();
    let mut edges: Vec<CrateEdge> = Vec::new();
    for l in &resolved.links {
        let to = match &l.to {
            Target::Local(i) => *i,
            Target::External { name, repo, .. } => {
                let id = format!("ext:{repo}/{name}");
                *ext_ix.entry(id.clone()).or_insert_with(|| {
                    boxes.push(CrateBox {
                        id,
                        label: name.clone(),
                        group: repo.clone(),
                        external: true,
                        lines: 0,
                        bin: false,
                        lib: true,
                        role: SemanticRole::External,
                        api: Vec::new(),
                        dir: String::new(),
                    });
                    boxes.len() - 1
                })
            }
        };
        match edges.iter_mut().find(|e| e.from == l.from && e.to == to) {
            Some(e) => e.optional &= l.optional,
            None => edges.push(CrateEdge {
                from: l.from,
                to,
                uses: 0,
                optional: l.optional,
            }),
        }
    }

    // `use dep_name::...` paths per dependent crate.
    let code_names: Vec<(String, usize)> = {
        let mut v: Vec<(String, usize)> = resolved
            .crates
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.manifest.code_name().map(|n| (n, i)))
            .collect();
        v.extend(
            boxes
                .iter()
                .enumerate()
                .filter(|(_, b)| b.external)
                .map(|(i, b)| (b.label.replace('-', "_"), i)),
        );
        v
    };
    for (file_id, &from) in &file_owner {
        let Some(specs) = graph.imports.get(*file_id) else {
            continue;
        };
        for root in specs
            .iter()
            .filter_map(|s| s.strip_prefix("rs:"))
            .filter_map(|s| s.split("::").next())
        {
            let Some(&(_, to)) = code_names.iter().find(|(n, _)| n == root) else {
                continue;
            };
            if let Some(e) = edges.iter_mut().find(|e| e.from == from && e.to == to) {
                e.uses += 1;
            }
        }
    }
    edges.sort_by_key(|e| (e.from, e.to));
    Some(CrateSummary { boxes, edges })
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_model::NodeV1;

    fn file(path: &str, name: &str) -> NodeV1 {
        NodeV1 {
            id: format!("file:{path}"),
            path: path.into(),
            name: name.into(),
            kind: NodeKind::File,
            span: None,
            search_body: String::new(),
            file_residual: String::new(),
        }
    }

    /// `app` uses `core` (path dep) and `katla` (sibling repo); `core` has 10 code lines.
    fn workspace() -> CodeGraph {
        let mut g = CodeGraph::new();
        let manifest = |toml: &str| Manifest::parse(toml).unwrap().to_specs();
        for (p, toml) in [
            ("/w/r/Cargo.toml", "[workspace]\n"),
            ("/w/r/core/Cargo.toml", "[package]\nname=\"core-x\"\n"),
            (
                "/w/r/app/Cargo.toml",
                "[package]\nname=\"app\"\n[dependencies]\ncore-x={path=\"../core\"}\nkatla={path=\"../../katla\"}\nclap=\"4\"\n",
            ),
        ] {
            g.add_node(file(p, "Cargo.toml"));
            g.imports.insert(format!("file:{p}"), manifest(toml));
        }
        g.add_node(file("/w/r/core/src/lib.rs", "lib.rs"));
        g.facts
            .insert("file:/w/r/core/src/lib.rs".into(), vec!["loc:10".into()]);
        g.add_node(file("/w/r/app/src/main.rs", "main.rs"));
        g.add_node(file("/w/r/app/tests/t.rs", "t.rs"));
        g.imports.insert(
            "file:/w/r/app/src/main.rs".into(),
            vec![
                "rs:core_x::A".into(),
                "rs:core_x::B".into(),
                "rs:katla::f".into(),
                "rs:std::fs".into(),
            ],
        );
        g
    }

    #[test]
    fn crates_dependencies_and_use_counts_come_from_cargo_and_imports() {
        let s = summarize(&workspace()).unwrap();
        let names: Vec<&str> = s.boxes.iter().map(|b| b.id.as_str()).collect();
        assert_eq!(names, ["core-x", "app", "ext:katla/katla"]);
        assert_eq!(s.boxes[0].lines, 10);
        assert_eq!(s.boxes[0].group, "r");
        assert!(s.boxes[1].bin && !s.boxes[1].lib);
        assert!(s.boxes[2].external);
        let uses = |a: usize, b: usize| {
            s.edges
                .iter()
                .find(|e| e.from == a && e.to == b)
                .map(|e| e.uses)
        };
        assert_eq!(uses(1, 0), Some(2));
        assert_eq!(uses(1, 2), Some(1));
    }

    #[test]
    fn a_graph_without_manifests_has_no_crate_level() {
        assert!(summarize(&CodeGraph::new()).is_none());
    }
}
