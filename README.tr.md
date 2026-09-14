# Archify-Graft (Rust) — Türkçe Dokümantasyon

**Graft Kod Zekası ve Archify Mimari Görselleştiricisini Birleştiren Saf Rust Motoru**

Archify-Graft; **Graft**'ın deterministik AST kod grafı ve Kişiselleştirilmiş PageRank (GraphRank) yeteneklerini, **Archify**'ın JSON-IR mimari modelleme ve imza neon görsel estetiğiyle harmanlar. Sistem %100 saf Rust ile yazılmış olup Node.js, Bun, V8 ve Python/uv bağımlılıklarını tamamen ortadan kaldırır.

---

## Temel Yetenekler

1. **Sıfır Çalışma Zamanı (Zero-Runtime):** Tek bir yerel ikili dosya (`single static binary`); harici çalışma zamanı kütüphanelerine ihtiyaç duymaz.
2. **Doğrudan Donanım & DMA G/Ç:** İşletim sistemi sayfa önbelleğini baypas eden doğrudan sektör hizalı (4096-byte) unbuffered disk G/Ç'si (Windows'ta `FILE_FLAG_NO_BUFFERING`, Linux'ta `O_DIRECT`).
3. **Üç Dilli Zeka (TR / AR / EN):** Türkçe (noktalı/noktasız ı/i güvenliği), Arapça (hareke/tashkeel soyma, Elif normalizasyonu, tam RTL) ve İngilizce için özel SIMD tokenizer.
4. **Mikro-Derme Mimarisi:** 1000+ satırlık monolitler yerine her biri 100-250 satırlık, birbirinden bağımsız 11 mikro-derme.
5. **İnteraktif Stüdyo & Neon Işıklandırma:** `egui` üzerinde çok katmanlı additive neon parlama (`NeonPainter`) ve SVG ihracı.
6. **Model Context Protocol (MCP):** Yapay zeka ajanları (Claude Code, Antigravity, Cursor) için yüksek hızlı yerel JSON-RPC stdio sunucusu.

---

## Çalıştırma Komutları

```powershell
# Projeyi indeksleme
cargo run -- index .

# Kod tabanında arama yapma
cargo run -- ask "direct DMA"

# Türkçe neon mimari haritası çıkarma
cargo run -- diagram --output mimari.svg --title "Sistem Mimarisi" --locale tr

# Arapça RTL neon mimari haritası çıkarma
cargo run -- diagram --output mimari-ar.svg --title "استوديو بنية النظام" --locale ar

# Yapay zeka ajanları için MCP sunucusunu başlatma
cargo run -- mcp

# Tüm testleri koşturma
cargo test --workspace
```
