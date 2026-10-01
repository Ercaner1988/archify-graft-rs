use super::*;
use archify_ir::{
    Component, Connection, DataPipeline, DataflowDiagram, DataflowNode, DeltaComponent,
    DeltaConnection, DeltaDiagram, DiagramMeta, DiffStatus, Region, SemanticRole, SequenceDiagram,
    SequenceMessage, SequenceParticipant, StoryBeat, VisualPreset,
};
use archify_route::poly::hits_rect;
use archify_scene::Scene;

fn meta(title: &str, locale: &str, preset: VisualPreset) -> DiagramMeta {
    DiagramMeta {
        title: title.to_string(),
        subtitle: Some("alt başlık".to_string()),
        locale: locale.to_string(),
        visual_preset: preset,
    }
}

fn comp(id: &str, label: &str, role: SemanticRole, x: f32, y: f32) -> Component {
    Component {
        id: id.to_string(),
        label: label.to_string(),
        sublabel: Some("alt metin".to_string()),
        role,
        x,
        y,
        width: 170.0,
        height: 64.0,
    }
}

fn conn(from: &str, to: &str, label: Option<&str>, style: &str) -> Connection {
    Connection {
        from: from.to_string(),
        to: to.to_string(),
        label: label.map(str::to_string),
        line_style: style.to_string(),
    }
}

/// Tiny XML well-formedness check: balanced tags, quoted attributes, only known entities.
fn assert_well_formed(svg: &str) {
    let b = svg.as_bytes();
    let mut stack: Vec<String> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'&' => {
                let rest = &svg[i..];
                assert!(
                    ["&amp;", "&lt;", "&gt;", "&quot;", "&#"]
                        .iter()
                        .any(|e| rest.starts_with(e)),
                    "bare & near {:?}",
                    &rest[..rest.len().min(30)]
                );
                i += 1;
            }
            b'<' => {
                let end = svg[i..].find('>').expect("unterminated tag") + i;
                let tag = &svg[i + 1..end];
                assert!(!tag.contains('<'), "nested < in tag {tag:?}");
                if let Some(name) = tag.strip_prefix('/') {
                    assert_eq!(
                        stack.pop().as_deref(),
                        Some(name.trim()),
                        "mismatched </{name}>"
                    );
                } else if !tag.starts_with('!') && !tag.starts_with('?') {
                    let name: String = tag
                        .chars()
                        .take_while(|c| !c.is_whitespace() && *c != '/')
                        .collect();
                    assert_eq!(
                        tag.matches('"').count() % 2,
                        0,
                        "unbalanced quotes in <{tag}>"
                    );
                    if !tag.ends_with('/') {
                        stack.push(name);
                    }
                }
                i = end + 1;
            }
            _ => i += 1,
        }
    }
    assert!(stack.is_empty(), "unclosed: {stack:?}");
}

fn grid_diagram(n: usize) -> ArchitectureDiagram {
    let cols = 5;
    let roles = [
        SemanticRole::Frontend,
        SemanticRole::Backend,
        SemanticRole::Database,
        SemanticRole::Messagebus,
        SemanticRole::External,
    ];
    let components: Vec<Component> = (0..n)
        .map(|i| {
            comp(
                &format!("c{i}"),
                &format!("modül-{i}"),
                roles[i % roles.len()],
                40.0 + (i % cols) as f32 * 270.0,
                80.0 + (i / cols) as f32 * 120.0,
            )
        })
        .collect();
    let mut connections = Vec::new();
    for i in 0..n {
        if i + 1 < n {
            connections.push(conn(
                &format!("c{i}"),
                &format!("c{}", i + 1),
                Some("çağırır ×3"),
                "",
            ));
        }
        if i + cols < n {
            connections.push(conn(
                &format!("c{i}"),
                &format!("c{}", i + cols),
                None,
                "dashed",
            ));
        }
        if i + 7 < n {
            connections.push(conn(
                &format!("c{i}"),
                &format!("c{}", i + 7),
                Some("uzun"),
                "strong",
            ));
        }
    }
    ArchitectureDiagram {
        meta: meta("Izgara", "tr", VisualPreset::Editorial),
        components,
        connections,
        regions: vec![],
        story_beats: vec![],
    }
}

fn reference_diagram() -> ArchitectureDiagram {
    let c = |id: &str, label: &str, sub: &str, role, col: f32, row: f32| Component {
        id: id.to_string(),
        label: label.to_string(),
        sublabel: Some(sub.to_string()),
        role,
        x: 40.0 + col * 270.0,
        y: 80.0 + row * 220.0,
        width: 170.0,
        height: 64.0,
    };
    ArchitectureDiagram {
        meta: meta(
            "Paslı Beyin — bağlantı haritası",
            "tr",
            VisualPreset::Classic,
        ),
        components: vec![
            c(
                "pano",
                "Pano envanteri",
                "envanter.json — tek kaynak",
                SemanticRole::Database,
                0.0,
                1.0,
            ),
            c(
                "kanca",
                "Git kancaları",
                "core.hooksPath → trigger",
                SemanticRole::Messagebus,
                0.0,
                2.0,
            ),
            c(
                "serve",
                "pasli serve",
                "named pipe + spool",
                SemanticRole::Backend,
                1.0,
                2.0,
            ),
            c(
                "pencere",
                "Pencere",
                "Tauri 2 canvas arayüz",
                SemanticRole::Frontend,
                2.0,
                2.0,
            ),
            c(
                "cekirdek",
                "pasli-çekirdek",
                "graf · istem · gölge",
                SemanticRole::Backend,
                2.0,
                1.0,
            ),
            c(
                "cli",
                "pasli CLI",
                "ara · istem · gonder",
                SemanticRole::Backend,
                3.0,
                1.0,
            ),
            c(
                "fihrist",
                "el-Fihrist",
                "BM25 — Türkçe katlamalı",
                SemanticRole::Backend,
                2.0,
                0.0,
            ),
            c(
                "graphify",
                "graphify-rs",
                "AST kenarları + Leiden",
                SemanticRole::Backend,
                3.0,
                0.0,
            ),
            c(
                "kasa",
                "Credential Manager",
                "pasli-beyin/glm-api-key",
                SemanticRole::Security,
                4.0,
                1.0,
            ),
            c(
                "mistral",
                "Mistral API",
                "OpenAI-uyumlu",
                SemanticRole::External,
                4.0,
                2.0,
            ),
        ],
        connections: vec![
            conn("pano", "cekirdek", Some("envanter oku"), "strong"),
            conn("kanca", "serve", Some("olay (boru/spool)"), ""),
            conn("serve", "pencere", Some("beyin.json tazele"), ""),
            conn("cekirdek", "fihrist", Some("üç kanallı arama"), ""),
            conn(
                "cekirdek",
                "graphify",
                Some("AST ölçüm (grafa YAZMAZ)"),
                "dashed",
            ),
            conn("pencere", "cekirdek", Some("bağlam · istem · gölge"), ""),
            conn("cli", "cekirdek", Some("komutlar"), ""),
            conn("cli", "kasa", Some("anahtar oku"), "security"),
            conn("cli", "mistral", Some("yalnız gonder"), "strong"),
        ],
        regions: vec![Region {
            id: "yerel".to_string(),
            label: "pasli-beyin çalışma yüzü (yerel)".to_string(),
            sublabel: None,
            x: 280.0,
            y: 50.0,
            width: 770.0,
            height: 554.0,
        }],
        story_beats: vec![StoryBeat {
            step: 1,
            title: "Ölçüm zinciri".to_string(),
            description: None,
            highlighted_nodes: vec!["pano".into(), "cekirdek".into(), "fihrist".into()],
        }],
    }
}

fn count(s: &str, needle: &str) -> usize {
    s.matches(needle).count()
}

#[test]
fn all_four_presets_use_their_own_tokens() {
    let mut diagram = grid_diagram(2);
    let expect = [
        (VisualPreset::Classic, "#020617"),
        (VisualPreset::SignalFlow, "#030711"),
        (VisualPreset::Blueprint, "#06131f"),
        (VisualPreset::Editorial, "#181611"),
    ];
    for (preset, bg) in expect {
        diagram.meta.visual_preset = preset;
        let svg = SvgRenderer::render(&diagram);
        assert!(svg.contains(bg), "{preset:?} misses {bg}");
        assert!(svg.starts_with("<svg") && svg.ends_with("</svg>\n"));
        assert_well_formed(&svg);
    }
    diagram.meta.visual_preset = VisualPreset::Editorial;
    assert!(SvgRenderer::render(&diagram).contains("Georgia"));
    diagram.meta.visual_preset = VisualPreset::Blueprint;
    assert!(SvgRenderer::render(&diagram).contains("12 4 2 4"));
}

#[test]
fn reference_diagram_structure() {
    let d = reference_diagram();
    let svg = SvgRenderer::render(&d);
    assert_well_formed(&svg);
    assert_eq!(count(&svg, "class=\"node\""), 10);
    assert!(count(&svg, "data-edge-from") >= 9);
    assert_eq!(
        count(&svg, "marker-end=\"url(#m-"),
        9,
        "one arrowhead per edge"
    );
    assert_eq!(
        count(&svg, "class=\"lbl\""),
        9,
        "a label plate per labeled edge"
    );
    assert!(svg.contains("markerWidth=\"10\" markerHeight=\"7\""));
    assert!(svg.contains("class=\"region-frame\""));
    assert!(svg.contains("role=\"img\"") && svg.contains("<title>") && svg.contains("<desc>"));
    assert!(svg.contains("pasli-çekirdek"));
    // legend lists only the roles that occur, in Turkish
    assert!(svg.contains(">Depolama<") && svg.contains(">Güvenlik<"));
    assert!(!svg.contains(">Bulut<"));
}

#[test]
fn edges_are_rounded_orthogonal_paths_without_beziers() {
    let svg = SvgRenderer::render(&reference_diagram());
    let mut quads = 0;
    for d in svg.split(" d=\"").skip(1) {
        let d = &d[..d.find('"').unwrap()];
        assert!(
            !d.contains(" C ") && !d.starts_with("C "),
            "cubic bezier in {d}"
        );
        quads += d.matches(" Q ").count();
    }
    assert!(quads > 0, "bent edges must carry rounded corners");
}

#[test]
fn style_lives_in_one_place_and_never_inside_a_path() {
    let svg = SvgRenderer::render(&grid_diagram(6));
    assert_eq!(count(&svg, "<style>"), 1);
    assert_eq!(count(&svg, "</style>"), 1);
    for p in svg.split("<path").skip(1) {
        let tag = &p[..p.find('>').unwrap()];
        assert!(!tag.contains("<style"));
    }
    assert!(!svg.contains("<path><style"));
    assert!(!svg.contains("<script"));
}

#[test]
fn motion_is_css_only_and_respects_reduced_motion() {
    let mut d = grid_diagram(6);
    d.meta.visual_preset = VisualPreset::Classic;
    let svg = SvgRenderer::render(&d);
    assert!(svg.contains("@keyframes edge-flow"));
    assert!(svg.contains("@keyframes node-pulse"));
    assert!(svg.contains("animation-delay:calc(var(--step,0)*160ms)"));
    assert!(svg.contains("prefers-reduced-motion:reduce"));
    assert!(svg.contains("transition:opacity .18s ease,filter .18s ease"));
    assert!(svg.contains("drop-shadow(0 0 7px var(--frontend-stroke))"));
    assert!(svg.contains("prefers-color-scheme:light"));
    assert!(svg.contains("data-theme=\"auto\""));
}

#[test]
fn hovering_a_box_dims_the_rest() {
    let svg = SvgRenderer::render(&grid_diagram(6));
    assert!(svg.contains("svg:has(#n0:is(:hover,:focus-visible)) .node:not(#n0)"));
    assert!(svg.contains("opacity:.2"));
}

#[test]
fn text_is_escaped_everywhere() {
    let mut d = grid_diagram(2);
    d.components[0].label = "Vec<F> & \"x\"".to_string();
    d.components[0].sublabel = Some("<b>&</b>".to_string());
    d.connections[0].label = Some("a<b & c".to_string());
    d.meta.title = "Başlık <&>".to_string();
    let svg = SvgRenderer::render(&d);
    assert_well_formed(&svg);
    assert!(svg.contains("Vec&lt;F&gt; &amp;"));
    assert!(!svg.contains("<b>"));
    let html = wrap_html(&svg, "a <title> & co");
    assert!(html.contains("<title>a &lt;title&gt; &amp; co</title>"));
}

#[test]
fn view_box_follows_the_content() {
    let small = SvgRenderer::render(&grid_diagram(3));
    let big = SvgRenderer::render(&grid_diagram(40));
    let width = |svg: &str| {
        let vb = svg.split("viewBox=\"").nth(1).unwrap();
        let vb = &vb[..vb.find('"').unwrap()];
        let v: Vec<f32> = vb.split(' ').map(|x| x.parse().unwrap()).collect();
        (v[2], v[3])
    };
    let (sw, sh) = width(&small);
    let (bw, bh) = width(&big);
    assert!(bw > 900.0 && bh > 650.0, "{bw}x{bh}");
    assert!(sw < bw && sh < bh);
    assert!(!big.contains("viewBox=\"0 0 900 650\""));
}

#[test]
fn no_edge_passes_through_a_box_in_a_20_box_diagram() {
    let d = grid_diagram(20);
    let scene = Scene::from_architecture(&d);
    assert!(scene.edges.len() >= 20);
    for e in &scene.edges {
        for n in &scene.nodes {
            assert!(
                !hits_rect(&e.route.points, &n.rect),
                "edge {}->{} crosses {}",
                scene.nodes[e.from].id,
                scene.nodes[e.to].id,
                n.id
            );
        }
    }
    let svg = SvgRenderer::render(&d);
    assert_eq!(count(&svg, "marker-end=\"url(#m-"), scene.edges.len());
}

#[test]
fn arabic_is_right_to_left_and_keeps_its_text() {
    let mut d = grid_diagram(2);
    d.meta.locale = "ar".to_string();
    d.components[0].label = "الخادم".to_string();
    let svg = SvgRenderer::render(&d);
    assert!(svg.contains("dir=\"rtl\"") && svg.contains("الخادم"));
    assert!(svg.contains("text-anchor=\"end\""));
    assert_well_formed(&svg);
}

#[test]
fn html_wrapper_has_no_script() {
    let html = wrap_html(&SvgRenderer::render(&grid_diagram(3)), "Başlık");
    assert!(!html.contains("<script"));
    assert!(html.contains("prefers-color-scheme: light"));
    assert!(html.contains("<svg") && html.trim_end().ends_with("</html>"));
}

#[test]
fn dataflow_uses_the_same_pipeline_and_grows_with_content() {
    let nodes: Vec<DataflowNode> = (0..30)
        .map(|i| DataflowNode {
            id: format!("n{i}"),
            label: if i == 3 {
                "00ebbd47387f9a1c2e3d4b5a69788796a5b4c3d2e1f00112233445566778899aabbc.json".into()
            } else {
                format!("Akış {i}")
            },
            role: SemanticRole::Backend,
            stream_rate: Some("Direct DMA".to_string()),
        })
        .collect();
    let pipelines: Vec<DataPipeline> = (0..29)
        .map(|i| DataPipeline {
            from: format!("n{i}"),
            to: format!("n{}", i + 1),
            schema: None,
            throughput: Some("95%".into()),
        })
        .collect();
    let df = DataflowDiagram {
        meta: meta("Veri & Akış", "tr", VisualPreset::SignalFlow),
        nodes,
        pipelines,
    };
    let svg = SvgRenderer::render_dataflow(&df);
    assert_well_formed(&svg);
    assert!(svg.contains("Veri &amp; Akış") && svg.contains("95%") && svg.contains("Direct DMA"));
    assert_eq!(count(&svg, "marker-end=\"url(#m-"), 29);
    assert!(svg.contains("aabbc.json"), "extension survives truncation");
    assert!(!svg.contains(
        "00ebbd47387f9a1c2e3d4b5a69788796a5b4c3d2e1f00112233445566778899aabbc.json</text>"
    ));
}

#[test]
fn delta_shows_status_colours_and_legend() {
    let a = comp("a", "Auth", SemanticRole::Security, 50.0, 50.0);
    let b = comp("b", "Db", SemanticRole::Database, 350.0, 50.0);
    let c = comp("c", "Yeni", SemanticRole::Backend, 350.0, 200.0);
    let delta = DeltaDiagram {
        meta: meta("Fark", "tr", VisualPreset::SignalFlow),
        components: vec![
            DeltaComponent {
                component: a,
                status: DiffStatus::Unchanged,
            },
            DeltaComponent {
                component: b,
                status: DiffStatus::Removed,
            },
            DeltaComponent {
                component: c,
                status: DiffStatus::Added,
            },
        ],
        connections: vec![
            DeltaConnection {
                connection: conn("a", "b", None, ""),
                status: DiffStatus::Removed,
            },
            DeltaConnection {
                connection: conn("a", "c", Some("yeni"), ""),
                status: DiffStatus::Added,
            },
        ],
    };
    let svg = SvgRenderer::render_delta(&delta);
    assert_well_formed(&svg);
    for s in [
        "[+] Added",
        "[-] Removed",
        "[Δ] Modified",
        "#22c55e",
        "#f43f5e",
        "#f59e0b",
    ] {
        assert!(svg.contains(s), "missing {s}");
    }
    assert_eq!(count(&svg, "marker-end=\"url(#m-st-"), 2);
    assert!(svg.contains("stroke-dasharray:7 5"));
}

#[test]
fn sequence_grows_with_participants_and_messages() {
    let participants: Vec<SequenceParticipant> = (0..9)
        .map(|i| SequenceParticipant {
            id: format!("p{i}"),
            label: format!("katılımcı-{i}"),
            role: SemanticRole::Backend,
        })
        .collect();
    let messages: Vec<SequenceMessage> = (0..12)
        .map(|i| SequenceMessage {
            order: i + 1,
            from: format!("p{}", i % 9),
            to: format!("p{}", (i + 1) % 9),
            action: format!("calls f{i}<T>()"),
            is_async: i % 4 == 0,
        })
        .collect();
    let seq = SequenceDiagram {
        meta: meta("Sıralı & akış", "en", VisualPreset::Editorial),
        participants,
        messages,
    };
    let svg = SvgRenderer::render_sequence(&seq);
    assert_well_formed(&svg);
    assert!(svg.contains("stroke-dasharray=\"4,4\"") && svg.contains("calls f1&lt;T&gt;()"));
    let vb = svg.split("viewBox=\"0 0 ").nth(1).unwrap();
    let w: f32 = vb.split(' ').next().unwrap().parse().unwrap();
    assert!(w > 900.0, "9 participants need more than 900 wide, got {w}");
    // participant boxes never overlap
    let mut xs: Vec<(f32, f32)> = svg
        .split("class=\"box c-backend\" x=\"")
        .skip(1)
        .map(|s| {
            let x: f32 = s.split('"').next().unwrap().parse().unwrap();
            let w: f32 = s
                .split("width=\"")
                .nth(1)
                .unwrap()
                .split('"')
                .next()
                .unwrap()
                .parse()
                .unwrap();
            (x, w)
        })
        .collect();
    xs.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert_eq!(xs.len(), 9);
    for w in xs.windows(2) {
        assert!(w[0].0 + w[0].1 <= w[1].0 + 0.01);
    }
}

/// `ARCHIFY_SVG_OUT=<dir> cargo test -p archify-render write_examples` dumps sample pictures.
#[test]
fn write_examples() {
    let Some(dir) = std::env::var_os("ARCHIFY_SVG_OUT") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut d = reference_diagram();
    for (name, preset) in [
        ("classic", VisualPreset::Classic),
        ("editorial", VisualPreset::Editorial),
        ("blueprint", VisualPreset::Blueprint),
        ("signal-flow", VisualPreset::SignalFlow),
    ] {
        d.meta.visual_preset = preset;
        std::fs::write(
            dir.join(format!("svg-ornek-mimari-{name}.svg")),
            SvgRenderer::render(&d),
        )
        .unwrap();
    }
    std::fs::write(
        dir.join("svg-ornek-mimari-20.svg"),
        SvgRenderer::render(&grid_diagram(20)),
    )
    .unwrap();
    let html = wrap_html(&SvgRenderer::render(&d), "Paslı Beyin");
    std::fs::write(dir.join("svg-ornek-mimari.html"), html).unwrap();
}
