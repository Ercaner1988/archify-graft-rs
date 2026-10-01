//! `cargo run -p archify-bridge --example bridge_dump -- <repo dir> [crate id] [locale]`:
//! indexes a repository and prints the level-1 diagram (or the level-2 one of a crate)
//! and the dataflow as plain text: boxes, arrows, regions, tour.

use archify_bridge::GraftToArchifyBridge as B;
use graft_parser::CodeExtractor;

fn main() {
    let mut args = std::env::args().skip(1);
    let root = args.next().unwrap_or_else(|| ".".into());
    let krate = args.next().filter(|s| s != "-").unwrap_or_default();
    let locale = args.next().unwrap_or_else(|| "tr".into());
    let t = std::time::Instant::now();
    let g = CodeExtractor::index_directory(&root).expect("index");
    let t_index = t.elapsed();

    let t = std::time::Instant::now();
    let d = if krate.is_empty() {
        B::compile(&g, "demo", &locale)
    } else {
        B::compile_module_level(&g, &krate, "demo", &locale).expect("crate not drillable")
    };
    println!("index {t_index:?}, compile {:?}", t.elapsed());
    let (mut x1, mut y1) = (0.0f32, 0.0f32);
    for r in &d.regions {
        x1 = x1.max(r.x + r.width);
        y1 = y1.max(r.y + r.height);
    }
    for c in &d.components {
        x1 = x1.max(c.x + c.width);
        y1 = y1.max(c.y + c.height);
    }
    let (w, h) = (x1 - 40.0, y1 - 110.0);
    println!("canvas {w:.0}x{h:.0} ratio {:.2}", w / h);
    println!(
        "{} | {}",
        d.meta.title,
        d.meta.subtitle.clone().unwrap_or_default()
    );
    for r in &d.regions {
        println!(
            "REGION {:<22} x{:>5.0} y{:>5.0} {:>5.0}x{:<5.0} {}",
            r.label,
            r.x,
            r.y,
            r.width,
            r.height,
            r.sublabel.clone().unwrap_or_default()
        );
    }
    for c in &d.components {
        println!(
            "BOX    {:<22} x{:>5.0} y{:>5.0} {:?} | {}",
            c.id,
            c.x,
            c.y,
            c.role,
            c.sublabel.clone().unwrap_or_default()
        );
    }
    for c in &d.connections {
        println!(
            "EDGE   {} -> {}  [{}] {}",
            c.from,
            c.to,
            c.line_style,
            c.label.clone().unwrap_or_default()
        );
    }
    for s in &d.story_beats {
        println!("TOUR   {} {:?}", s.title, s.highlighted_nodes);
        println!("         {}", s.description.clone().unwrap_or_default());
    }
    if krate.is_empty() {
        let f = B::compile_dataflow(&g, "demo", &locale);
        println!("DATAFLOW {}", f.meta.subtitle.clone().unwrap_or_default());
        for n in &f.nodes {
            println!(
                "  NODE {:<34} {:?} | {}",
                n.label,
                n.role,
                n.stream_rate.clone().unwrap_or_default()
            );
        }
        for p in &f.pipelines {
            println!(
                "  FLOW {} -[{}]-> {}",
                p.from.rsplit(['/', '\\']).next().unwrap_or(""),
                p.schema.clone().unwrap_or_default(),
                p.to.rsplit(['/', '\\']).next().unwrap_or("")
            );
        }
    }
}
