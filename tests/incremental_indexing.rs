//! Integration test: Incremental codebase scanning and hash index change detection.

use graft_model::CodeGraph;
use graft_parser::{CodeExtractor, HashIndex};
use std::fs;

#[test]
fn test_incremental_indexing_cycle() {
    let temp_dir =
        std::env::temp_dir().join(format!("archify_graft_inc_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    let file_a = temp_dir.join("service.rs");
    fs::write(
        &file_a,
        "pub struct ServiceWorker;\nimpl ServiceWorker {\n    pub fn run() {}\n}\n",
    )
    .expect("write file_a failed");

    let mut graph = CodeGraph::new();
    let mut hash_index = HashIndex::new();

    // 1. Initial indexing
    let changed_1 =
        CodeExtractor::index_directory_incremental(&temp_dir, &mut graph, &mut hash_index)
            .expect("initial index failed");
    assert_eq!(changed_1, 1, "Initial index must process 1 file");
    assert_eq!(graph.nodes.len(), 3, "Must have File + Struct + Method");

    // 2. Unchanged indexing -> zero files reprocessed
    let changed_2 =
        CodeExtractor::index_directory_incremental(&temp_dir, &mut graph, &mut hash_index)
            .expect("second index failed");
    assert_eq!(changed_2, 0, "Unchanged index must reprocess 0 files");
    assert_eq!(graph.nodes.len(), 3, "Graph nodes must be preserved");

    // 3. Add second file
    let file_b = temp_dir.join("api.rs");
    fs::write(&file_b, "pub fn handler() {}\n").expect("write file_b failed");

    let changed_3 =
        CodeExtractor::index_directory_incremental(&temp_dir, &mut graph, &mut hash_index)
            .expect("third index failed");
    assert_eq!(changed_3, 1, "Only the new file should be processed");
    assert_eq!(graph.nodes.len(), 5, "Should have 3 original + 2 new nodes");

    // 4. Modify existing file
    fs::write(
        &file_a,
        "pub struct ServiceWorker;\nimpl ServiceWorker {\n    pub fn run() {}\n    pub fn stop() {}\n}\n",
    )
    .expect("update file_a failed");

    let changed_4 =
        CodeExtractor::index_directory_incremental(&temp_dir, &mut graph, &mut hash_index)
            .expect("fourth index failed");
    assert_eq!(changed_4, 1, "Only modified file_a should be re-parsed");
    assert_eq!(
        graph.nodes.len(),
        6,
        "Nodes should reflect added stop() method"
    );

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}
