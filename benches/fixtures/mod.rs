//! Deterministic synthetic fixtures shared by the CodSpeed benchmark targets.
//!
//! Every generator is pure and seed-free so that benchmark inputs stay
//! byte-for-byte identical across runs, which is a prerequisite for stable
//! measurements.

#![allow(dead_code)]

use graft_model::CodeGraph;
use graft_parser::AstExtractor;

/// English/Turkish/Arabic corpora used to exercise the trilingual tokenizer.
pub const ENGLISH: &str = "The parallel fileScanner streams sectorAligned blocks through the \
directDmaReader, then the AstExtractor emits NodeV1 records into the CodeGraph; \
the BM25 indexer scores whole files while GraphRank propagates personalized ranks.";

pub const TURKISH: &str = "Işık hızında çalışan ayrıştırıcı, İstanbul'daki düğümleri \
sıralarken önbelleği atlayıp doğrudan DMA ile okuma yapar; İŞLEM birimi \
çağrı izlerini şıkır şıkır çözümler ve şemayı yeniden üretir.";

pub const ARABIC: &str = "يقوم المُحلِّل اللغوي بتفكيك الرموز العَرَبِيَّة إلى وحدات \
أصغر، ثم يبني فهرس البحث عن المكوّنات المعمارية أحمد إبراهيم آدم، \
ويُصدر مخطط البنية بتنسيق SVG مع دعم الاتجاه من اليمين إلى اليسار.";

/// Returns a corpus for `lang`, repeated `repeat` times.
pub fn corpus(lang: &str, repeat: usize) -> String {
    let base = match lang {
        "turkish" => TURKISH,
        "arabic" => ARABIC,
        _ => ENGLISH,
    };

    let mut out = String::with_capacity(base.len() * repeat + repeat);
    for _ in 0..repeat {
        out.push_str(base);
        out.push('\n');
    }
    out
}

/// Generates a synthetic source file containing structs, enums, traits and
/// functions, plus trilingual comments so tokenizer-backed indexing is also
/// exercised realistically.
pub fn source_file(file_idx: usize, items: usize) -> String {
    let mut src = String::with_capacity(items * 512);
    src.push_str("//! Synthetic module used by the archify-graft benchmark fixtures.\n");
    src.push_str("//! Ayrıştırıcı düğümleri / وحدات المُحلِّل / parser nodes.\n");
    src.push_str("use crate::storage_repo::GraphStorage;\n\n");

    for item in 0..items {
        let id = file_idx * items + item;
        src.push_str(&format!(
            "pub struct ServiceHandler{id} {{\n    pub storage_repo: usize,\n    pub render_view: String,\n}}\n\n"
        ));
        src.push_str(&format!(
            "pub enum RenderState{id} {{\n    Idle,\n    Streaming,\n    Flushed,\n}}\n\n"
        ));
        src.push_str(&format!(
            "pub trait EventBusPort{id} {{\n    fn dispatch_event(&self, payload: &str) -> usize;\n}}\n\n"
        ));
        src.push_str(&format!(
            "pub fn parse_ast_node_{id}(source: &str) -> usize {{ let t = normalize_query_text_{prev}(source); render_neon_svg_{next}(t) }}\n",
            prev = id.saturating_sub(1),
            next = (id + 1) % (items.max(1) * 4),
        ));
        src.push_str(&format!(
            "pub fn normalize_query_text_{id}(source: &str) -> usize {{ source.len() + hash_file_bytes_{id}(source) }}\n"
        ));
        src.push_str(&format!(
            "pub fn render_neon_svg_{id}(width: usize) -> usize {{ GraphStorage::save(width) + parse_ast_node_{id}(\"svg\") }}\n"
        ));
        src.push_str(&format!(
            "pub fn hash_file_bytes_{id}(source: &str) -> usize {{ source.bytes().map(|b| b as usize).sum() }}\n\n"
        ));
    }

    src
}

/// Builds a merged [`CodeGraph`] out of `files` synthetic source files.
pub fn code_graph(files: usize, items_per_file: usize) -> CodeGraph {
    let mut graph = CodeGraph::new();

    for file_idx in 0..files {
        let path = format!("crates/synthetic/src/module_{file_idx}.rs");
        let name = format!("module_{file_idx}.rs");
        let source = source_file(file_idx, items_per_file);
        let sub = AstExtractor::extract_content(&path, &name, source.as_bytes());

        for node in sub.nodes {
            graph.add_node(node);
        }
        for edge in sub.edges {
            graph.add_edge(edge);
        }
    }

    graph.rebuild_index();
    graph
}

/// Same as [`code_graph`], with one extra file and one renamed component so
/// delta diffing has Added / Modified / Removed work to do.
pub fn mutated_code_graph(files: usize, items_per_file: usize) -> CodeGraph {
    let mut graph = code_graph(files.saturating_sub(1).max(1), items_per_file);

    let path = format!("crates/synthetic/src/module_{files}_new.rs");
    let name = format!("module_{files}_new.rs");
    let source = source_file(files + 1, items_per_file);
    let sub = AstExtractor::extract_content(&path, &name, source.as_bytes());
    for node in sub.nodes {
        graph.add_node(node);
    }
    for edge in sub.edges {
        graph.add_edge(edge);
    }

    if let Some(first) = graph.nodes.first_mut() {
        first.name = format!("{}-renamed", first.name);
    }

    graph.rebuild_index();
    graph
}

/// A deterministic byte payload, used for content hashing benchmarks.
pub fn file_bytes(kib: usize) -> Vec<u8> {
    let chunk = source_file(0, 4).into_bytes();
    let target = kib * 1024;
    let mut out = Vec::with_capacity(target);
    while out.len() < target {
        let take = (target - out.len()).min(chunk.len());
        out.extend_from_slice(&chunk[..take]);
    }
    out
}

/// PageRank seeds taken from the first nodes of the graph.
pub fn seeds(graph: &CodeGraph, count: usize) -> Vec<(String, f32)> {
    graph
        .nodes
        .iter()
        .take(count)
        .enumerate()
        .map(|(i, node)| (node.id.clone(), 1.0 / (i as f32 + 1.0)))
        .collect()
}
