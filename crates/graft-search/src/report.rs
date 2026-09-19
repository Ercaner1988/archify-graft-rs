//! Repo map ve grep raporlayıcıları. CLI (`archify-graft-cli map/grep`) ve
//! MCP sunucusu (`graft_repo_map`/`graft_find_all`) AYNI fonksiyonları
//! çağırır — iki yüzeyin farklı davranması riskini ortadan kaldırır.

use graft_model::{CodeGraph, NodeKind, NodeV1};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::Path;

struct DizinIstatistik {
    dosya: usize,
    sembol: usize,
    /// (isim, dosya_yolu, gelen_referans)
    hublar: Vec<(String, String, usize)>,
}

fn gelen_referanslar(graph: &CodeGraph) -> HashMap<&str, usize> {
    let mut gelen: HashMap<&str, usize> = HashMap::new();
    for e in &graph.edges {
        *gelen.entry(e.target.as_str()).or_insert(0) += 1;
    }
    gelen
}

/// Token-bütçeli depo özeti: dizin başına dosya/sembol sayısı + en çok
/// referans alan sembol ("hub"), sonra genel "hotspot" listesi.
///
/// Sıralama sinyali "gelen kenar sayısı" (relation farketmeksizin): şu an bu
/// ÇOĞUNLUKLA `Contains` (dosya→sembol) demek, `Calls` kenarları
/// extractor'ın yalnız bildirim SATIRINI taraması yüzünden az/gürültülü
/// (bkz. `graft-parser/src/resolver.rs`, ölçüldü: pasli-beyin/cekirdek'te
/// 480 kenarın 89'u Calls). Extractor gerçek çağrı çözümlemesi kazanınca bu
/// fonksiyon DEĞİŞMEDEN doğruluğu otomatik yükselir (girdi kalitesi sorunu,
/// algoritma değil).
///
/// BİLİNEN VERİ HATASI (extractor'da, burada değil): `struct Foo {` gibi
/// (açan parantez boşlukla ayrı) bildirimlerde isim çıkarımı boş string
/// üretiyor → aynı dosyada birden çok düğüm aynı id'yi paylaşıp çakışıyor.
/// Bu fonksiyon veriyi olduğu gibi yansıtır, gizlemez veya düzeltmez.
pub fn build_repo_map(graph: &CodeGraph, max_dirs: usize) -> String {
    let gelen = gelen_referanslar(graph);
    let mut out = String::new();

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

    let _ = writeln!(
        out,
        "repo map — {} dosya · {} sembol · {} kenar",
        dosya_sayisi,
        sembol_sayisi,
        graph.edges.len()
    );
    let _ = writeln!(out);

    let toplam_dizin = dizinler.len();
    for (i, (dizin, ist)) in dizinler.iter().enumerate() {
        if i >= max_dirs {
            let _ = writeln!(
                out,
                "… +{} dizin daha gösterilmedi (max_dirs ile genişlet)",
                toplam_dizin - max_dirs
            );
            break;
        }
        let mut hublar = ist.hublar.clone();
        hublar.sort_by_key(|h| std::cmp::Reverse(h.2));
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
        let _ = write!(
            out,
            "{:<45}{} dosya · {} sembol",
            if dizin.is_empty() { "(kök)" } else { dizin },
            ist.dosya,
            ist.sembol
        );
        if !ust.is_empty() {
            let _ = write!(out, "   hublar: {}", ust.join(", "));
        }
        let _ = writeln!(out);
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
    tum_semboller.sort_by_key(|s| std::cmp::Reverse(s.3));
    let hotspotlar: Vec<String> = tum_semboller
        .iter()
        .filter(|s| s.3 > 0)
        .take(12)
        .map(|(isim, tur, yol, refs)| {
            format!("{isim} · {tur:?} · {} · {refs}←", yol.replace('\\', "/"))
        })
        .collect();
    if !hotspotlar.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "hotspots: {}", hotspotlar.join("  "));
    }

    out
}

/// `skeleton` — bir dosyanın imza yüzeyi: o dosyaya ait Function/Method/
/// Class/Enum/Interface düğümlerinin bildirim satırları. Bu SEARCH_BODY
/// zaten tam bildirim metnidir (bkz. `extractor.rs`) — extractor'ın bilinen
/// isim-çıkarma hatasından (boşluklu `{` içeren bildirimlerde `name` boş
/// çıkması) ETKİLENMEZ, çünkü burada `name` değil ham satır gösteriliyor.
///
/// `dosya_yolu` eşlemesi: ters slash'e normalize edilip tam ya da SONEK
/// eşleşmesi aranır — "gomme.rs" ya da "src/gomme.rs" gibi kısaltılmış bir
/// yol da çalışır.
pub fn file_skeleton(graph: &CodeGraph, dosya_yolu: &str) -> String {
    let gelen = gelen_referanslar(graph);
    let hedef = dosya_yolu.replace('\\', "/");

    let eslesen: Vec<&NodeV1> = graph
        .nodes
        .iter()
        .filter(|n| n.kind != NodeKind::File)
        .filter(|n| {
            let yol = n.path.replace('\\', "/");
            yol == hedef || yol.ends_with(&hedef)
        })
        .collect();

    if eslesen.is_empty() {
        return format!("'{dosya_yolu}' eşleşen indekslenmiş dosya bulunamadı.\n");
    }

    let mut out = String::new();
    let tam_yol = eslesen[0].path.replace('\\', "/");
    let _ = writeln!(out, "{tam_yol} — {} sembol", eslesen.len());
    for node in eslesen {
        let refs = gelen.get(node.id.as_str()).copied().unwrap_or(0);
        let _ = writeln!(out, "  {:?}  {}   ({refs}←)", node.kind, node.search_body);
    }
    out
}

/// Regex arama, iki katman:
///  1) Bildirim-satırı eşleşmeleri: Function/Method/Class/vb. düğümlerin
///     `search_body`'si TEK SATIRLIK bildirim metnidir (bkz. `extractor.rs`)
///     — yani eşleşme "isim/imza örtüşüyor" demektir, gövde değil.
///  2) Dosya-içeriği eşleşmeleri: File düğümlerinin `file_residual`'ı (ilk
///     8000 karakter) satır satır taranır, GERÇEK satır numarası verilir.
///
/// "Kapsayan-sembole-göre-gruplama" (@nanonets/graft'ın yaptığı) YOK —
/// `span` alanı extractor tarafından hiç doldurulmuyor (`extractor.rs`: her
/// düğümde `span: None`), yani bir satırın hangi fonksiyonun içinde olduğu
/// bilinmiyor. `span` doldurulunca bu fonksiyon gruplamayı ekleyebilir.
pub fn grep_graph(graph: &CodeGraph, pattern: &str, ignore_case: bool) -> anyhow::Result<String> {
    let re = regex::RegexBuilder::new(pattern)
        .case_insensitive(ignore_case)
        .build()?;
    let gelen = gelen_referanslar(graph);
    let mut out = String::new();

    let mut bildirim_eslesme: Vec<(&NodeV1, usize)> = graph
        .nodes
        .iter()
        .filter(|n| n.kind != NodeKind::File && re.is_match(&n.search_body))
        .map(|n| (n, gelen.get(n.id.as_str()).copied().unwrap_or(0)))
        .collect();
    bildirim_eslesme.sort_by_key(|(_, refs)| std::cmp::Reverse(*refs));

    if !bildirim_eslesme.is_empty() {
        let _ = writeln!(
            out,
            "## bildirim satırı eşleşmeleri (isim/imza — gövde değil)"
        );
        for (node, refs) in &bildirim_eslesme {
            let _ = writeln!(
                out,
                "  {} · {:?} · {} · {refs}←",
                node.name,
                node.kind,
                node.path.replace('\\', "/")
            );
        }
        let _ = writeln!(out);
    }

    let _ = writeln!(
        out,
        "## dosya içeriği eşleşmeleri (satır numaralı, ilk 8000 karakterle sınırlı)"
    );
    let mut dosya_eslesme = 0usize;
    for node in &graph.nodes {
        if node.kind != NodeKind::File {
            continue;
        }
        let mut yol_yazildi = false;
        for (i, satir) in node.file_residual.lines().enumerate() {
            if re.is_match(satir) {
                if !yol_yazildi {
                    let _ = writeln!(out, "{}", node.path.replace('\\', "/"));
                    yol_yazildi = true;
                }
                let _ = writeln!(out, "  {}: {}", i + 1, satir.trim());
                dosya_eslesme += 1;
            }
        }
    }

    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "{} bildirim eşleşmesi, {} dosya-içeriği eşleşmesi",
        bildirim_eslesme.len(),
        dosya_eslesme
    );

    Ok(out)
}
