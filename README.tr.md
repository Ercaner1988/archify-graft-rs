# Archify-Graft (Rust) — Türkçe Dokümantasyon

**Graft Kod Zekası ve Archify Mimari Görselleştiricisini Birleştiren Saf Rust Motoru**

[![CI](https://github.com/Ercaner1988/archify-graft-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/Ercaner1988/archify-graft-rs/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![CodSpeed](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://app.codspeed.io/Ercaner1988/archify-graft-rs?utm_source=badge)

Archify-Graft; **Graft**'ın deterministik AST kod grafı ve Kişiselleştirilmiş PageRank (GraphRank) yeteneklerini, **Archify**'ın JSON-IR mimari modelleme ve imza neon görsel estetiğiyle harmanlar. Sistem %100 saf Rust ile yazılmış olup Node.js, Bun, V8 ve Python/uv bağımlılıklarını tamamen ortadan kaldırır.

---

## Temel Yetenekler

1. **Sıfır Çalışma Zamanı (Zero-Runtime):** Tek bir yerel ikili dosya (`single static binary`); harici runtime ve sanal makine gerektirmez.
2. **Doğrudan Donanım & DMA G/Ç:** İşletim sistemi sayfa önbelleğini atlayan doğrudan sektör hizalı (4096-byte) unbuffered disk G/Ç'si (Windows'ta `FILE_FLAG_NO_BUFFERING`, Linux'ta `O_DIRECT`).
3. **Üç Dilli Zeka (TR / AR / EN):** Türkçe (noktalı/noktasız ı/i güvenliği), Arapça (hareke/tashkeel temizleme, Elif normalizasyonu, tam RTL desteği) ve İngilizce (camelCase/snake_case) normalizasyon ve belirteçleme.
4. **Çoklu Dil AST Çıkarımı:** Rust, JavaScript/TypeScript, Python, Go, Java ve C/C++ için gövde taramalı sembol ve çağrı kenarı çıkarımı.
5. **Sıfır-Kopya İkili Depolama:** `rkyv 0.8` ile ultra hızlı ikili serileştirme ve kanonik varint çerçeveleme.
6. **24 Mikro-Derme Mimarisi:** Altın-kapı 1500 kod satırı kuralına tam uyumlu, temiz boru hatlarıyla birbirine bağlı 24 derme.
7. **5 Diyagram Türü & Delta Analizi:** Mimari (Architecture), Sıralama (Sequence), Veri Akışı (Dataflow), Yaşam Döngüsü (Lifecycle), İş Akışı (Workflow) diyagramları ve önce/sonra delta diff karşılaştırması.
8. **İnteraktif Stüdyo & GPU Neon Aydınlatma:** `egui` ve `wgpu` (DirectX 12 / Vulkan / Metal) ile donanım hızlandırmalı çok katmanlı additive neon parlama (`NeonPainter`) ve bağımsız SVG/HTML ihracı.
9. **Yazar Destekli Hikaye Adımları (Story Beats):** Mimariyi adım adım gezen interaktif tur ve animasyonlu rota sunumu.
10. **Model Context Protocol (MCP):** Yapay zeka ajanları (Claude Code, Antigravity, Cursor) için yüksek hızlı yerel JSON-RPC stdio sunucusu.

---

## Çalışma Alanı Dermeleri (24 Derme)

### Donanım ve Çekirdek Model
- `lowlevel-sys`: Doğrudan DMA G/Ç, önbellek satır tespiti, çekirdek ilgisi ve TSC sayacı.
- `graft-model`: Çekirdek AST veri modelleri (`NodeV1`, `EdgeV1`, `CodeGraph`), rkyv sıfır-kopya ikili depolama.
- `graft-i18n`: Türkçe, Arapça ve İngilizce normalizasyon ve belirteçleme motoru.

### Kod Analizi ve Çıkarım
- `graft-cargo`: Cargo.toml ayrıştırma ve çalışma alanı yol bağımlılık çözümleyicisi.
- `graft-rust`: Saf Rust kaynak tarayıcısı, yorum/dizge maskeleme ve çağrı çıkarıcı.
- `graft-langs`: Çok dilli AST öğe ve çağrı çıkarıcıları (JS/TS, Python, Go, Java, C/C++).
- `graft-langs-link`: Diller arası import ve çağrı bağlantı çözümleyicisi.
- `graft-parser`: Rayon tabanlı paralel dosya tarayıcı, artımlı FNV-1a hash indeksi ve AST yöneticisi.
- `graft-search`: Bellek haritalı BM25 sözcüksel indeksi ve Kişiselleştirilmiş GraphRank (PageRank).

### Archify Görselleştirme ve Yerleşim
- `archify-ir`: 5 diyagram türü, delta modelleri ve hikaye adımları için katı JSON-IR şemaları.
- `archify-geometry`: Graf erişilebilirliği (BFS) ve topolojik A* en kısa rota tarayıcısı.
- `archify-style`: Önayarlı paletler (Editorial, Neon, Classic), temalar ve vuruş biçimleri.
- `archify-route`: Dik açılı kenar yönlendirme, A* ızgara bulucu, kenar yolu demetleme ve şerit itme.
- `archify-layout`: Bağımsız Sugiyama katmanlı kutu yerleştirme algoritması.
- `archify-motion`: Yumuşak animasyon eğrileri (enerji ışınları, kamera pan/zoom enterpolasyonu, tur adımları).
- `archify-scene`: Mekansal sahne düğüm oluşturucu, katmanlı yapı derleyici ve tur sıralayıcı.
- `archify-crates`: Çalışma alanı derme bağımlılık hiyerarşisi çıkarımı ve rol etiketleme.
- `archify-bridge`: AST'den Mimari, Sıralama ve Veri Akışı diyagram derleyicisi.
- `archify-render`: 4 önayarlı Neon SVG / HTML ihracı, Arapça RTL ve delta lejantı.
- `archify-delta`: İki diyagram arasındaki farkları (eklenen, silinen, değişen) hesaplayan diff motoru.

### Masaüstü Arayüzü ve Entegrasyon
- `archify-egui-paint`: Çok geçişli neon ışıma boyayıcısı, parlama gölgelendiricileri ve bağlantı yayları.
- `archify-egui`: İnteraktif masaüstü stüdyosu (`egui` + `wgpu`), tuval yönetimi ve arama katmanı.
- `graft-mcp`: Yapay zeka ajanları için asenkron Model Context Protocol JSON-RPC sunucusu.
- `archify-graft-cli`: Tüm boru hatlarını yöneten kök komut satırı arayüzü.

---

## Hızlı Başlangıç

```powershell
# Kod tabanını artımlı doğrudan DMA ile indeksleme
cargo run -- index .

# Kod tabanında arama yapma (TR / AR / EN)
cargo run -- ask "direct DMA"

# Türkçe neon mimari haritası oluşturma
cargo run -- diagram --output mimari.svg --title "Sistem Mimarisi" --locale tr

# Sıralama (Sequence) çağrı izi diyagramı oluşturma
cargo run -- diagram --type sequence --output sira.svg --title "Çağrı İzi"

# Veri Akışı (Dataflow) boru hattı diyagramı oluşturma
cargo run -- diagram --type dataflow --output akis.svg --title "Veri Akışı"

# Arapça RTL neon mimari haritası oluşturma
cargo run -- diagram --output mimari-ar.svg --title "استوديو بنية النظام" --locale ar

# İnteraktif Masaüstü Stüdyosunu Başlatma (egui + wgpu DirectX 12 / Vulkan)
cargo run --features gui -- gui

# Yapay zeka ajanları için MCP sunucusunu başlatma
cargo run -- mcp

# Tüm çalışma alanı testlerini koşturma
cargo test --workspace --all-targets
```
