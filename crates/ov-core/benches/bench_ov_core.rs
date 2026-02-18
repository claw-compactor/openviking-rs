use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ov_core::context::{Context, ContextType};
use ov_core::tree::BuildingTree;
use ov_core::config::OpenVikingConfig;

fn bench_context_creation(c: &mut Criterion) {
    c.bench_function("context_creation_1000", |b| {
        b.iter(|| {
            for i in 0..1000 {
                let ctx = Context::builder(format!("viking://memory/test/{i}"))
                    .abstract_text(format!("Test context {i} with some meaningful text"))
                    .context_type(ContextType::Memory)
                    .category("preferences")
                    .build();
                black_box(ctx);
            }
        })
    });
}

fn bench_tree_operations(c: &mut Criterion) {
    c.bench_function("tree_add_1000_contexts", |b| {
        b.iter(|| {
            let mut tree = BuildingTree::new();
            for i in 0..1000 {
                let ctx = Context::builder(format!("viking://memory/test/{i}"))
                    .abstract_text(format!("Context {i}"))
                    .context_type(ContextType::Memory)
                    .build();
                tree.add_context(ctx);
            }
            tree.set_root("viking://memory/test/0");
            black_box(tree);
        })
    });

    let mut tree = BuildingTree::new();
    for i in 0..10000 {
        let ctx = Context::builder(format!("viking://memory/test/{i}"))
            .abstract_text(format!("Context {i}"))
            .context_type(ContextType::Memory)
            .build();
        tree.add_context(ctx);
    }
    tree.set_root("viking://memory/test/0");

    c.bench_function("tree_lookup_1000", |b| {
        b.iter(|| {
            for i in 0..1000 {
                black_box(tree.get(&format!("viking://memory/test/{i}")));
            }
        })
    });
}

fn bench_config_parsing(c: &mut Criterion) {
    let json_str = serde_json::to_string(&OpenVikingConfig::default()).unwrap();
    c.bench_function("config_parse_1000", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                let cfg: OpenVikingConfig = serde_json::from_str(black_box(&json_str)).unwrap();
                black_box(cfg);
            }
        })
    });

    c.bench_function("config_serialize_1000", |b| {
        let cfg = OpenVikingConfig::default();
        b.iter(|| {
            for _ in 0..1000 {
                black_box(serde_json::to_string(black_box(&cfg)).unwrap());
            }
        })
    });
}

fn bench_context_serialization(c: &mut Criterion) {
    let ctx = Context::builder("viking://memory/test/benchmark")
        .abstract_text("A comprehensive test context for serialization benchmarks")
        .context_type(ContextType::Memory)
        .category("preferences")
        .build();
    let json = serde_json::to_string(&ctx).unwrap();

    c.bench_function("context_serialize_1000", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                black_box(serde_json::to_string(&ctx).unwrap());
            }
        })
    });

    c.bench_function("context_deserialize_1000", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                let c: Context = serde_json::from_str(black_box(&json)).unwrap();
                black_box(c);
            }
        })
    });
}

criterion_group!(benches, bench_context_creation, bench_tree_operations, bench_config_parsing, bench_context_serialization);
criterion_main!(benches);
