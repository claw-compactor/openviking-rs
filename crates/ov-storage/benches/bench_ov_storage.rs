use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ov_storage::viking_fs::VikingFS;
use ov_core::context::Context;
use tempfile::TempDir;
use tokio::runtime::Runtime;

fn bench_viking_fs_write_read(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let data_1k = vec![b'A'; 1024];
    let data_100k = vec![b'B'; 100 * 1024];
    let data_1m = vec![b'C'; 1024 * 1024];

    c.bench_function("vfs_write_read_1kb_x100", |b| {
        let tmp = TempDir::new().unwrap();
        let vfs = VikingFS::new(tmp.path());
        b.iter(|| {
            rt.block_on(async {
                for i in 0..100 {
                    let uri = format!("viking://bench/file_{i}");
                    vfs.write(&uri, &data_1k).await.unwrap();
                    black_box(vfs.read(&uri).await.unwrap());
                }
            })
        })
    });

    c.bench_function("vfs_write_read_100kb_x50", |b| {
        let tmp = TempDir::new().unwrap();
        let vfs = VikingFS::new(tmp.path());
        b.iter(|| {
            rt.block_on(async {
                for i in 0..50 {
                    let uri = format!("viking://bench/file_{i}");
                    vfs.write(&uri, &data_100k).await.unwrap();
                    black_box(vfs.read(&uri).await.unwrap());
                }
            })
        })
    });

    c.bench_function("vfs_write_read_1mb_x10", |b| {
        let tmp = TempDir::new().unwrap();
        let vfs = VikingFS::new(tmp.path());
        b.iter(|| {
            rt.block_on(async {
                for i in 0..10 {
                    let uri = format!("viking://bench/file_{i}");
                    vfs.write(&uri, &data_1m).await.unwrap();
                    black_box(vfs.read(&uri).await.unwrap());
                }
            })
        })
    });
}

fn bench_context_serde(c: &mut Criterion) {
    let ctx = Context::builder("viking://memory/bench/test")
        .abstract_text("Benchmark context for serialization testing")
        .context_type(ov_core::context::ContextType::Memory)
        .category("benchmark")
        .build();
    let json = serde_json::to_vec(&ctx).unwrap();

    c.bench_function("context_to_bytes_1000", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                black_box(serde_json::to_vec(&ctx).unwrap());
            }
        })
    });

    c.bench_function("context_from_bytes_1000", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                let c: Context = serde_json::from_slice(black_box(&json)).unwrap();
                black_box(c);
            }
        })
    });
}

fn bench_uri_conversion(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    let vfs = VikingFS::new(tmp.path());

    c.bench_function("uri_to_path_10000", |b| {
        b.iter(|| {
            for i in 0..10000 {
                black_box(vfs.uri_to_path(&format!("viking://scope/memory/item_{i}")));
            }
        })
    });
}

criterion_group!(benches, bench_viking_fs_write_read, bench_context_serde, bench_uri_conversion);
criterion_main!(benches);
