//! Benchmarks for the indexing pipeline: content hashing, AST extraction and
//! inter-symbol call resolution (`graft-parser`).

#[path = "fixtures/mod.rs"]
mod fixtures;

use graft_parser::{AstExtractor, CallResolver, HashIndex};

fn main() {
    divan::main();
}

/// FNV-1a content hashing, run on every file of an incremental index pass.
#[divan::bench(args = [4, 64, 512])]
fn fnv1a_hash(bencher: divan::Bencher, kib: usize) {
    let bytes = fixtures::file_bytes(kib);
    bencher.bench(|| HashIndex::hash_bytes(divan::black_box(&bytes)));
}

/// Line-oriented AST extraction of a single file into a `CodeGraph`.
#[divan::bench(args = [4, 32])]
fn extract_file(bencher: divan::Bencher, items: usize) {
    let source = fixtures::source_file(0, items);
    let bytes = source.as_bytes();
    bencher.bench(|| {
        AstExtractor::extract_content(
            divan::black_box("crates/synthetic/src/module_0.rs"),
            divan::black_box("module_0.rs"),
            divan::black_box(bytes),
        )
    });
}

/// Extraction + merge of a whole synthetic workspace (24 files).
#[divan::bench]
fn extract_workspace(bencher: divan::Bencher) {
    bencher.bench(|| fixtures::code_graph(divan::black_box(24), divan::black_box(4)));
}

/// Call resolution links every caller to the symbols it references; this is the
/// quadratic stage of indexing and the most interesting one to track.
#[divan::bench(args = [4, 12])]
fn resolve_calls(bencher: divan::Bencher, files: usize) {
    let graph = fixtures::code_graph(files, 4);
    bencher.with_inputs(|| graph.clone()).bench_values(|mut g| {
        CallResolver::resolve_calls(&mut g);
        g
    });
}

/// Index rebuild: the id -> node offset map recomputed after a graph mutation.
#[divan::bench]
fn rebuild_node_index(bencher: divan::Bencher) {
    let graph = fixtures::code_graph(16, 4);
    bencher.with_inputs(|| graph.clone()).bench_values(|mut g| {
        g.rebuild_index();
        g
    });
}
