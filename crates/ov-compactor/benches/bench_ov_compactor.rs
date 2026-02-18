use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ov_compactor::{CompactorPipeline, CompressionLevel};

fn generate_text(size_kb: usize) -> String {
    let base = "The quick brown fox jumps over the lazy dog. This is a test sentence for benchmarking text compression in OpenViking. We need realistic content that includes repetition, technical terms, and natural language patterns. The system processes context data through multiple layers of compression including JSONL cleanup, CCP abbreviation, dictionary encoding, deduplication via shingle hashing, and format optimization. ";
    let mut text = String::with_capacity(size_kb * 1024);
    while text.len() < size_kb * 1024 {
        text.push_str(base);
    }
    text.truncate(size_kb * 1024);
    text
}

fn generate_jsonl(size_kb: usize) -> String {
    let mut lines = Vec::new();
    let mut total = 0;
    let mut i = 0;
    while total < size_kb * 1024 {
        let line = format!(
            r#"{{"id":"{idx}","role":"user","content":"Message {idx} with content","timestamp":"2026-01-15T10:00:00Z"}}"#,
            idx = i
        );
        total += line.len() + 1;
        lines.push(line);
        i += 1;
    }
    lines.join("\n")
}

fn bench_compress_text(c: &mut Criterion) {
    let text_1k = generate_text(1);
    let text_10k = generate_text(10);
    let text_100k = generate_text(100);

    for &(name, level) in &[("lossless", CompressionLevel::Lossless), ("minimal", CompressionLevel::Minimal), ("balanced", CompressionLevel::Balanced)] {
        let pipeline = CompactorPipeline::new(level);
        let n = name;
        c.bench_function(&format!("compress_{n}_1kb"), |b| {
            b.iter(|| black_box(pipeline.compress(black_box(&text_1k))))
        });
        c.bench_function(&format!("compress_{n}_10kb"), |b| {
            b.iter(|| black_box(pipeline.compress(black_box(&text_10k))))
        });
        c.bench_function(&format!("compress_{n}_100kb"), |b| {
            b.iter(|| black_box(pipeline.compress(black_box(&text_100k))))
        });
    }
}

fn bench_compress_jsonl(c: &mut Criterion) {
    let jsonl_10k = generate_jsonl(10);
    let pipeline = CompactorPipeline::balanced();
    c.bench_function("compress_jsonl_10kb", |b| {
        b.iter(|| black_box(pipeline.compress(black_box(&jsonl_10k))))
    });
}

criterion_group!(benches, bench_compress_text, bench_compress_jsonl);
criterion_main!(benches);
