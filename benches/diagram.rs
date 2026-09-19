//! Benchmarks for the diagram pipeline: AST-to-IR compilation, SVG rendering,
//! delta diffing and route probing (`archify-bridge`, `archify-render`,
//! `archify-delta`, `archify-geometry`).

#[path = "fixtures/mod.rs"]
mod fixtures;

use archify_bridge::GraftToArchifyBridge;
use archify_delta::DeltaEngine;
use archify_geometry::ReachabilityEngine;
use archify_render::SvgRenderer;

fn main() {
    divan::main();
}

const LOCALES: [&str; 3] = ["en", "tr", "ar"];

/// Compiles the AST graph into the architecture JSON-IR (layout + story beats).
#[divan::bench(args = LOCALES)]
fn compile_architecture(bencher: divan::Bencher, locale: &str) {
    let graph = fixtures::code_graph(12, 4);
    bencher.bench(|| {
        GraftToArchifyBridge::compile(
            divan::black_box(&graph),
            "Benchmark Architecture",
            divan::black_box(locale),
        )
    });
}

/// Dataflow IR compilation (stream pipelines).
#[divan::bench]
fn compile_dataflow(bencher: divan::Bencher) {
    let graph = fixtures::code_graph(12, 4);
    bencher.bench(|| {
        GraftToArchifyBridge::compile_dataflow(divan::black_box(&graph), "Benchmark Dataflow", "en")
    });
}

/// Sequence IR compilation (call tracing from a root function).
#[divan::bench]
fn compile_sequence(bencher: divan::Bencher) {
    let graph = fixtures::code_graph(12, 4);
    bencher.bench(|| {
        GraftToArchifyBridge::compile_sequence(
            divan::black_box(&graph),
            "parse_ast_node_0",
            "Benchmark Sequence",
            "en",
        )
    });
}

/// Neon SVG export of the architecture diagram, including Arabic RTL layout.
#[divan::bench(args = LOCALES)]
fn render_architecture_svg(bencher: divan::Bencher, locale: &str) {
    let graph = fixtures::code_graph(12, 4);
    let diagram = GraftToArchifyBridge::compile(&graph, "Benchmark Architecture", locale);
    bencher.bench(|| SvgRenderer::render(divan::black_box(&diagram)));
}

/// Dataflow SVG export.
#[divan::bench]
fn render_dataflow_svg(bencher: divan::Bencher) {
    let graph = fixtures::code_graph(12, 4);
    let diagram = GraftToArchifyBridge::compile_dataflow(&graph, "Benchmark Dataflow", "en");
    bencher.bench(|| SvgRenderer::render_dataflow(divan::black_box(&diagram)));
}

/// Sequence SVG export.
#[divan::bench]
fn render_sequence_svg(bencher: divan::Bencher) {
    let graph = fixtures::code_graph(12, 4);
    let diagram =
        GraftToArchifyBridge::compile_sequence(&graph, "parse_ast_node_0", "Benchmark Seq", "en");
    bencher.bench(|| SvgRenderer::render_sequence(divan::black_box(&diagram)));
}

/// Delta diffing of two architecture revisions (Added / Removed / Modified).
#[divan::bench]
fn compute_delta(bencher: divan::Bencher) {
    let before = fixtures::code_graph(12, 4);
    let after = fixtures::mutated_code_graph(12, 4);
    let before_diagram = GraftToArchifyBridge::compile(&before, "Delta", "en");
    let after_diagram = GraftToArchifyBridge::compile(&after, "Delta", "en");
    bencher.bench(|| {
        DeltaEngine::compute_delta(
            divan::black_box(&before_diagram),
            divan::black_box(&after_diagram),
        )
    });
}

/// Full graph-level delta: compile both revisions, then diff them.
#[divan::bench]
fn compute_graph_delta(bencher: divan::Bencher) {
    let before = fixtures::code_graph(12, 4);
    let after = fixtures::mutated_code_graph(12, 4);
    bencher.bench(|| {
        DeltaEngine::compute_graph_delta(
            divan::black_box(&before),
            divan::black_box(&after),
            "Delta",
            "en",
        )
    });
}

/// Delta SVG export with the diff legend.
#[divan::bench]
fn render_delta_svg(bencher: divan::Bencher) {
    let before = fixtures::code_graph(12, 4);
    let after = fixtures::mutated_code_graph(12, 4);
    let delta = DeltaEngine::compute_graph_delta(&before, &after, "Delta", "en");
    bencher.bench(|| SvgRenderer::render_delta(divan::black_box(&delta)));
}

/// A* route probing between two components of the architecture diagram.
#[divan::bench]
fn find_route(bencher: divan::Bencher) {
    let graph = fixtures::code_graph(12, 4);
    let diagram = GraftToArchifyBridge::compile(&graph, "Routing", "en");
    let start = diagram.components.first().map(|c| c.id.clone()).unwrap();
    let target = diagram.components.last().map(|c| c.id.clone()).unwrap();
    bencher.bench(|| {
        ReachabilityEngine::find_route(
            divan::black_box(&diagram),
            divan::black_box(&start),
            divan::black_box(&target),
        )
    });
}
