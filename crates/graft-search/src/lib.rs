//! graft-search: Whole-file BM25 index, GraphRank (PageRank), and memory-mapped storage.

mod report;
pub use report::{build_repo_map, file_skeleton, grep_graph};

use anyhow::Context;
use graft_i18n::TrilingualTokenizer;
use graft_model::CodeGraph;
use memmap2::Mmap;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Bm25Document {
    pub doc_id: String,
    pub length: usize,
    pub term_freqs: HashMap<String, u32>,
}

#[derive(Debug, Clone, Default)]
pub struct Bm25Index {
    pub docs: Vec<Bm25Document>,
    pub doc_freqs: HashMap<String, u32>,
    pub total_docs: usize,
    pub avg_doc_len: f32,
}

impl Bm25Index {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_document(&mut self, doc_id: String, text: &str) {
        let tokens = TrilingualTokenizer::tokenize(text);
        let length = tokens.len();
        let mut term_freqs: HashMap<String, u32> = HashMap::new();

        for token in tokens {
            *term_freqs.entry(token).or_insert(0) += 1;
        }

        for term in term_freqs.keys() {
            *self.doc_freqs.entry(term.clone()).or_insert(0) += 1;
        }

        self.docs.push(Bm25Document {
            doc_id,
            length,
            term_freqs,
        });

        self.total_docs = self.docs.len();
        let total_len: usize = self.docs.iter().map(|d| d.length).sum();
        self.avg_doc_len = if self.total_docs > 0 {
            total_len as f32 / self.total_docs as f32
        } else {
            1.0
        };
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<(String, f32)> {
        let query_tokens = TrilingualTokenizer::tokenize(query);
        if query_tokens.is_empty() || self.total_docs == 0 {
            return Vec::new();
        }

        let k1: f32 = 1.2;
        let b: f32 = 0.75;
        let mut scores: Vec<(String, f32)> = Vec::with_capacity(self.docs.len());

        for doc in &self.docs {
            let mut score = 0.0;
            let doc_len_ratio = doc.length as f32 / self.avg_doc_len.max(1.0);

            for q_term in &query_tokens {
                if let Some(&tf) = doc.term_freqs.get(q_term) {
                    let df = *self.doc_freqs.get(q_term).unwrap_or(&1) as f32;
                    let n = self.total_docs as f32;

                    let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
                    let tf_norm =
                        (tf as f32 * (k1 + 1.0)) / (tf as f32 + k1 * (1.0 - b + b * doc_len_ratio));
                    score += idf * tf_norm;
                }
            }

            if score > 0.001 {
                scores.push((doc.doc_id.clone(), score));
            }
        }

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(limit);
        scores
    }
}

pub struct GraphRank;

impl GraphRank {
    pub fn compute(
        graph: &CodeGraph,
        seeds: &[(String, f32)],
        alpha: f32,
        iterations: usize,
    ) -> Vec<(String, f32)> {
        let n = graph.nodes.len();
        if n == 0 || seeds.is_empty() {
            return Vec::new();
        }

        let mut id_to_idx: HashMap<&str, usize> = HashMap::with_capacity(n);
        for (i, node) in graph.nodes.iter().enumerate() {
            id_to_idx.insert(&node.id, i);
        }

        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
        for edge in &graph.edges {
            if let (Some(&src), Some(&tgt)) = (
                id_to_idx.get(edge.source.as_str()),
                id_to_idx.get(edge.target.as_str()),
            ) {
                adj[src].push(tgt);
            }
        }

        let mut p = vec![0.0f32; n];
        let mut seed_sum = 0.0f32;
        for (seed_id, weight) in seeds {
            if let Some(&idx) = id_to_idx.get(seed_id.as_str()) {
                p[idx] += *weight;
                seed_sum += *weight;
            }
        }

        if seed_sum > 0.0 {
            for v in &mut p {
                *v /= seed_sum;
            }
        } else {
            p.fill(1.0 / n as f32);
        }

        let mut rank = p.clone();
        let mut next_rank = vec![0.0f32; n];

        for _ in 0..iterations {
            next_rank.fill(0.0);

            for i in 0..n {
                let out_degree = adj[i].len();
                if out_degree > 0 {
                    let share = rank[i] / out_degree as f32;
                    for &target in &adj[i] {
                        next_rank[target] += share;
                    }
                } else {
                    for j in 0..n {
                        next_rank[j] += rank[i] * p[j];
                    }
                }
            }

            for i in 0..n {
                rank[i] = (1.0 - alpha) * next_rank[i] + alpha * p[i];
            }
        }

        let mut results: Vec<(String, f32)> = graph
            .nodes
            .iter()
            .enumerate()
            .map(|(i, node)| (node.id.clone(), rank[i]))
            .filter(|(_, r)| *r > 0.0001)
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }
}

pub struct GraphStorage;

impl GraphStorage {
    pub fn save<P: AsRef<Path>>(graph: &CodeGraph, path: P) -> anyhow::Result<()> {
        let bytes = bincode::serde::encode_to_vec(graph, bincode::config::standard())?;
        let mut file = File::create(path)?;
        file.write_all(&bytes)?;
        file.flush()?;
        Ok(())
    }

    pub fn load_mmap<P: AsRef<Path>>(path: P) -> anyhow::Result<CodeGraph> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        let (mut graph, _): (CodeGraph, usize) =
            bincode::serde::decode_from_slice(&mmap, bincode::config::standard())
                .context("graph cache is unreadable (older layout?); run `index` again")?;
        graph.rebuild_index();
        Ok(graph)
    }

    /// Shared by the CLI's `index` command and the MCP `graft_index` tool: reuse
    /// the on-disk cache via an incremental hash-based rescan when one exists,
    /// otherwise a full scan, then persist the result. Without this, an MCP
    /// caller (a fresh archify-graft process per Claude Code session) did a full
    /// re-parse in RAM every session and never touched the CLI's cache — cold
    /// every time, and the CLI cache never saw MCP-driven updates either.
    pub fn index_or_refresh<P: AsRef<Path>>(path: P, cache: P) -> anyhow::Result<CodeGraph> {
        let path = path.as_ref();
        let cache = cache.as_ref();
        let hash_file = cache.with_extension("hashes.bin");

        let previous = if cache.exists() && hash_file.exists() {
            Self::load_mmap(cache)
                .and_then(|g| Ok((g, graft_parser::HashIndex::load_from_file(&hash_file)?)))
                .ok()
        } else {
            None
        };

        let (graph, dirty) = if let Some((mut prev_graph, mut hash_index)) = previous {
            let known_before = hash_index.hashes.len();
            let changed = graft_parser::CodeExtractor::index_directory_incremental(
                path,
                &mut prev_graph,
                &mut hash_index,
            )?;
            let dirty = changed > 0 || hash_index.hashes.len() != known_before;
            if dirty {
                let _ = hash_index.save_to_file(&hash_file);
            }
            (prev_graph, dirty)
        } else {
            let (graph, hash_index) =
                graft_parser::CodeExtractor::index_directory_with_hashes(path)?;
            if let Some(parent) = hash_file.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = hash_index.save_to_file(&hash_file);
            (graph, true)
        };

        if dirty {
            if let Some(parent) = cache.parent() {
                std::fs::create_dir_all(parent)?;
            }
            Self::save(&graph, cache)?;
        }

        Ok(graph)
    }
}
