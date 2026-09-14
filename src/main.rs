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
    /// Generate an Archify architecture JSON-IR and neon SVG diagram from indexed code
    Diagram {
        /// Output path for SVG diagram file
        #[arg(short, long, default_value = "architecture.svg")]
        output: String,
        /// Title for the diagram
        #[arg(short, long, default_value = "Codebase Architecture Map")]
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Index { path, cache } => {
            println!("⚡ [Hardware-Sys] Detected CPU cache line: {} bytes", cache_line_size());
            println!("🚀 [Graft-Core] Indexing codebase at '{}' using direct DMA buffers...", path);
            
            let timer = HardwareTimer::start();
            let graph = CodeExtractor::index_directory(&path)?;
            let elapsed_cycles = timer.elapsed_cycles();

            println!("✅ [Graft-Core] Extracted {} nodes and {} edges in {} CPU cycles.", 
                graph.nodes.len(), graph.edges.len(), elapsed_cycles);

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
        Commands::Diagram { output, title, locale, cache } => {
            if !Path::new(&cache).exists() {
                eprintln!("❌ Cache not found at '{}'. Run 'archify-graft index' first.", cache);
                std::process::exit(1);
            }

            println!("📖 [Graft-Core] Loading graph from '{}'...", cache);
            let graph = GraphStorage::load_mmap(&cache)?;

            println!("🎨 [Archify-Core] Compiling AST into Archify Architecture JSON-IR (Locale: {})...", locale);
            let diagram = GraftToArchifyBridge::compile(&graph, &title, &locale);

            println!("✨ [Archify-Core] Rendering Neon Glow SVG to '{}'...", output);
            let svg = SvgRenderer::render(&diagram);

            fs::write(&output, svg)?;
            println!("🎉 [Archify-Core] Generated architecture diagram at '{}' successfully!", output);
        }
        Commands::Mcp => {
            let mcp = McpServer::new();
            mcp.run_stdio().await?;
        }
    }

    Ok(())
}
