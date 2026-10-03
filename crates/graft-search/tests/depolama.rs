//! Graf önbelleği (`GraphStorage`): rkyv dosyası gidiş-dönüşte aynı grafı verir, eski ya da
//! bozuk dosya açık hata verir (sessizce boş graf döndürmez).

use graft_model::{CodeGraph, EdgeRelation, EdgeV1, NodeKind, NodeV1, Span};
use graft_search::GraphStorage;
use std::path::PathBuf;

fn dosya(ad: &str) -> PathBuf {
    std::env::temp_dir().join(format!("graft-depolama-{}-{ad}.bin", std::process::id()))
}

fn ornek() -> CodeGraph {
    let mut g = CodeGraph::new();
    for (id, kind) in [
        ("file:a.rs", NodeKind::File),
        ("a.rs:f", NodeKind::Function),
    ] {
        g.add_node(NodeV1 {
            id: id.into(),
            path: "a.rs".into(),
            name: id.rsplit([':', '/']).next().unwrap_or(id).into(),
            kind,
            span: Some(Span {
                start_line: 1,
                start_col: 0,
                end_line: 9,
                end_col: 1,
            }),
            search_body: "fn f() { işlem() }".into(),
            file_residual: String::new(),
        });
    }
    g.add_edge(EdgeV1 {
        source: "file:a.rs".into(),
        target: "a.rs:f".into(),
        relation: EdgeRelation::Contains,
        confidence: 0.75,
    });
    g.imports.insert("file:a.rs".into(), vec!["std::fs".into()]);
    g.facts
        .insert("a.rs:f".into(), vec!["c:işlem".into(), "loc:9".into()]);
    g
}

#[test]
fn graf_onbellegi_gidis_donus() {
    let yol = dosya("gidis");
    let g = ornek();
    GraphStorage::save(&g, &yol).unwrap();
    let y = GraphStorage::load_mmap(&yol).unwrap();
    std::fs::remove_file(&yol).ok();
    assert_eq!(y.nodes.len(), 2);
    assert_eq!(y.edges.len(), 1);
    assert_eq!(y.edges[0].confidence, 0.75);
    assert_eq!(y.imports, g.imports);
    assert_eq!(y.facts, g.facts);
    assert_eq!(y.nodes[1].search_body, "fn f() { işlem() }");
    assert!(
        y.get_node_by_id("a.rs:f").is_some(),
        "dizin yeniden kurulmalı"
    );
}

#[test]
fn eski_ya_da_bozuk_onbellek_acik_hata_verir() {
    let yol = dosya("eski");
    // bincode'un yazdığı türden bayt dizisi: imza yok.
    std::fs::write(
        &yol,
        [
            2u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
        ],
    )
    .unwrap();
    let e = GraphStorage::load_mmap(&yol).unwrap_err().to_string();
    assert!(e.contains("run `index` again"), "{e}");

    GraphStorage::save(&ornek(), &yol).unwrap();
    let mut b = std::fs::read(&yol).unwrap();
    let n = b.len();
    b[n - 1] ^= 0xFF;
    std::fs::write(&yol, b).unwrap();
    assert!(
        GraphStorage::load_mmap(&yol).is_err(),
        "doğrulanmayan yük reddedilmeli"
    );
    std::fs::remove_file(&yol).ok();
}
