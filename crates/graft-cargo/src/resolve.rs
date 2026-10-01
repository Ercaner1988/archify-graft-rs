//! Crate graph of a set of manifests: which crate depends on which, inside the indexed
//! tree (`Local`) or on a sibling repository outside it (`External`).

use crate::{DepKind, Manifest};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CrateEntry {
    /// `/`-separated path of the `Cargo.toml`.
    pub manifest_path: String,
    /// Directory of the manifest.
    pub dir: String,
    pub manifest: Manifest,
    /// Directory of the nearest `[workspace]` manifest at or above `dir` (else `dir`).
    pub workspace_dir: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Local(usize),
    External {
        /// Name in `[dependencies]`.
        name: String,
        /// Directory the path points at.
        dir: String,
        /// First folder below the common ancestor, i.e. the sibling repository.
        repo: String,
    },
}

#[derive(Debug, Clone)]
pub struct Link {
    pub from: usize,
    pub to: Target,
    pub optional: bool,
    pub kind: DepKind,
}

#[derive(Debug, Default)]
pub struct Resolved {
    pub crates: Vec<CrateEntry>,
    pub links: Vec<Link>,
}

/// `dir` + relative `spec`, with `.`/`..` folded. A leading `C:` stays a component.
pub fn join_norm(dir: &str, spec: &str) -> String {
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
    let joined = parts.join("/");
    if dir.starts_with('/') {
        format!("/{joined}")
    } else {
        joined
    }
}

fn dir_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(d, _)| d)
}

/// First folder of `target` below the longest common ancestor of `base` and `target`.
fn sibling_repo(base: &str, target: &str) -> String {
    let (a, b): (Vec<&str>, Vec<&str>) = (
        base.split('/').filter(|s| !s.is_empty()).collect(),
        target.split('/').filter(|s| !s.is_empty()).collect(),
    );
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    b.get(common)
        .or_else(|| b.last())
        .copied()
        .unwrap_or("")
        .to_string()
}

/// `manifests`: `(path of Cargo.toml, parsed)`. Only manifests with a `[package]`
/// become crates; `[workspace]` roots feed `workspace = true` dependencies. Dev
/// dependencies are dropped (they describe tests, not the architecture); registry
/// dependencies carry no path and are dropped too.
pub fn resolve(manifests: Vec<(String, Manifest)>) -> Resolved {
    let norm: Vec<(String, Manifest)> = manifests
        .into_iter()
        .map(|(p, m)| (p.replace('\\', "/"), m))
        .collect();
    let ws_roots: HashMap<&str, &Manifest> = norm
        .iter()
        .filter(|(_, m)| m.is_workspace)
        .map(|(p, m)| (dir_of(p), m))
        .collect();
    let nearest_ws = |dir: &str| -> Option<String> {
        let mut d = dir;
        loop {
            if ws_roots.contains_key(d) {
                return Some(d.to_string());
            }
            d = d.rsplit_once('/')?.0;
        }
    };

    let mut out = Resolved::default();
    for (path, m) in &norm {
        if m.package.is_none() {
            continue;
        }
        let dir = dir_of(path).to_string();
        let workspace_dir = nearest_ws(&dir).unwrap_or_else(|| dir.clone());
        out.crates.push(CrateEntry {
            manifest_path: path.clone(),
            dir,
            manifest: m.clone(),
            workspace_dir,
        });
    }
    let by_dir: HashMap<&str, usize> = out
        .crates
        .iter()
        .enumerate()
        .map(|(i, c)| (c.dir.as_str(), i))
        .collect();

    let mut links = Vec::new();
    for (from, c) in out.crates.iter().enumerate() {
        for d in c.manifest.deps.iter().filter(|d| d.kind != DepKind::Dev) {
            let (base, rel) = match (&d.path, d.workspace) {
                (Some(p), _) => (c.dir.as_str(), p.as_str()),
                (None, true) => {
                    let Some(root) = ws_roots.get(c.workspace_dir.as_str()) else {
                        continue;
                    };
                    let Some((_, p)) = root.ws_deps.iter().find(|(n, _)| *n == d.name) else {
                        continue;
                    };
                    (c.workspace_dir.as_str(), p.as_str())
                }
                (None, false) => continue,
            };
            let abs = join_norm(base, rel);
            let to = match by_dir.get(abs.as_str()) {
                Some(&i) if i != from => Target::Local(i),
                Some(_) => continue,
                None => Target::External {
                    name: d.name.clone(),
                    repo: sibling_repo(&c.dir, &abs),
                    dir: abs,
                },
            };
            links.push(Link {
                from,
                to,
                optional: d.optional,
                kind: d.kind,
            });
        }
    }
    out.links = links;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(s: &str) -> Manifest {
        Manifest::parse(s).unwrap()
    }

    #[test]
    fn local_external_and_workspace_inherited_dependencies_resolve() {
        let r = resolve(vec![
            (
                "C:/w/rust/el-Fihrist/Cargo.toml".into(),
                m("[workspace]\n[workspace.dependencies]\nfihrist-core = { path = \"crates/fihrist-core\" }\nkatla = { path = \"../katla\" }\n"),
            ),
            (
                "C:\\w\\rust\\el-Fihrist\\crates\\fihrist-core\\Cargo.toml".into(),
                m("[package]\nname=\"fihrist-core\"\n"),
            ),
            (
                "C:/w/rust/el-Fihrist/crates/fihrist-gui/Cargo.toml".into(),
                m("[package]\nname=\"fihrist-gui\"\n[dependencies]\nfihrist-core.workspace = true\nkatla.workspace = true\nserde = \"1\"\n[dev-dependencies]\nx = { path = \"../x\" }\n"),
            ),
        ]);
        assert_eq!(r.crates.len(), 2);
        assert_eq!(r.crates[0].workspace_dir, "C:/w/rust/el-Fihrist");
        assert_eq!(r.links.len(), 2, "{:?}", r.links);
        let gui = r
            .crates
            .iter()
            .position(|c| c.dir.ends_with("fihrist-gui"))
            .unwrap();
        assert!(r.links.iter().all(|l| l.from == gui));
        assert!(r.links.iter().any(|l| matches!(l.to, Target::Local(_))));
        assert!(r.links.iter().any(|l| matches!(
            &l.to,
            Target::External { name, repo, .. } if name == "katla" && repo == "katla"
        )));
    }

    #[test]
    fn join_norm_folds_dots_and_keeps_drive() {
        assert_eq!(join_norm("C:/a/b/c", "../../d/./e"), "C:/a/d/e");
        assert_eq!(join_norm("/a/b", "../c"), "/a/c");
    }
}
