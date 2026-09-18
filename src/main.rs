use archify_bridge::GraftToArchifyBridge;
use archify_render::SvgRenderer;
use clap::{Parser, Subcommand};
use graft_mcp::McpServer;
use graft_parser::CodeExtractor;
use graft_search::{Bm25Index, GraphRank, GraphStorage};
use lowlevel_sys::{cache_line_size, HardwareTimer};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// `graft/.graph/wiring.json` başlığı — pasli-beyin'in `kopru.rs::graft_koprusu`
/// köprüsünün beklediği KÜÇÜK, sürümlü sözleşme: yalnız `meta` (nodeCount,
/// edgeCount, languages). Düğüm/kenar dizisinin TAMAMINI yazmıyoruz — köprü
/// zaten onlara dokunmuyor (`kopru.rs`: "düğüm dizisi büyük olabilir, ona hiç
/// dokunmayız"), gereksiz yere büyük bir JSON üretmenin anlamı yok. Bu, eski
/// Node.js `@nanonets/graft`'ın yerini almanın en ucuz, en somut adımı.
fn write_wiring_meta(indexed_path: &str, graph: &graft_model::CodeGraph) -> anyhow::Result<()> {
    let languages: BTreeSet<&'static str> = graph
        .nodes
        .iter()
        .filter_map(|n| Path::new(&n.path).extension()?.to_str())
        .filter_map(extension_to_language)
        .collect();

    let wiring = serde_json::json!({
        "meta": {
            "version": 1,
            "nodeCount": graph.nodes.len(),
            "edgeCount": graph.edges.len(),
            "languages": languages.into_iter().collect::<Vec<_>>(),
        }
    });

    let dir = Path::new(indexed_path).join("graft").join(".graph");
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("wiring.json"), serde_json::to_string_pretty(&wiring)?)?;
    Ok(())
}

/// `graft-parser`'ın taradığı uzantı kümesiyle AYNI (bkz. `is_supported_extension`) —
/// biri değişirse öbürü bayatlar, o yüzden burada da elle listelenir.
fn extension_to_language(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "rs" => "rust",
        "ts" => "typescript",
        "js" => "javascript",
        "py" => "python",
        "go" => "go",
        "java" => "java",
        "c" | "h" => "c",
        "cpp" | "hpp" => "cpp",
        "json" => "json",
        _ => return None,
    })
}

struct DizinIstatistik {
    dosya: usize,
    sembol: usize,
    /// (isim, dosya_yolu, gelen_referans)
    hublar: Vec<(String, String, usize)>,
}

/// `map` — dizin başına özet + genel hotspot listesi. Sıralama sinyali
/// "gelen kenar sayısı" (relation farketmeksizin): şu an bu ÇOĞUNLUKLA
/// `Contains` (dosya→sembol) demek, `Calls` kenarları extractor'ın yalnız
/// bildirim SATIRINI taraması yüzünden az/gürültülü (bkz. `resolver.rs`,
/// ölçüldü: pasli-beyin/cekirdek'te 480 kenarın 89'u Calls). Yani "hub" burada
/// "bu dosyada en çok sembol barındıran/adı geçen" anlamına geliyor —
/// `@nanonets/graft`'ın AST-doğru çağrı-grafiği hotspot'larıyla BİREBİR AYNI
/// KEsinlikte değil. Extractor gerçek çağrı çözümlemesi kazanınca bu fonksiyon
/// DEĞİŞMEDEN doğruluğu otomatik yükselir (girdi kalitesi sorunu, algoritma değil).
fn print_map(graph: &graft_model::CodeGraph, max_dirs: usize) {
    use graft_model::NodeKind;
    use std::collections::BTreeMap;

    let mut gelen: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for e in &graph.edges {
        *gelen.entry(e.target.as_str()).or_insert(0) += 1;
    }

    let dosya_sayisi = graph
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::File)
        .count();
    let sembol_sayisi = graph.nodes.len() - dosya_sayisi;

    let mut dizinler: BTreeMap<String, DizinIstatistik> = BTreeMap::new();
    for node in &graph.nodes {
        let dizin = Path::new(&node.path)
            .parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let girdi = dizinler.entry(dizin).or_insert(DizinIstatistik {
            dosya: 0,
            sembol: 0,
            hublar: Vec::new(),
        });
        if node.kind == NodeKind::File {
            girdi.dosya += 1;
        } else {
            girdi.sembol += 1;
            let ref_sayisi = gelen.get(node.id.as_str()).copied().unwrap_or(0);
            girdi
                .hublar
                .push((node.name.clone(), node.path.clone(), ref_sayisi));
        }
    }

    println!(
        "repo map — {} dosya · {} sembol · {} kenar",
        dosya_sayisi,
        sembol_sayisi,
        graph.edges.len()
    );
    println!();

    let toplam_dizin = dizinler.len();
    for (i, (dizin, ist)) in dizinler.iter().enumerate() {
        if i >= max_dirs {
            println!(
                "… +{} dizin daha gösterilmedi (--max-dirs ile genişlet)",
                toplam_dizin - max_dirs
            );
            break;
        }
        let mut hublar = ist.hublar.clone();
        hublar.sort_by(|a, b| b.2.cmp(&a.2));
        let ust: Vec<String> = hublar
            .iter()
            .filter(|h| h.2 > 0)
            .take(3)
            .map(|(isim, yol, refs)| {
                let dosya_adi = Path::new(yol)
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_default();
                format!("{isim} ({dosya_adi}, {refs}←)")
            })
            .collect();
        print!(
            "{:<45}{} dosya · {} sembol",
            if dizin.is_empty() { "(kök)" } else { dizin },
            ist.dosya,
            ist.sembol
        );
        if !ust.is_empty() {
            print!("   hublar: {}", ust.join(", "));
        }
        println!();
    }

    let mut tum_semboller: Vec<(&str, &NodeKind, &str, usize)> = graph
        .nodes
        .iter()
        .filter(|n| n.kind != NodeKind::File)
        .map(|n| {
            (
                n.name.as_str(),
                &n.kind,
                n.path.as_str(),
                gelen.get(n.id.as_str()).copied().unwrap_or(0),
            )
        })
        .collect();
    tum_semboller.sort_by(|a, b| b.3.cmp(&a.3));
    let hotspotlar: Vec<String> = tum_semboller
        .iter()
        .filter(|s| s.3 > 0)
        .take(12)
        .map(|(isim, tur, yol, refs)| {
            format!("{isim} · {tur:?} · {} · {refs}←", yol.replace('\\', "/"))
        })
        .collect();
    if !hotspotlar.is_empty() {
        println!();
        println!("hotspots: {}", hotspotlar.join("  "));
    }
}

/// `grep` — iki ayrı katman:
///  1) Bildirim-satırı eşleşmeleri: Function/Method/Class/vb. düğümlerin
///     `search_body`'si TEK SATIRLIK bildirim metnidir (bkz. `extractor.rs`) —
///     yani burada eşleşme "bu isim/imza örtüşüyor" demektir, gövde değil.
///  2) Dosya-içeriği eşleşmeleri: File düğümlerinin `file_residual`'ı (ilk
///     8000 karakter) satır satır taranır, GERÇEK satır numarası verilir.
/// "Kapsayan sembole göre gruplama" (@nanonets/graft'ın yaptığı) burada YOK
/// — `span` alanı extractor tarafından hiç doldurulmuyor (`extractor.rs`:
/// her düğümde `span: None`), yani bir satırın hangi fonksiyonun içinde
/// olduğunu bilmiyoruz. Bu, gerçek bir extractor sınırı — span doldurulunca
/// bu fonksiyon gruplamayı ekleyebilir.
fn run_grep(graph: &graft_model::CodeGraph, pattern: &str, ignore_case: bool) -> anyhow::Result<()> {
    use graft_model::NodeKind;

    let re = regex::RegexBuilder::new(pattern)
        .case_insensitive(ignore_case)
        .build()?;

    let mut gelen: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for e in &graph.edges {
        *gelen.entry(e.target.as_str()).or_insert(0) += 1;
    }

    let mut bildirim_eslesme: Vec<(&graft_model::NodeV1, usize)> = graph
        .nodes
        .iter()
        .filter(|n| n.kind != NodeKind::File && re.is_match(&n.search_body))
        .map(|n| (n, gelen.get(n.id.as_str()).copied().unwrap_or(0)))
        .collect();
    bildirim_eslesme.sort_by(|a, b| b.1.cmp(&a.1));

    if !bildirim_eslesme.is_empty() {
        println!("## bildirim satırı eşleşmeleri (isim/imza — gövde değil)");
        for (node, refs) in &bildirim_eslesme {
            println!(
                "  {} · {:?} · {} · {refs}←",
                node.name,
                node.kind,
                node.path.replace('\\', "/")
            );
        }
        println!();
    }

    println!("## dosya içeriği eşleşmeleri (satır numaralı, ilk 8000 karakterle sınırlı)");
    let mut dosya_eslesme = 0usize;
    for node in &graph.nodes {
        if node.kind != NodeKind::File {
            continue;
        }
        let mut yol_yazildi = false;
        for (i, satir) in node.file_residual.lines().enumerate() {
            if re.is_match(satir) {
                if !yol_yazildi {
                    println!("{}", node.path.replace('\\', "/"));
                    yol_yazildi = true;
                }
                println!("  {}: {}", i + 1, satir.trim());
                dosya_eslesme += 1;
            }
        }
    }

    println!();
    println!(
        "{} bildirim eşleşmesi, {} dosya-içeriği eşleşmesi",
        bildirim_eslesme.len(),
        dosya_eslesme
    );

    Ok(())
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
        Commands::Index { path, cache } => {
            println!(
                "⚡ [Hardware-Sys] Detected CPU cache line: {} bytes",
                cache_line_size()
            );
            println!(
                "🚀 [Graft-Core] Indexing codebase at '{}' using direct DMA buffers...",
                path
            );

            let hash_file = Path::new(&cache).with_extension("hashes.json");
            let timer = HardwareTimer::start();

            let (graph, changed) = if Path::new(&cache).exists() && hash_file.exists() {
                println!(
                    "⚡ [Graft-Core] Found existing index. Performing incremental change scan..."
                );
                let mut prev_graph = GraphStorage::load_mmap(&cache)?;
                let mut hash_index = graft_parser::HashIndex::load_from_file(&hash_file)?;
                let changed = CodeExtractor::index_directory_incremental(
                    &path,
                    &mut prev_graph,
                    &mut hash_index,
                )?;
                let _ = hash_index.save_to_file(&hash_file);
                (prev_graph, changed)
            } else {
                let graph = CodeExtractor::index_directory(&path)?;
                let mut hash_index = graft_parser::HashIndex::new();
                let mut file_paths = Vec::new();
                let _ = CodeExtractor::collect_files(Path::new(&path), &mut file_paths);
                for fp in file_paths {
                    if let Ok(bytes) = lowlevel_sys::DirectReader::read_file(&fp) {
                        hash_index.update(
                            fp.to_string_lossy().to_string(),
                            graft_parser::HashIndex::hash_bytes(&bytes),
                        );
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
            println!(
                "💾 [Graft-Core] Saved memory-mapped graph index to '{}'.",
                cache
            );

            write_wiring_meta(&path, &graph)?;
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

            println!(
                "✨ [Archify-Core] Rendering Neon Glow SVG to '{}'...",
                output
            );
            fs::write(&output, svg)?;
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
            print_map(&graph, max_dirs);
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
            run_grep(&graph, &pattern, ignore_case)?;
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
