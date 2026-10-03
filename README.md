# Archify-Graft (Rust)

**Unified Pure-Rust Engine: Graft Codebase Intelligence & Archify Architecture Studio**

[![CI](https://github.com/Ercaner1988/archify-graft-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/Ercaner1988/archify-graft-rs/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![CodSpeed](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://app.codspeed.io/Ercaner1988/archify-graft-rs?utm_source=badge)

Archify-Graft merges the deterministic code graph and personalized PageRank intelligence of **Graft** with the JSON-IR architecture modeling and signature neon visual aesthetics of **Archify**. Built 100% in native Rust, it completely eliminates Node.js, Bun, V8, and Python/uv runtime dependencies.

- **Zero-Runtime Overhead:** Single static binary with zero external runtime dependencies.
- **Hardware-Deep Direct I/O:** Bypasses OS page cache using sector-aligned direct DMA (`FILE_FLAG_NO_BUFFERING` on Windows, `O_DIRECT` on Linux).
- **Trilingual Intelligence:** First-class tokenizer and normalizer for **Arabic**, **Turkish**, and **English**.
- **Multi-Language AST Support:** Deep body-scanning and call edge extraction for **Rust**, **JavaScript/TypeScript**, **Python**, **Go**, **Java**, and **C/C++**.
- **Zero-Copy Persistence:** Lightning-fast serialization using **rkyv** with pinned wire contracts for cross-engine interoperability.
- **Micro-Crate Architecture:** 24 decoupled micro-crates strictly complying with the 1500 LOC per crate constraint.
- **5 Diagram Types & Delta Engine:** Architecture, Sequence, Dataflow, Lifecycle, Workflow diagrams + before/after delta diff analysis.
- **Interactive Studio & Neon Aesthetics:** Hardware GPU-accelerated neon bloom rendering in `egui` via **wgpu** (DirectX 12 / Vulkan / Metal) and self-contained SVG export.
- **Authored Story Beats:** Step-through architectural storytelling and multi-phase tour playback.
- **Model Context Protocol (MCP):** High-speed native JSON-RPC server for AI agents (Claude Code, Google Antigravity, Cursor).

---

## Workspace Crates (24 Crates)

### Hardware & Core Model
- `lowlevel-sys`: Hardware direct DMA, cache line detection, core affinity, TSC profiling.
- `graft-model`: Core AST data models (`NodeV1`, `EdgeV1`, `CodeGraph`), rkyv zero-copy serialization, binary varint framing.
- `graft-i18n`: Trilingual tokenizer with Arabic diacritics stripping, Turkish locale safety, English subwords.

### Graft Code Intelligence & Extraction
- `graft-cargo`: Cargo workspace manifest parser and path-dependency resolver.
- `graft-rust`: Zero-dependency Rust source code scanner, tokenizer, comment/string masking, and call extractor.
- `graft-langs`: Multi-language AST item and call extractors (JS/TS, Python, Go, Java, C/C++).
- `graft-langs-link`: Cross-file call resolution and import linker for multilingual projects.
- `graft-parser`: Parallel Rayon file scanner, incremental FNV-1a hash index, and AST orchestrator.
- `graft-search`: Whole-file memory-mapped BM25 index and Personalized GraphRank (PageRank) search.

### Archify Visualization & Layout
- `archify-ir`: Strict JSON-IR schema types for 5 diagram types, delta diff models, and story beats.
- `archify-geometry`: Graph reachability (BFS) and topological route probing (A* shortest path).
- `archify-style`: Preset palettes (Editorial, Neon, Classic), themes, stroke styling, and text measurement.
- `archify-route`: Orthogonal edge routing, A* grid pathfinder, edge bus bundling, and lane nudging.
- `archify-layout`: Hierarchical Sugiyama/layered placement algorithm without third-party dependencies.
- `archify-motion`: Smooth animation curves (easing, energy beams, camera pan/zoom interpolation, tour steps).
- `archify-scene`: Spatial scene node builder, layered structure compiler, and tour sequencer.
- `archify-crates`: Workspace crate-level dependency hierarchy extractor and heuristic role labeling.
- `archify-bridge`: AST-to-Architecture, Sequence call tracer, and Dataflow pipeline compiler.
- `archify-render`: Preset-aware Neon SVG / HTML export with Arabic RTL support and delta diff legend.
- `archify-delta`: Diff engine comparing before/after diagrams (Added/Removed/Modified/Unchanged).

### Interactive Desktop Studio & Integration
- `archify-egui-paint`: Multi-pass neon bloom painter, glow shaders, and connection arc drawing.
- `archify-egui`: Interactive desktop studio (`egui` + `wgpu` DirectX 12/Vulkan), interactive canvas, search overlay.
- `graft-mcp`: Asynchronous Model Context Protocol stdio JSON-RPC server for AI coding agents.
- `archify-graft-cli`: Unified command-line interface orchestrating the entire pipeline.

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

# Launch Interactive GUI Studio (egui + wgpu DirectX 12 / Vulkan canvas)
cargo run --features gui -- gui

# Start MCP server for AI agents
cargo run -- mcp
```

---

## Benchmarks

Performance is tracked continuously with [CodSpeed](https://codspeed.io). The suites live in
`benches/` and cover the hot paths of the engine:

- `tokenizer`: trilingual tokenization and character folding (`graft-i18n`).
- `indexing`: FNV-1a content hashing, AST extraction, call resolution (`graft-parser`).
- `search`: BM25 index build/scoring and GraphRank propagation (`graft-search`).
- `diagram`: JSON-IR compilation, neon SVG export, delta diffing, A* routing
  (`archify-bridge`, `archify-render`, `archify-delta`, `archify-geometry`).

```bash
# Run locally with divan (walltime, human-readable output)
cargo bench

# Run through the CodSpeed CPU simulation instrument
cargo codspeed build --measurement-mode simulation
codspeed run --mode simulation -- cargo codspeed run
```
