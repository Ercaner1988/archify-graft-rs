use archify_bridge::GraftToArchifyBridge;
use archify_render::{wrap_html, SvgRenderer};
use clap::{Parser, Subcommand};
use graft_mcp::McpServer;
use graft_parser::CodeExtractor;
use graft_search::{build_repo_map, file_skeleton, grep_graph, Bm25Index, GraphRank, GraphStorage};
use lowlevel_sys::{cache_line_size, HardwareTimer};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// `graft/.graph/wiring.bin` başlığı — pasli-beyin'in `kopru.rs::graft_koprusu`
/// köprüsünün okuduğu KÜÇÜK, sürümlü sözleşme (`graft_model::WiringMeta`, bincode
/// `config::standard()`). JSON değil: alan SIRASI biçimin parçasıdır ve pasli-beyin
/// aynı yapıyı aynı sırayla yansıtır. Düğüm/kenar dizisinin tamamı yazılmaz; köprü
/// onlara dokunmuyor. Eski Node.js `@nanonets/graft`'ın yerini alan en ucuz adım.
fn write_wiring_meta(indexed_path: &str, graph: &graft_model::CodeGraph) -> anyhow::Result<()> {
    let languages: BTreeSet<&'static str> = graph
        .nodes
        .iter()
        .filter_map(|n| Path::new(&n.path).extension()?.to_str())
        .filter_map(extension_to_language)
        .collect();

    let wiring = graft_model::WiringMeta {
        version: 1,
        node_count: graph.nodes.len() as u64,
        edge_count: graph.edges.len() as u64,
        languages: languages.into_iter().map(str::to_string).collect(),
    };

    let dir = Path::new(indexed_path).join("graft").join(".graph");
    fs::create_dir_all(&dir)?;
    fs::write(
        dir.join("wiring.bin"),
        bincode::serde::encode_to_vec(&wiring, bincode::config::standard())?,
    )?;
    Ok(())
}

/// `graft-parser`'ın taradığı uzantı kümesiyle AYNI (bkz. `is_supported_extension`) —
/// biri değişirse öbürü bayatlar, o yüzden burada da elle listelenir.
fn extension_to_language(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "rs" => "rust",
        "ts" | "tsx" | "mts" | "cts" => "typescript",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "py" => "python",
        "go" => "go",
        "java" => "java",
        "c" | "h" => "c",
        "cpp" | "hpp" => "cpp",
        "json" => "json",
        _ => return None,
    })
}

#[derive(Parser, Debug)]
#[command(name = "archify-graft")]
#[command(about = "Unified Pure-Rust Engine: Graft Codebase Intelligence & Archify Architecture Studio", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Analyze and index a codebase with hardware-accelerated direct DMA AST parser
    Index {
        /// Target directory to index
        #[arg(default_value = ".")]
        path: String,
        /// Cache output path
        #[arg(short, long, default_value = ".cache/graft-graph.bin")]
        cache: String,
        /// Do not write `graft/.graph/wiring.bin` into the indexed folder (keeps a
        /// foreign repository untouched, e.g. when only a diagram is wanted)
        #[arg(long)]
        no_wiring: bool,
    },
    /// Query the codebase using trilingual BM25 + PageRank (TR/AR/EN)
    Ask {
        /// Natural language or keyword query
        query: String,
        /// Maximum number of file references to return
        #[arg(short, long, default_value_t = 10)]
        limit: usize,
        /// Cache path to read graph from
        #[arg(short, long, default_value = ".cache/graft-graph.bin")]
        cache: String,
    },
    /// Generate an Archify architecture, sequence, dataflow, or delta SVG diagram from indexed code
    Diagram {
        /// Diagram type: architecture, sequence, dataflow, delta
        #[arg(short = 't', long = "type", default_value = "architecture")]
        diagram_type: String,
        /// Entrypoint function for sequence diagram
        #[arg(long, default_value = "main")]
        entrypoint: String,
        /// Previous cache path for delta comparison
        #[arg(long)]
        compare: Option<String>,
        /// Output path for SVG diagram file
        #[arg(short, long, default_value = "architecture.svg")]
        output: String,
        /// Title for the diagram
        #[arg(long, default_value = "Codebase Architecture Map")]
        title: String,
        /// Locale: tr (Turkish), ar (Arabic), en (English)
        #[arg(short, long, default_value = "tr")]
        locale: String,
        /// Cache path to read graph from
        #[arg(short, long, default_value = ".cache/graft-graph.bin")]
        cache: String,
    },
    /// Token-budgeted repo orientation — directory clusters, per-directory hubs,
    /// and global hotspots from the wiring graph
    Map {
        /// Cache path to read graph from
        #[arg(short, long, default_value = ".cache/graft-graph.bin")]
        cache: String,
        /// Directory rows to print before summarizing the rest
        #[arg(long, default_value_t = 30)]
        max_dirs: usize,
    },
    /// Regex search over indexed files, hits grouped by declaration vs. file body
    Grep {
        /// Regex pattern (Rust `regex` crate syntax)
        pattern: String,
        /// Cache path to read graph from
        #[arg(short, long, default_value = ".cache/graft-graph.bin")]
        cache: String,
        /// Case-insensitive match
        #[arg(short = 'i', long)]
        ignore_case: bool,
    },
    /// Signatures-only view of one file from the wiring graph
    Skeleton {
        /// File path (exact or suffix match against indexed paths)
        file: String,
        /// Cache path to read graph from
        #[arg(short, long, default_value = ".cache/graft-graph.bin")]
        cache: String,
    },
    /// Fail if the index is stale relative to the code (for CI)
    Check {
        /// Directory that was indexed
        #[arg(default_value = ".")]
        path: String,
        /// Cache path to check freshness against
        #[arg(short, long, default_value = ".cache/graft-graph.bin")]
        cache: String,
    },
    /// Start the Model Context Protocol (MCP) JSON-RPC stdio server for AI agents
    Mcp,
    /// Launch the interactive pure-Rust desktop Archify studio window
    #[cfg(feature = "gui")]
    Gui {
        /// Diagram title
        #[arg(short, long, default_value = "Archify Studio")]
        title: String,
        /// Locale: tr (Turkish), ar (Arabic), en (English)
        #[arg(short, long, default_value = "tr")]
        locale: String,
        /// Cache path to read graph from (optional)
        #[arg(short, long, default_value = ".cache/graft-graph.bin")]
        cache: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Index {
            path,
            cache,
            no_wiring,
        } => {
            println!(
                "⚡ [Hardware-Sys] Detected CPU cache line: {} bytes",
                cache_line_size()
            );
            println!(
                "🚀 [Graft-Core] Indexing codebase at '{}' using direct DMA buffers...",
                path
            );

            let hash_file = Path::new(&cache).with_extension("hashes.bin");
            let timer = HardwareTimer::start();

            // An unreadable previous index (older cache layout, half-written file) is
            // not an error: it is rebuilt from scratch below.
            let previous = if Path::new(&cache).exists() && hash_file.exists() {
                GraphStorage::load_mmap(&cache)
                    .and_then(|g| Ok((g, graft_parser::HashIndex::load_from_file(&hash_file)?)))
                    .map_err(|e| println!("⚠️  [Graft-Core] Existing index unusable ({e}); re-indexing from scratch."))
                    .ok()
            } else {
                None
            };

            let (graph, changed, dirty) = if let Some((mut prev_graph, mut hash_index)) = previous {
                println!(
                    "⚡ [Graft-Core] Found existing index. Performing incremental change scan..."
                );
                let known_before = hash_index.hashes.len();
                let changed = CodeExtractor::index_directory_incremental(
                    &path,
                    &mut prev_graph,
                    &mut hash_index,
                )?;
                // Deleted files shrink the hash index without counting as "changed".
                let dirty = changed > 0 || hash_index.hashes.len() != known_before;
                if dirty {
                    let _ = hash_index.save_to_file(&hash_file);
                }
                (prev_graph, changed, dirty)
            } else {
                let (graph, hash_index) = CodeExtractor::index_directory_with_hashes(&path)?;
                if let Some(parent) = hash_file.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = hash_index.save_to_file(&hash_file);
                let count = graph.nodes.len();
                (graph, count, true)
            };
            let elapsed_cycles = timer.elapsed_cycles();

            println!("✅ [Graft-Core] Processed {} files. Index contains {} nodes and {} edges (in {} CPU cycles).", 
                changed, graph.nodes.len(), graph.edges.len(), elapsed_cycles);

            if let Some(parent) = Path::new(&cache).parent() {
                fs::create_dir_all(parent)?;
            }

            // Nothing changed: leave the cache untouched (no rewrite, and two sessions
            // starting together cannot race on it).
            if dirty {
                GraphStorage::save(&graph, &cache)?;
                println!(
                    "💾 [Graft-Core] Saved memory-mapped graph index to '{}'.",
                    cache
                );
            } else {
                println!(
                    "💤 [Graft-Core] Nothing changed; cache '{}' left as is.",
                    cache
                );
            }

            if !no_wiring {
                write_wiring_meta(&path, &graph)?;
            }
        }
        Commands::Ask {
            query,
            limit,
            cache,
        } => {
            if !Path::new(&cache).exists() {
                eprintln!(
                    "❌ Cache not found at '{}'. Run 'archify-graft index' first.",
                    cache
                );
                std::process::exit(1);
            }

            println!(
                "📖 [Graft-Core] Loading memory-mapped graph from '{}'...",
                cache
            );
            let graph = GraphStorage::load_mmap(&cache)?;

            println!("🔍 [Graft-Core] Building trilingual lexical index (TR/AR/EN)...");
            let mut bm25 = Bm25Index::new();
            for node in &graph.nodes {
                let content = format!("{} {} {}", node.name, node.path, node.search_body);
                bm25.add_document(node.id.clone(), &content);
            }

            println!(
                "⚡ [Graft-Core] Searching query '{}' with BM25 + GraphRank...",
                query
            );
            let bm25_results = bm25.search(&query, limit * 3);
            let ranked = GraphRank::compute(&graph, &bm25_results, 0.25, 25);

            println!("\n🏆 [Results: Top {} Matches]:", limit);
            for (idx, (id, score)) in ranked.iter().take(limit).enumerate() {
                println!("  {:2}. [{:.4}] {}", idx + 1, score, id);
            }
        }
        Commands::Diagram {
            diagram_type,
            entrypoint,
            compare,
            output,
            title,
            locale,
            cache,
        } => {
            if !Path::new(&cache).exists() {
                eprintln!(
                    "❌ Cache not found at '{}'. Run 'archify-graft index' first.",
                    cache
                );
                std::process::exit(1);
            }

            println!("📖 [Graft-Core] Loading graph from '{}'...", cache);
            let graph = GraphStorage::load_mmap(&cache)?;

            let svg = match diagram_type.to_lowercase().as_str() {
                "sequence" => {
                    println!(
                        "🎨 [Archify-Core] Compiling Sequence Diagram for entrypoint '{}'...",
                        entrypoint
                    );
                    let seq = GraftToArchifyBridge::compile_sequence(
                        &graph,
                        &entrypoint,
                        &title,
                        &locale,
                    );
                    SvgRenderer::render_sequence(&seq)
                }
                "dataflow" => {
                    println!("🎨 [Archify-Core] Compiling Dataflow Pipeline Diagram...");
                    let df = GraftToArchifyBridge::compile_dataflow(&graph, &title, &locale);
                    SvgRenderer::render_dataflow(&df)
                }
                "delta" => {
                    let prev_cache = compare.unwrap_or_default();
                    let before_graph = if !prev_cache.is_empty() && Path::new(&prev_cache).exists()
                    {
                        println!("📖 [Archify-Core] Loading base graph for delta comparison from '{}'...", prev_cache);
                        GraphStorage::load_mmap(&prev_cache)?
                    } else {
                        println!("ℹ️ [Archify-Core] No valid prior cache given, comparing against empty state.");
                        graft_model::CodeGraph::new()
                    };
                    println!("🎨 [Archify-Core] Computing Architectural Delta Analysis...");
                    let delta = archify_delta::DeltaEngine::compute_graph_delta(
                        &before_graph,
                        &graph,
                        &title,
                        &locale,
                    );
                    SvgRenderer::render_delta(&delta)
                }
                _ => {
                    println!("🎨 [Archify-Core] Compiling Architecture Diagram...");
                    let diagram = GraftToArchifyBridge::compile(&graph, &title, &locale);
                    SvgRenderer::render(&diagram)
                }
            };

            // `.html`/`.htm` gets a self-contained pan/zoom viewer around the same SVG;
            // any other extension (default `.svg`) writes the raw markup as before.
            let is_html = Path::new(&output)
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("html") || e.eq_ignore_ascii_case("htm"));
            let contents = if is_html {
                wrap_html(&svg, &title)
            } else {
                svg
            };

            println!(
                "✨ [Archify-Core] Rendering Neon Glow {} to '{}'...",
                if is_html { "HTML" } else { "SVG" },
                output
            );
            fs::write(&output, contents)?;
            println!(
                "🎉 [Archify-Core] Generated {} diagram at '{}' successfully!",
                diagram_type, output
            );
        }
        Commands::Map { cache, max_dirs } => {
            if !Path::new(&cache).exists() {
                eprintln!(
                    "❌ Cache not found at '{}'. Run 'archify-graft index' first.",
                    cache
                );
                std::process::exit(1);
            }
            let graph = GraphStorage::load_mmap(&cache)?;
            print!("{}", build_repo_map(&graph, max_dirs));
        }
        Commands::Grep {
            pattern,
            cache,
            ignore_case,
        } => {
            if !Path::new(&cache).exists() {
                eprintln!(
                    "❌ Cache not found at '{}'. Run 'archify-graft index' first.",
                    cache
                );
                std::process::exit(1);
            }
            let graph = GraphStorage::load_mmap(&cache)?;
            print!("{}", grep_graph(&graph, &pattern, ignore_case)?);
        }
        Commands::Skeleton { file, cache } => {
            if !Path::new(&cache).exists() {
                eprintln!(
                    "❌ Cache not found at '{}'. Run 'archify-graft index' first.",
                    cache
                );
                std::process::exit(1);
            }
            let graph = GraphStorage::load_mmap(&cache)?;
            print!("{}", file_skeleton(&graph, &file));
        }
        Commands::Check { path, cache } => {
            let (taze, bayat) = CodeExtractor::freshness_report(path, cache)?;
            if taze {
                println!("✅ graft cache güncel.");
            } else {
                println!(
                    "❌ graft cache BAYAT. {} dosya kaynaktan daha eski:",
                    bayat.len()
                );
                for f in &bayat {
                    println!("  {f}");
                }
                std::process::exit(1);
            }
        }
        Commands::Mcp => {
            let mcp = McpServer::new();
            mcp.run_stdio().await?;
        }
        #[cfg(feature = "gui")]
        Commands::Gui {
            title,
            locale,
            cache,
        } => {
            let diagram = if Path::new(&cache).exists() {
                println!("📖 [Archify-Studio] Loading graph from '{}'...", cache);
                let graph = GraphStorage::load_mmap(&cache)?;
                Some(GraftToArchifyBridge::compile(&graph, &title, &locale))
            } else {
                println!(
                    "ℹ️ [Archify-Studio] No cache found at '{}', opening empty studio canvas...",
                    cache
                );
                None
            };

            println!("🚀 [Archify-Studio] Launching interactive neon desktop studio window (Locale: {})...", locale);
            archify_egui::run_desktop(diagram, &locale)
                .map_err(|e| anyhow::anyhow!("Desktop studio error: {:?}", e))?;
        }
    }

    Ok(())
}
