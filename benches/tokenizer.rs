//! Benchmarks for the trilingual tokenizer and normalizer (`graft-i18n`).

#[path = "fixtures/mod.rs"]
mod fixtures;

use graft_i18n::{normalize_trilingual, TrilingualTokenizer};

fn main() {
    divan::main();
}

const LANGUAGES: [&str; 3] = ["english", "turkish", "arabic"];

/// Tokenizes a mid-sized corpus: NFC normalization, camelCase splitting and
/// per-language character folding.
#[divan::bench(args = LANGUAGES)]
fn tokenize_corpus(bencher: divan::Bencher, lang: &str) {
    let text = fixtures::corpus(lang, 64);
    bencher.bench(|| TrilingualTokenizer::tokenize(divan::black_box(&text)));
}

/// Tokenizer throughput scaling on the mixed-script corpus.
#[divan::bench(args = [8, 64, 256])]
fn tokenize_scaling(bencher: divan::Bencher, repeat: usize) {
    let text = format!(
        "{}\n{}\n{}",
        fixtures::corpus("english", repeat),
        fixtures::corpus("turkish", repeat),
        fixtures::corpus("arabic", repeat),
    );
    bencher.bench(|| TrilingualTokenizer::tokenize(divan::black_box(&text)));
}

/// Character folding only (Arabic diacritics stripping, Turkish dotted/dotless
/// `i` handling, Unicode lowercasing).
#[divan::bench(args = LANGUAGES)]
fn normalize(bencher: divan::Bencher, lang: &str) {
    let text = fixtures::corpus(lang, 64);
    bencher.bench(|| normalize_trilingual(divan::black_box(&text)));
}

/// Tokenizing identifier-heavy source code, the dominant workload while indexing.
#[divan::bench]
fn tokenize_source_code(bencher: divan::Bencher) {
    let source = fixtures::source_file(0, 24);
    bencher.bench(|| TrilingualTokenizer::tokenize(divan::black_box(&source)));
}
