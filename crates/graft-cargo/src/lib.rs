//! graft-cargo: reads `Cargo.toml` files and turns them into a crate graph. Pure data
//! in, pure data out (no file access, no graph types), so the indexer and the diagram
//! bridge share one truth about "who depends on whom".

mod resolve;

pub use resolve::{join_norm, resolve, CrateEntry, Link, Resolved, Target};

/// Tag of a manifest spec inside `CodeGraph::imports` (`cargo:<field>|<value>...`).
pub const SPEC_PREFIX: &str = "cargo:";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepKind {
    Normal,
    Dev,
    Build,
}

impl DepKind {
    fn tag(self) -> char {
        match self {
            DepKind::Normal => 'n',
            DepKind::Dev => 'd',
            DepKind::Build => 'b',
        }
    }
    fn from_tag(c: &str) -> DepKind {
        match c {
            "d" => DepKind::Dev,
            "b" => DepKind::Build,
            _ => DepKind::Normal,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dep {
    /// The key in `[dependencies]` (what the source code calls the crate, `-` kept).
    pub name: String,
    /// Relative path of a path dependency.
    pub path: Option<String>,
    pub optional: bool,
    pub kind: DepKind,
    /// `name.workspace = true`: the path lives in the workspace root manifest.
    pub workspace: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Manifest {
    pub package: Option<String>,
    pub lib_name: Option<String>,
    pub bins: Vec<String>,
    pub deps: Vec<Dep>,
    pub is_workspace: bool,
    /// `[workspace.dependencies]` entries that carry a path: `(name, path)`.
    pub ws_deps: Vec<(String, String)>,
}

fn dep_of(name: &str, v: &toml::Value, kind: DepKind) -> Dep {
    let table = v.as_table();
    let get_bool = |k: &str| {
        table
            .and_then(|t| t.get(k))
            .and_then(toml::Value::as_bool)
            .unwrap_or(false)
    };
    Dep {
        name: name.to_string(),
        path: table
            .and_then(|t| t.get("path"))
            .and_then(toml::Value::as_str)
            .map(str::to_string),
        optional: get_bool("optional"),
        kind,
        workspace: get_bool("workspace"),
    }
}

fn collect_deps(table: &toml::Table, out: &mut Vec<Dep>) {
    for (key, kind) in [
        ("dependencies", DepKind::Normal),
        ("dev-dependencies", DepKind::Dev),
        ("build-dependencies", DepKind::Build),
    ] {
        if let Some(t) = table.get(key).and_then(toml::Value::as_table) {
            out.extend(t.iter().map(|(n, v)| dep_of(n, v, kind)));
        }
    }
}

impl Manifest {
    /// `None` when the text is not valid TOML.
    pub fn parse(content: &str) -> Option<Manifest> {
        let root: toml::Table = content.parse().ok()?;
        let mut m = Manifest::default();
        let str_at = |t: Option<&toml::Value>, k: &str| {
            t.and_then(toml::Value::as_table)
                .and_then(|t| t.get(k))
                .and_then(toml::Value::as_str)
                .map(str::to_string)
        };
        m.package = str_at(root.get("package"), "name");
        m.lib_name = str_at(root.get("lib"), "name");
        if let Some(bins) = root.get("bin").and_then(toml::Value::as_array) {
            m.bins = bins
                .iter()
                .filter_map(|b| str_at(Some(b), "name"))
                .collect();
        }
        collect_deps(&root, &mut m.deps);
        if let Some(targets) = root.get("target").and_then(toml::Value::as_table) {
            for t in targets.values().filter_map(toml::Value::as_table) {
                collect_deps(t, &mut m.deps);
            }
        }
        if let Some(ws) = root.get("workspace").and_then(toml::Value::as_table) {
            m.is_workspace = true;
            if let Some(deps) = ws.get("dependencies").and_then(toml::Value::as_table) {
                for (name, v) in deps {
                    if let Some(p) = dep_of(name, v, DepKind::Normal).path {
                        m.ws_deps.push((name.clone(), p));
                    }
                }
            }
        }
        Some(m)
    }

    /// Flat text form kept in the graph (`CodeGraph::imports` of the manifest file).
    pub fn to_specs(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(p) = &self.package {
            out.push(format!("{SPEC_PREFIX}pkg|{p}"));
        }
        if let Some(l) = &self.lib_name {
            out.push(format!("{SPEC_PREFIX}lib|{l}"));
        }
        out.extend(self.bins.iter().map(|b| format!("{SPEC_PREFIX}bin|{b}")));
        if self.is_workspace {
            out.push(format!("{SPEC_PREFIX}ws|"));
        }
        for (n, p) in &self.ws_deps {
            out.push(format!("{SPEC_PREFIX}wsdep|{n}|{p}"));
        }
        for d in &self.deps {
            out.push(format!(
                "{SPEC_PREFIX}dep|{}|{}|{}|{}|{}",
                d.kind.tag(),
                d.name,
                u8::from(d.optional),
                u8::from(d.workspace),
                d.path.as_deref().unwrap_or("")
            ));
        }
        out
    }

    /// Inverse of [`Manifest::to_specs`]; other specs of the same list are ignored.
    pub fn from_specs(specs: &[String]) -> Manifest {
        let mut m = Manifest::default();
        for s in specs {
            let Some(rest) = s.strip_prefix(SPEC_PREFIX) else {
                continue;
            };
            let f: Vec<&str> = rest.splitn(6, '|').collect();
            match f.as_slice() {
                ["pkg", v] => m.package = Some((*v).to_string()),
                ["lib", v] => m.lib_name = Some((*v).to_string()),
                ["bin", v] => m.bins.push((*v).to_string()),
                ["ws", _] => m.is_workspace = true,
                ["wsdep", n, p] => m.ws_deps.push(((*n).to_string(), (*p).to_string())),
                ["dep", kind, name, opt, ws, path] => m.deps.push(Dep {
                    name: (*name).to_string(),
                    path: (!path.is_empty()).then(|| (*path).to_string()),
                    optional: *opt == "1",
                    kind: DepKind::from_tag(kind),
                    workspace: *ws == "1",
                }),
                _ => {}
            }
        }
        m
    }

    /// Name the source code uses for this crate: `[lib] name`, else the package name
    /// with `-` as `_`.
    pub fn code_name(&self) -> Option<String> {
        self.lib_name
            .clone()
            .or_else(|| self.package.as_ref().map(|p| p.replace('-', "_")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[package]
name = "pasli-cekirdek"
[lib]
name = "pasli_cekirdek"
[[bin]]
name = "pasli"
[dependencies]
katla = { path = "../../katla" }
serde.workspace = true
tuval = { path = "../tuval", optional = true }
clap = "4"
[dev-dependencies]
tempfile = "3"
[target.'cfg(windows)'.dependencies]
winapi = "0.3"
"#;

    #[test]
    fn parses_package_lib_bins_and_every_dependency_table() {
        let m = Manifest::parse(SAMPLE).unwrap();
        assert_eq!(m.package.as_deref(), Some("pasli-cekirdek"));
        assert_eq!(m.code_name().as_deref(), Some("pasli_cekirdek"));
        assert_eq!(m.bins, ["pasli"]);
        let names: Vec<&str> = m.deps.iter().map(|d| d.name.as_str()).collect();
        for n in ["katla", "serde", "tuval", "clap", "tempfile", "winapi"] {
            assert!(names.contains(&n), "{n} missing in {names:?}");
        }
        let tuval = m.deps.iter().find(|d| d.name == "tuval").unwrap();
        assert!(tuval.optional && tuval.path.as_deref() == Some("../tuval"));
        assert!(m.deps.iter().find(|d| d.name == "serde").unwrap().workspace);
        assert_eq!(
            m.deps.iter().find(|d| d.name == "tempfile").unwrap().kind,
            DepKind::Dev
        );
    }

    #[test]
    fn specs_round_trip() {
        let m = Manifest::parse(SAMPLE).unwrap();
        assert_eq!(Manifest::from_specs(&m.to_specs()), m);
    }

    #[test]
    fn workspace_root_keeps_path_dependencies() {
        let m = Manifest::parse(
            "[workspace]\nmembers=[\"a\"]\n[workspace.dependencies]\nfihrist-core = { path = \"crates/fihrist-core\" }\nserde = \"1\"\n",
        )
        .unwrap();
        assert!(m.is_workspace && m.package.is_none());
        assert_eq!(
            m.ws_deps,
            [(
                "fihrist-core".to_string(),
                "crates/fihrist-core".to_string()
            )]
        );
    }

    #[test]
    fn invalid_toml_is_none() {
        assert!(Manifest::parse("[package\nname=").is_none());
    }
}
