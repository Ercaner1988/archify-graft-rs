# Archify-Graft (Rust)

**Unified Pure-Rust Engine: Graft Codebase Intelligence & Archify Architecture Studio**

[![CI](https://github.com/Ercaner1988/archify-graft-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/Ercaner1988/archify-graft-rs/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Archify-Graft merges the deterministic code graph and personalized PageRank intelligence of **Graft** with the JSON-IR architecture modeling and signature neon visual aesthetics of **Archify**. Built 100% in native Rust, it completely eliminates Node.js, Bun, V8, and Python/uv runtime dependencies.

- **Zero-Runtime Overhead:** Single static binary with zero external dependencies.
- **Hardware-Deep Direct I/O:** Bypasses OS page cache using sector-aligned direct DMA (`FILE_FLAG_NO_BUFFERING` on Windows, `O_DIRECT` on Linux).
- **Trilingual Intelligence:** First-class tokenizer and normalizer for **Arabic**, **Turkish**, and **English**.
- **Micro-Crate Architecture:** 11 decoupled, focused crates (< 250 LOC each) connected via clean pipelines.
- **Interactive Studio & Neon Aesthetics:** Hardware GPU-accelerated neon bloom rendering in `egui` and self-contained SVG export.
- **Model Context Protocol (MCP):** High-speed native JSON-RPC server for AI agents (Claude Code, Google Antigravity, Cursor).

---

## Workspace Crates

- `lowlevel-sys`: Hardware direct DMA, cache line detection, core affinity, TSC profiling.
- `graft-model`: Core AST data models (`NodeV1`, `EdgeV1`, `CodeGraph`).
- `graft-i18n`: Trilingual tokenizer with Arabic diacritics stripping, Turkish locale safety, English subwords.
- `graft-parser`: Parallel file scanner and AST extractor.
- `graft-search`: Whole-file BM25 index and GraphRank (PageRank) search.
- `archify-ir`: JSON-IR schema types and 7 semantic roles.
- `archify-geometry`: Topologic routing, reachability (BFS) and route probing (Dijkstra).
- `archify-bridge`: AST-to-Architecture diagram compiler.
- `archify-render`: Neon SVG / HTML export with Arabic RTL support.
- `archify-egui`: Pure-Rust `egui` interactive studio with neon bloom painter.
- `graft-mcp`: Asynchronous Model Context Protocol stdio JSON-RPC server.
- `archify-graft-cli`: Command-line interface orchestrating the pipelines.

---

## Quick Start

```bash
# Index codebase
cargo run -- index .

# Ask / Search with BM25 + GraphRank
cargo run -- ask "direct DMA"

# Generate Neon Architecture Diagram (Turkish)
cargo run -- diagram --output architecture.svg --title "Sistem Mimarisi" --locale tr

# Generate Neon Architecture Diagram (Arabic RTL)
cargo run -- diagram --output architecture-ar.svg --title "استوديو بنية النظام" --locale ar

# Start MCP server for AI agents
cargo run -- mcp
```
