//! Opens the studio on the hand-drawn reference diagram (10 boxes, 9 edges, 1 region, 3 tour
//! beats): `cargo run -p archify-egui --example studio --features wgpu`.

use archify_ir::{
    ArchitectureDiagram, Component, Connection, DataPipeline, DataflowDiagram, DataflowNode,
    DiagramMeta, Region, SemanticRole as R, StoryBeat, VisualPreset,
};

fn comp(id: &str, label: &str, sub: &str, role: R, col: f32, row: f32) -> Component {
    Component {
        id: id.into(),
        label: label.into(),
        sublabel: Some(sub.into()),
        role,
        x: 40.0 + col * 270.0,
        y: 80.0 + row * 220.0,
        width: 170.0,
        height: 64.0,
    }
}

fn edge(from: &str, to: &str, label: &str, style: &str) -> Connection {
    Connection {
        from: from.into(),
        to: to.into(),
        label: Some(label.into()),
        line_style: style.into(),
    }
}

fn beat(step: usize, title: &str, desc: &str, nodes: &[&str]) -> StoryBeat {
    StoryBeat {
        step,
        title: title.into(),
        description: Some(desc.into()),
        highlighted_nodes: nodes.iter().map(|s| s.to_string()).collect(),
    }
}

fn reference() -> ArchitectureDiagram {
    ArchitectureDiagram {
        meta: DiagramMeta {
            title: "Paslı Beyin — bağlantı haritası".into(),
            subtitle: Some("graphify destekli".into()),
            locale: "tr".into(),
            visual_preset: VisualPreset::Editorial,
        },
        components: vec![
            comp(
                "pano",
                "Pano envanteri",
                "envanter.json — tek kaynak",
                R::Database,
                0.0,
                1.0,
            ),
            comp(
                "kanca",
                "Git kancaları",
                "core.hooksPath → trigger",
                R::Messagebus,
                0.0,
                2.0,
            ),
            comp(
                "serve",
                "pasli serve",
                "named pipe + spool",
                R::Backend,
                1.0,
                2.0,
            ),
            comp(
                "pencere",
                "Pencere",
                "Tauri 2 canvas arayüz",
                R::Frontend,
                2.0,
                2.0,
            ),
            comp(
                "cekirdek",
                "pasli-çekirdek",
                "graf · istem · gölge",
                R::Backend,
                2.0,
                1.0,
            ),
            comp(
                "cli",
                "pasli CLI",
                "ara · istem · gonder",
                R::Backend,
                3.0,
                1.0,
            ),
            comp(
                "fihrist",
                "el-Fihrist",
                "BM25 — Türkçe katlamalı",
                R::Backend,
                2.0,
                0.0,
            ),
            comp(
                "graphify",
                "graphify-rs",
                "AST kenarları + Leiden",
                R::Backend,
                3.0,
                0.0,
            ),
            comp(
                "kasa",
                "Credential Manager",
                "pasli-beyin/glm-api-key",
                R::Security,
                4.0,
                1.0,
            ),
            comp(
                "mistral",
                "Mistral API",
                "OpenAI-uyumlu /chat",
                R::External,
                4.0,
                2.0,
            ),
        ],
        connections: vec![
            edge("pano", "cekirdek", "envanter oku", "strong"),
            edge("kanca", "serve", "olay (boru/spool)", ""),
            edge("serve", "pencere", "beyin.json tazele", ""),
            edge("cekirdek", "fihrist", "üç kanallı arama", ""),
            edge("cekirdek", "graphify", "AST ölçüm (grafa YAZMAZ)", "dashed"),
            edge("pencere", "cekirdek", "bağlam · istem · gölge", ""),
            edge("cli", "cekirdek", "komutlar", ""),
            edge("cli", "kasa", "anahtar oku", "security"),
            edge("cli", "mistral", "yalnız gonder", "strong"),
        ],
        regions: vec![Region {
            id: "yerel".into(),
            label: "pasli-beyin çalışma yüzü (yerel)".into(),
            sublabel: None,
            x: 280.0,
            y: 50.0,
            width: 770.0,
            height: 554.0,
        }],
        story_beats: vec![
            beat(
                1,
                "Ölçüm zinciri",
                "Pano envanteri tek ölçüm kaynağı; çekirdek grafa çevirir.",
                &["pano", "cekirdek", "fihrist", "graphify"],
            ),
            beat(
                2,
                "Üç kanallı arama",
                "BM25 + yerel gömme + karmaşık.",
                &["fihrist", "cekirdek", "graphify"],
            ),
            beat(
                3,
                "Model yolu (tek kapı)",
                "Ağ yalnız 'pasli gonder' ile açılır.",
                &["cli", "kasa", "mistral"],
            ),
        ],
    }
}

fn flow() -> DataflowDiagram {
    let n = |id: &str, label: &str, rate: &str, role: R| DataflowNode {
        id: id.into(),
        label: label.into(),
        role,
        stream_rate: Some(rate.into()),
    };
    let p = |a: &str, b: &str, s: &str| DataPipeline {
        from: a.into(),
        to: b.into(),
        schema: Some(s.into()),
        throughput: None,
    };
    DataflowDiagram {
        meta: DiagramMeta {
            title: "Veri akışı".into(),
            subtitle: None,
            locale: "tr".into(),
            visual_preset: VisualPreset::Editorial,
        },
        nodes: vec![
            n("uret", "pasli uret", "yazar", R::Backend),
            n("env", "envanter.bin", "artifact", R::Database),
            n("oku", "oku", "okur", R::Backend),
            n("pencere", "Pencere", "gösterir", R::Frontend),
            n(
                "uzun",
                "00ebbd47387f9a1c2e3d4b5a69788796a5b4c3d2e1f00112233445566778899aabbc.json",
                "önbellek",
                R::External,
            ),
        ],
        pipelines: vec![
            p("uret", "env", "yazar"),
            p("env", "oku", "okur"),
            p("oku", "pencere", "graf"),
            p("oku", "uzun", "önbellek"),
        ],
    }
}

fn main() -> eframe::Result<()> {
    archify_egui::run_desktop(Some(reference()), Some(flow()), "tr")
}
