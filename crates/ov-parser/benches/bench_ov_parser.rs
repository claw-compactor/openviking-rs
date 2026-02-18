use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ov_parser::{TextParser, MarkdownParser, TextChunker};
use ov_parser::traits::DocumentParser;

fn generate_text(size_kb: usize) -> String {
    let sentences = [
        "The quick brown fox jumps over the lazy dog.",
        "OpenViking provides agent-native context management.",
        "Rust offers memory safety without garbage collection.",
        "Vector databases enable semantic similarity search.",
        "This is a longer sentence that contains more words and provides additional context for testing the parser and chunker implementations in the OpenViking project.",
    ];
    let mut text = String::with_capacity(size_kb * 1024);
    let mut i = 0;
    while text.len() < size_kb * 1024 {
        text.push_str(sentences[i % sentences.len()]);
        text.push(' ');
        if i % 5 == 4 { text.push_str("\n\n"); }
        i += 1;
    }
    text.truncate(size_kb * 1024);
    text
}

fn generate_markdown(size_kb: usize) -> String {
    let mut md = String::with_capacity(size_kb * 1024);
    let mut section = 0;
    while md.len() < size_kb * 1024 {
        md.push_str(&format!("# Section {}\n\n", section));
        md.push_str("This is a paragraph with **bold** and *italic* text.\n\n");
        md.push_str("```rust\nfn main() {\n    println!(\"hello\");\n}\n```\n\n");
        md.push_str("- Item 1\n- Item 2\n- Item 3\n\n");
        md.push_str("| Col A | Col B |\n|-------|-------|\n| val1  | val2  |\n\n");
        section += 1;
    }
    md.truncate(size_kb * 1024);
    md
}

fn bench_text_parser(c: &mut Criterion) {
    let parser = TextParser::new();
    let text_10k = generate_text(10);
    let text_100k = generate_text(100);

    c.bench_function("text_parse_10kb", |b| {
        b.iter(|| black_box(parser.parse_content(black_box(&text_10k)).unwrap()))
    });
    c.bench_function("text_parse_100kb", |b| {
        b.iter(|| black_box(parser.parse_content(black_box(&text_100k)).unwrap()))
    });
}

fn bench_markdown_parser(c: &mut Criterion) {
    let parser = MarkdownParser::new();
    let md_10k = generate_markdown(10);
    let md_100k = generate_markdown(100);

    c.bench_function("markdown_parse_10kb", |b| {
        b.iter(|| black_box(parser.parse_content(black_box(&md_10k)).unwrap()))
    });
    c.bench_function("markdown_parse_100kb", |b| {
        b.iter(|| black_box(parser.parse_content(black_box(&md_100k)).unwrap()))
    });
}

fn bench_chunker(c: &mut Criterion) {
    let text_10k = generate_text(10);
    let text_100k = generate_text(100);
    let chunker = TextChunker::new(512, 50);

    c.bench_function("chunk_fixed_10kb", |b| {
        b.iter(|| black_box(chunker.chunk_fixed(black_box(&text_10k))))
    });
    c.bench_function("chunk_fixed_100kb", |b| {
        b.iter(|| black_box(chunker.chunk_fixed(black_box(&text_100k))))
    });
    c.bench_function("chunk_semantic_10kb", |b| {
        b.iter(|| black_box(chunker.chunk_semantic(black_box(&text_10k))))
    });
    c.bench_function("chunk_semantic_100kb", |b| {
        b.iter(|| black_box(chunker.chunk_semantic(black_box(&text_100k))))
    });
}

criterion_group!(benches, bench_text_parser, bench_markdown_parser, bench_chunker);
criterion_main!(benches);
