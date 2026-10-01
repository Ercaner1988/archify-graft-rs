//! graft-parser: Direct DMA unbuffered file reader, parallel extractor, incremental hash
//! tracker, and the graph-wide resolvers (imports, crate dependencies, calls, data flow).

pub mod crate_graph;
pub mod extractor;
pub mod flow;
pub mod imports;
pub mod incremental;
pub mod resolver;
mod rust_file;

pub use extractor::AstExtractor;
pub use incremental::HashIndex;
pub use resolver::CallResolver;

use graft_model::CodeGraph;
use lowlevel_sys::DirectReader;
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub struct CodeExtractor;

/// Folders that hold tool output, vendored code or build products, never the project's
/// own source: walking them turned a 1500-file Python runtime into "the architecture".
fn skip_dir(name: &str, parent: &str) -> bool {
    name.starts_with('.')
        || name.ends_with("-out")
        || name.starts_with("pythoncore-")
        || matches!(
            name,
            "target"
                | "node_modules"
                | "dist"
                | "build"
                | "vendor"
                | "third_party"
                | "out"
                | "coverage"
                | "__pycache__"
                | "site-packages"
                | "venv"
        )
        || (parent == "docs" && name == "diyagramlar")
}

impl CodeExtractor {
    /// Full parallel scan of a codebase directory
    pub fn index_directory<P: AsRef<Path>>(root: P) -> anyhow::Result<CodeGraph> {
        Ok(Self::index_directory_with_hashes(root)?.0)
    }

    /// Full parallel scan that also records every file's content hash from the same
    /// read, so the incremental index needs no second walk and re-read of the tree.
    pub fn index_directory_with_hashes<P: AsRef<Path>>(
        root: P,
    ) -> anyhow::Result<(CodeGraph, HashIndex)> {
        let root_path = root.as_ref().to_path_buf();
        let mut file_paths = Vec::new();
        Self::collect_files(&root_path, &mut file_paths)?;

        let extracted: Vec<(CodeGraph, String, u64)> = file_paths
            .par_iter()
            .filter_map(|path| {
                let bytes = DirectReader::read_file(path).ok()?;
                let path_str = path.to_string_lossy().to_string();
                let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                let graph = AstExtractor::extract_content(&path_str, &file_name, &bytes);
                Some((graph, path_str, HashIndex::hash_bytes(&bytes)))
            })
            .collect();

        let mut master = CodeGraph::new();
        let mut hashes = HashIndex::new();
        for (sub, path_str, hash) in extracted {
            master.absorb(sub);
            hashes.update(path_str, hash);
        }

        Self::resolve_inter_symbol_calls(&mut master);
        master.rebuild_index();
        Ok((master, hashes))
    }

    /// Incremental scan: skips unchanged files by verifying FNV-1a content hashes
    pub fn index_directory_incremental<P: AsRef<Path>>(
        root: P,
        graph: &mut CodeGraph,
        hash_index: &mut HashIndex,
    ) -> anyhow::Result<usize> {
        let root_path = root.as_ref().to_path_buf();
        let mut file_paths = Vec::new();
        Self::collect_files(&root_path, &mut file_paths)?;

        let mut changed_count = 0;
        // Every listed file counts as present, readable or not (a transient read error
        // must not look like a deletion).
        let current_paths: HashSet<String> = file_paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        // Reading dominates a no-change run (8.9k files: 4.2 s sequential, well under a
        // second in parallel), so hash in parallel and keep the bytes of changed files only.
        let scanned: Vec<(String, u64, Option<Vec<u8>>)> = file_paths
            .par_iter()
            .filter_map(|path| {
                let bytes = DirectReader::read_file(path).ok()?;
                let path_str = path.to_string_lossy().to_string();
                let hash = HashIndex::hash_bytes(&bytes);
                let changed = !hash_index.is_unchanged(&path_str, hash);
                Some((path_str, hash, changed.then_some(bytes)))
            })
            .collect();

        for (path_str, content_hash, bytes) in scanned {
            if let Some(bytes) = bytes {
                let path = Path::new(&path_str);
                changed_count += 1;
                hash_index.update(path_str.clone(), content_hash);
                graph.forget_file(&path_str);
                let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                graph.absorb(AstExtractor::extract_content(&path_str, &file_name, &bytes));
            }
        }

        let all_known: Vec<String> = hash_index.hashes.keys().cloned().collect();
        for known in all_known {
            if !current_paths.contains(&known) {
                hash_index.remove(&known);
                graph.forget_file(&known);
            }
        }

        if changed_count > 0 {
            Self::resolve_inter_symbol_calls(graph);
            graph.rebuild_index();
        }

        Ok(changed_count)
    }

    pub fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) -> anyhow::Result<()> {
        let parent = dir.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

                // The directory listing already carries the entry type; `path.is_dir()`
                // is one more stat per entry (5 s of a 6 s walk on a 27k-entry tree).
                // Symlinks and unreadable types still take the `is_dir` path.
                let is_dir = match entry.file_type() {
                    Ok(t) if !t.is_symlink() => t.is_dir(),
                    _ => path.is_dir(),
                };
                if is_dir {
                    if !skip_dir(file_name, parent) {
                        Self::collect_files(&path, files)?;
                    }
                } else if !file_name.starts_with('.') && Self::is_supported_file(&path) {
                    files.push(path);
                }
            }
        }
        Ok(())
    }

    /// Source files plus `Cargo.toml` (crates and their dependencies). `.json` is data,
    /// not architecture: it used to make 89 % of one repository's "files".
    fn is_supported_file(path: &Path) -> bool {
        path.file_name().and_then(|s| s.to_str()) == Some("Cargo.toml")
            || matches!(
                path.extension().and_then(|s| s.to_str()),
                Some(
                    "rs" | "ts"
                        | "tsx"
                        | "js"
                        | "jsx"
                        // ESM/CJS explicit-extension variants (`"type": "module"`
                        // packages, TS's own ESM/CJS-explicit sources): without these a
                        // Node project built on `.mjs` indexes as zero source files.
                        | "mjs"
                        | "cjs"
                        | "mts"
                        | "cts"
                        | "py"
                        | "go"
                        | "java"
                        | "c"
                        | "cpp"
                        | "h"
                        | "hpp"
                )
            )
    }

    pub fn extract_file(path: &Path) -> anyhow::Result<CodeGraph> {
        let bytes = DirectReader::read_file(path)?;
        let path_str = path.to_string_lossy().to_string();
        let file_name = path.file_name().unwrap_or_default().to_string_lossy();
        Ok(AstExtractor::extract_content(&path_str, &file_name, &bytes))
    }

    /// Everything that needs the whole file set: imports, crate dependencies, calls,
    /// data-file flows. Each step rebuilds its own edges from scratch.
    pub fn resolve_inter_symbol_calls(graph: &mut CodeGraph) {
        imports::resolve(graph);
        crate_graph::link(graph);
        CallResolver::resolve_calls(graph);
        flow::link(graph);
        graph.rebuild_index();
    }

    pub fn is_fresh<P: AsRef<Path>>(root: P, cache_file: P) -> bool {
        let cache_path = cache_file.as_ref();
        if !cache_path.exists() {
            return false;
        }

        let cache_mtime = match cache_path.metadata().and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(_) => return false,
        };

        let mut file_paths = Vec::new();
        if Self::collect_files(root.as_ref(), &mut file_paths).is_err() {
            return false;
        }

        for path in file_paths {
            if let Ok(m) = path.metadata().and_then(|m| m.modified()) {
                if m > cache_mtime {
                    return false;
                }
            }
        }

        true
    }

    /// `is_fresh`'in ayrıntılı hali — yalnız evet/hayır değil, CI'da tanı için
    /// HANGİ dosyaların cache'ten daha yeni olduğunu da döner. `is_fresh` hiç
    /// çağrılmıyordu (grep ile doğrulandı) — `check` komutu/`graft_check_freshness`
    /// bunun ilk gerçek tüketicisi.
    pub fn freshness_report<P: AsRef<Path>>(
        root: P,
        cache_file: P,
    ) -> anyhow::Result<(bool, Vec<String>)> {
        let cache_path = cache_file.as_ref();
        if !cache_path.exists() {
            return Ok((false, vec!["<cache hiç üretilmemiş>".to_string()]));
        }
        let cache_mtime = cache_path.metadata()?.modified()?;

        let mut file_paths = Vec::new();
        Self::collect_files(root.as_ref(), &mut file_paths)?;

        let mut bayat: Vec<String> = file_paths
            .iter()
            .filter_map(|p| {
                let m = p.metadata().ok()?.modified().ok()?;
                (m > cache_mtime).then(|| p.to_string_lossy().replace('\\', "/"))
            })
            .collect();
        bayat.sort();

        Ok((bayat.is_empty(), bayat))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_model::{EdgeRelation, NodeKind};

    #[test]
    fn test_call_resolution_links_a_unique_function_of_the_same_crate() {
        let dir = std::env::temp_dir().join(format!("graft-calls-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("src").join("main.rs"),
            "fn run_job() { calculate_total(); }\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("src").join("calc.rs"),
            "pub fn calculate_total() -> i32 { 42 }\n",
        )
        .unwrap();
        let graph = CodeExtractor::index_directory(&dir).unwrap();
        let calls: Vec<_> = graph
            .edges
            .iter()
            .filter(|e| e.relation == EdgeRelation::Calls)
            .collect();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].source.ends_with("main.rs:run_job"));
        assert!(calls[0].target.ends_with("calc.rs:calculate_total"));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Node's own `"type": "module"` convention (and TS's `.mts`/`.cts`) means a whole
    /// real-world JS/TS project can be 100% `.mjs`/`.cjs` files with not a single bare
    /// `.js`.
    #[test]
    fn mjs_cjs_mts_cts_files_are_indexed_not_skipped() {
        let dir = std::env::temp_dir().join(format!("graft-esm-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.mjs"), "export function alpha() {}\n").unwrap();
        std::fs::write(
            dir.join("b.cjs"),
            "function beta() {}\nmodule.exports = beta;\n",
        )
        .unwrap();
        std::fs::write(dir.join("c.mts"), "export function gamma() {}\n").unwrap();

        let (graph, hashes) = CodeExtractor::index_directory_with_hashes(&dir).unwrap();

        assert_eq!(
            hashes.hashes.len(),
            3,
            "all three ESM/TS variants get walked"
        );
        assert!(graph.nodes.iter().any(|n| n.name == "alpha"));
        assert!(graph.nodes.iter().any(|n| n.name == "beta"));
        assert!(graph.nodes.iter().any(|n| n.name == "gamma"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn json_generated_and_vendored_folders_are_not_indexed() {
        let dir = std::env::temp_dir().join(format!("graft-skip-{}", std::process::id()));
        for d in [
            "src",
            "graphify-rs-out/cache",
            "Python/pythoncore-3.14/Lib",
            "vendor/x",
            "docs/diyagramlar",
        ] {
            std::fs::create_dir_all(dir.join(d)).unwrap();
            std::fs::write(dir.join(d).join("x.rs"), "fn hidden() {}\n").unwrap();
            std::fs::write(dir.join(d).join("d.json"), "{}").unwrap();
        }
        std::fs::write(dir.join("src").join("lib.rs"), "pub fn shown() {}\n").unwrap();
        let (_, hashes) = CodeExtractor::index_directory_with_hashes(&dir).unwrap();
        let mut files: Vec<&String> = hashes.hashes.keys().collect();
        files.sort();
        assert_eq!(files.len(), 2, "{files:?}");
        assert!(files.iter().all(|f| f.contains("src")));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_cargo_workspace_becomes_crate_nodes_with_depends_on_edges() {
        let dir = std::env::temp_dir().join(format!("graft-cargo-{}", std::process::id()));
        for (krate, deps) in [("core", ""), ("app", "core = { path = \"../core\" }\n")] {
            std::fs::create_dir_all(dir.join(krate).join("src")).unwrap();
            std::fs::write(
                dir.join(krate).join("Cargo.toml"),
                format!("[package]\nname = \"{krate}\"\n[dependencies]\n{deps}"),
            )
            .unwrap();
        }
        std::fs::write(dir.join("core/src/lib.rs"), "pub struct Thing;\n").unwrap();
        std::fs::write(
            dir.join("app/src/main.rs"),
            "use core::{Thing};\nfn main() {}\n",
        )
        .unwrap();
        let graph = CodeExtractor::index_directory(&dir).unwrap();
        assert_eq!(
            graph
                .nodes
                .iter()
                .filter(|n| n.kind == NodeKind::Crate)
                .count(),
            2
        );
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|e| e.relation == EdgeRelation::DependsOn)
                .count(),
            1
        );
        // `use core::{Thing}` reaches the crate root file through the brace group.
        assert!(graph
            .edges
            .iter()
            .any(|e| e.relation == EdgeRelation::Imports && e.target.ends_with("lib.rs")));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_index_directory_with_hashes_hashes_the_bytes_the_graph_came_from() {
        let dir = std::env::temp_dir().join(format!("graft-hashes-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("a.rs"), "pub fn alpha() {}\n").unwrap();
        std::fs::write(dir.join("sub").join("b.rs"), "pub fn beta() { alpha(); }\n").unwrap();
        std::fs::write(dir.join("notes.txt"), "not indexed").unwrap();

        let (graph, hashes) = CodeExtractor::index_directory_with_hashes(&dir).unwrap();

        assert_eq!(hashes.hashes.len(), 2, "one hash per indexed file");
        for (path, hash) in &hashes.hashes {
            assert_eq!(*hash, HashIndex::hash_bytes(&std::fs::read(path).unwrap()));
        }
        // the subdirectory is walked (entry type comes from the listing)
        assert!(hashes.hashes.keys().any(|p| p.ends_with("b.rs")));
        assert!(graph.nodes.iter().any(|n| n.name == "beta"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_incremental_reindexes_only_changed_files_and_forgets_deleted_ones() {
        let dir = std::env::temp_dir().join(format!("graft-incr-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.rs"), "pub fn alpha() {}\n").unwrap();
        std::fs::write(dir.join("b.rs"), "pub fn beta() {}\n").unwrap();
        let (mut graph, mut hashes) = CodeExtractor::index_directory_with_hashes(&dir).unwrap();

        let unchanged =
            CodeExtractor::index_directory_incremental(&dir, &mut graph, &mut hashes).unwrap();
        assert_eq!(unchanged, 0, "nothing edited, nothing re-read");

        std::fs::write(dir.join("a.rs"), "pub fn alpha2() {}\n").unwrap();
        std::fs::remove_file(dir.join("b.rs")).unwrap();
        let changed =
            CodeExtractor::index_directory_incremental(&dir, &mut graph, &mut hashes).unwrap();

        assert_eq!(changed, 1);
        assert!(graph.nodes.iter().any(|n| n.name == "alpha2"));
        assert!(!graph
            .nodes
            .iter()
            .any(|n| n.name == "alpha" || n.name == "beta"));
        assert_eq!(
            hashes.hashes.len(),
            1,
            "the deleted file left the hash index"
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
