use clap::{Parser, Subcommand};
use graft_parser::CodeExtractor;
use graft_search::{Bm25Index, GraphRank, GraphStorage};
use archify_bridge::GraftToArchifyBridge;
use archify_render::SvgRenderer;
use graft_mcp::McpServer;
use lowlevel_sys::{cache_line_size, HardwareTimer};
use std::fs;
use std::path::Path;

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
        #[arg(short, long, default_value = ".cache/graft-graph.json")]
        cache: String,
    },
    /// Query the codebase using trilingual BM25 + PageRank (TR/AR/EN)
    Ask {
        /// Natural language or keyword query
        query: String,
        /// Maximum number of file references to return
        #[arg(short, long, default_value_t = 10)]
        limit: usize,
        /// Cache path to read graph from
        #[arg(short, long, default_value = ".cache/graft-graph.json")]
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
        #[arg(short, long, default_value = ".cache/graft-graph.json")]
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
        #[arg(short, long, default_value = ".cache/graft-graph.json")]
        cache: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Index { path, cache } => {
            println!("⚡ [Hardware-Sys] Detected CPU cache line: {} bytes", cache_line_size());
            println!("🚀 [Graft-Core] Indexing codebase at '{}' using direct DMA buffers...", path);
            
            let hash_file = Path::new(&cache).with_extension("hashes.json");
            let timer = HardwareTimer::start();

            let (graph, changed) = if Path::new(&cache).exists() && hash_file.exists() {
                println!("⚡ [Graft-Core] Found existing index. Performing incremental change scan...");
                let mut prev_graph = GraphStorage::load_mmap(&cache)?;
                let mut hash_index = graft_parser::HashIndex::load_from_file(&hash_file)?;
                let changed = CodeExtractor::index_directory_incremental(&path, &mut prev_graph, &mut hash_index)?;
                let _ = hash_index.save_to_file(&hash_file);
                (prev_graph, changed)
            } else {
                let graph = CodeExtractor::index_directory(&path)?;
                let mut hash_index = graft_parser::HashIndex::new();
                let mut file_paths = Vec::new();
                let _ = CodeExtractor::collect_files(Path::new(&path), &mut file_paths);
                for fp in file_paths {
                    if let Ok(bytes) = lowlevel_sys::DirectReader::read_file(&fp) {
                        hash_index.update(fp.to_string_lossy().to_string(), graft_parser::HashIndex::hash_bytes(&bytes));
                    }
                }
                if let Some(parent) = hash_file.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = hash_index.save_to_file(&hash_file);
                let count = graph.nodes.len();
                (graph, count)
            };
            let elapsed_cycles = timer.elapsed_cycles();

            println!("✅ [Graft-Core] Processed {} files. Index contains {} nodes and {} edges (in {} CPU cycles).", 
                changed, graph.nodes.len(), graph.edges.len(), elapsed_cycles);

            if let Some(parent) = Path::new(&cache).parent() {
                fs::create_dir_all(parent)?;
            }

            GraphStorage::save(&graph, &cache)?;
            println!("💾 [Graft-Core] Saved memory-mapped graph index to '{}'.", cache);
        }
        Commands::Ask { query, limit, cache } => {
            if !Path::new(&cache).exists() {
                eprintln!("❌ Cache not found at '{}'. Run 'archify-graft index' first.", cache);
                std::process::exit(1);
            }

            println!("📖 [Graft-Core] Loading memory-mapped graph from '{}'...", cache);
            let graph = GraphStorage::load_mmap(&cache)?;

            println!("🔍 [Graft-Core] Building trilingual lexical index (TR/AR/EN)...");
            let mut bm25 = Bm25Index::new();
            for node in &graph.nodes {
                let content = format!("{} {} {}", node.name, node.path, node.search_body);
                bm25.add_document(node.id.clone(), &content);
            }

            println!("⚡ [Graft-Core] Searching query '{}' with BM25 + GraphRank...", query);
            let bm25_results = bm25.search(&query, limit * 3);
            let ranked = GraphRank::compute(&graph, &bm25_results, 0.25, 25);

            println!("\n🏆 [Results: Top {} Matches]:", limit);
            for (idx, (id, score)) in ranked.iter().take(limit).enumerate() {
                println!("  {:2}. [{:.4}] {}", idx + 1, score, id);
            }
        }
        Commands::Diagram { diagram_type, entrypoint, compare, output, title, locale, cache } => {
            if !Path::new(&cache).exists() {
                eprintln!("❌ Cache not found at '{}'. Run 'archify-graft index' first.", cache);
                std::process::exit(1);
            }

            println!("📖 [Graft-Core] Loading graph from '{}'...", cache);
            let graph = GraphStorage::load_mmap(&cache)?;

            let svg = match diagram_type.to_lowercase().as_str() {
                "sequence" => {
                    println!("🎨 [Archify-Core] Compiling Sequence Diagram for entrypoint '{}'...", entrypoint);
                    let seq = GraftToArchifyBridge::compile_sequence(&graph, &entrypoint, &title, &locale);
                    SvgRenderer::render_sequence(&seq)
                }
                "dataflow" => {
                    println!("🎨 [Archify-Core] Compiling Dataflow Pipeline Diagram...");
                    let df = GraftToArchifyBridge::compile_dataflow(&graph, &title, &locale);
                    SvgRenderer::render_dataflow(&df)
                }
                "delta" => {
                    let prev_cache = compare.unwrap_or_default();
                    let before_graph = if !prev_cache.is_empty() && Path::new(&prev_cache).exists() {
                        println!("📖 [Archify-Core] Loading base graph for delta comparison from '{}'...", prev_cache);
                        GraphStorage::load_mmap(&prev_cache)?
                    } else {
                        println!("ℹ️ [Archify-Core] No valid prior cache given, comparing against empty state.");
                        graft_model::CodeGraph::new()
                    };
                    println!("🎨 [Archify-Core] Computing Architectural Delta Analysis...");
                    let delta = archify_delta::DeltaEngine::compute_graph_delta(&before_graph, &graph, &title, &locale);
                    SvgRenderer::render_delta(&delta)
                }
                _ => {
                    println!("🎨 [Archify-Core] Compiling Architecture Diagram...");
                    let diagram = GraftToArchifyBridge::compile(&graph, &title, &locale);
                    SvgRenderer::render(&diagram)
                }
            };

            println!("✨ [Archify-Core] Rendering Neon Glow SVG to '{}'...", output);
            fs::write(&output, svg)?;
            println!("🎉 [Archify-Core] Generated {} diagram at '{}' successfully!", diagram_type, output);
        }
        Commands::Mcp => {
            let mcp = McpServer::new();
            mcp.run_stdio().await?;
        }
        #[cfg(feature = "gui")]
        Commands::Gui { title, locale, cache } => {
            let diagram = if Path::new(&cache).exists() {
                println!("📖 [Archify-Studio] Loading graph from '{}'...", cache);
                let graph = GraphStorage::load_mmap(&cache)?;
                Some(GraftToArchifyBridge::compile(&graph, &title, &locale))
            } else {
                println!("ℹ️ [Archify-Studio] No cache found at '{}', opening empty studio canvas...", cache);
                None
            };

            println!("🚀 [Archify-Studio] Launching interactive neon desktop studio window (Locale: {})...", locale);
            archify_egui::run_desktop(diagram, &locale)
                .map_err(|e| anyhow::anyhow!("Desktop studio error: {:?}", e))?;
        }
    }

    Ok(())
}
