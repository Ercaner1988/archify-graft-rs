//! Benchmarks for BM25 indexing/scoring and personalized GraphRank
//! (`graft-search`).

#[path = "fixtures/mod.rs"]
mod fixtures;

use graft_model::CodeGraph;
use graft_search::{Bm25Index, GraphRank};

fn main() {
    divan::main();
}

fn build_index(graph: &CodeGraph) -> Bm25Index {
    let mut index = Bm25Index::new();
    for node in &graph.nodes {
        if !node.search_body.is_empty() {
            index.add_document(node.id.clone(), &node.search_body);
        }
    }
    index
}

/// Whole-file BM25 index construction (tokenization + term/document frequencies).
#[divan::bench(args = [4, 12])]
fn bm25_build(bencher: divan::Bencher, files: usize) {
    let graph = fixtures::code_graph(files, 4);
    bencher.bench(|| build_index(divan::black_box(&graph)));
}

/// BM25 scoring of a multilingual query over the full document set.
#[divan::bench(args = ["direct dma reader", "ayrıştırıcı düğüm", "المُحلِّل البحث"])]
fn bm25_search(bencher: divan::Bencher, query: &str) {
    let graph = fixtures::code_graph(12, 4);
    let index = build_index(&graph);
    bencher.bench(|| index.search(divan::black_box(query), 10));
}

/// Personalized PageRank propagation over the AST graph.
#[divan::bench(args = [8, 20])]
fn graphrank(bencher: divan::Bencher, iterations: usize) {
    let graph = fixtures::code_graph(6, 3);
    let seeds = fixtures::seeds(&graph, 4);
    bencher.bench(|| {
        GraphRank::compute(
            divan::black_box(&graph),
            divan::black_box(&seeds),
            0.15,
            iterations,
        )
    });
}

/// End-to-end ask pipeline: BM25 retrieval feeding GraphRank seeds.
#[divan::bench]
fn bm25_then_graphrank(bencher: divan::Bencher) {
    let graph = fixtures::code_graph(6, 3);
    let index = build_index(&graph);
    bencher.bench(|| {
        let hits = index.search(divan::black_box("parse ast node render"), 8);
        GraphRank::compute(&graph, &hits, 0.15, 10)
    });
}
