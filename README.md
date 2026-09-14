# Archify-Graft (Rust)

**Unified Pure-Rust Engine: Graft Codebase Intelligence & Archify Architecture Studio**

[![CI](https://github.com/Ercaner1988/archify-graft-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/Ercaner1988/archify-graft-rs/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Archify-Graft merges the deterministic code graph and personalized PageRank intelligence of **Graft** with the JSON-IR architecture modeling and signature neon visual aesthetics of **Archify**. Built 100% in native Rust, it completely eliminates Node.js, Bun, V8, and Python/uv runtime dependencies.

- **Zero-Runtime Overhead:** Single static binary with zero external dependencies.
- **Hardware-Deep Direct I/O:** Bypasses OS page cache using sector-aligned direct DMA (`FILE_FLAG_NO_BUFFERING` on Windows, `O_DIRECT` on Linux).
- **Trilingual Intelligence:** First-class tokenizer and normalizer for **Arabic**, **Turkish**, and **English**.
- **Micro-Crate Architecture:** 12 decoupled, focused micro-crates (< 230 LOC per file) connected via clean pipelines.
- **5 Diagram Types & Delta Engine:** Architecture, Sequence, Dataflow, Lifecycle, Workflow diagrams + before/after delta diff analysis.
- **Interactive Studio & Neon Aesthetics:** Hardware GPU-accelerated neon bloom rendering in `egui` (optional glow backend) and self-contained SVG export.
- **Authored Story Beats:** Step-through architectural storytelling and multi-phase tour playback.
- **Model Context Protocol (MCP):** High-speed native JSON-RPC server for AI agents (Claude Code, Google Antigravity, Cursor).

---

## Workspace Crates

- `lowlevel-sys`: Hardware direct DMA, cache line detection, core affinity, TSC profiling.
- `graft-model`: Core AST data models (`NodeV1`, `EdgeV1`, `CodeGraph`).
- `graft-i18n`: Trilingual tokenizer with Arabic diacritics stripping, Turkish locale safety, English subwords.
- `graft-parser`: Parallel file scanner, incremental FNV-1a hash index, and AST extractor.
- `graft-search`: Whole-file BM25 index and GraphRank (PageRank) search.
- `archify-ir`: JSON-IR schema types for 5 diagram types, delta diff models, and story beats.
- `archify-geometry`: Topologic routing, reachability (BFS) and route probing (A* shortest path).
- `archify-bridge`: AST-to-Architecture, Sequence call tracer, and Dataflow compiler.
- `archify-render`: Preset-aware Neon SVG / HTML export with Arabic RTL support and delta diff legend.
- `archify-delta`: Diff engine comparing before/after diagrams (Added/Removed/Modified).
- `archify-egui`: Pure-Rust `egui` interactive studio with neon bloom painter and story beat player.
- `graft-mcp`: Asynchronous Model Context Protocol stdio JSON-RPC server.
- `archify-graft-cli`: Command-line interface orchestrating the pipelines.

---

## Quick Start

```bash
# Index codebase incrementally with direct DMA I/O
cargo run -- index .

# Ask / Search with BM25 + GraphRank (Turkish, Arabic, English)
cargo run -- ask "direct DMA"

# Generate Neon Architecture Diagram (Turkish)
cargo run -- diagram --output architecture.svg --title "Sistem Mimarisi" --locale tr

# Generate Sequence Diagram (Call Trace)
cargo run -- diagram --type sequence --output sequence.svg --title "Çağrı İzi"

# Generate Dataflow Diagram (Pipelines)
cargo run -- diagram --type dataflow --output dataflow.svg --title "Veri Akışı"

# Generate Neon Architecture Diagram (Arabic RTL)
cargo run -- diagram --output architecture-ar.svg --title "استوديو بنية النظام" --locale ar

# Launch Interactive GUI Studio (egui + neon bloom canvas)
cargo run --features gui -- gui

# Start MCP server for AI agents
cargo run -- mcp
```
