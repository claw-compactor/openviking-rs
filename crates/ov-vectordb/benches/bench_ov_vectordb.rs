use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ov_vectordb::index::{FlatIndex, HnswIndex, VectorIndex};
use ov_vectordb::distance::DistanceMetric;
use ov_vectordb::store::{MemoryKvStore, KvStore};
use rand::Rng;

fn random_vector(dim: usize) -> Vec<f32> {
    let mut rng = rand::thread_rng();
    (0..dim).map(|_| rng.gen::<f32>()).collect()
}

fn bench_flat_insert(c: &mut Criterion) {
    let dim = 128;
    c.bench_function("flat_insert_1k_128d", |b| {
        b.iter(|| {
            let idx = FlatIndex::with_capacity(dim, DistanceMetric::Cosine, 1000);
            for i in 0..1000u64 {
                idx.insert(i, &random_vector(dim)).unwrap();
            }
            black_box(&idx);
        })
    });

    c.bench_function("flat_insert_10k_128d", |b| {
        b.iter(|| {
            let idx = FlatIndex::with_capacity(dim, DistanceMetric::Cosine, 10000);
            for i in 0..10000u64 {
                idx.insert(i, &random_vector(dim)).unwrap();
            }
            black_box(&idx);
        })
    });
}

fn bench_flat_search(c: &mut Criterion) {
    let dim = 128;
    let idx = FlatIndex::with_capacity(dim, DistanceMetric::Cosine, 10000);
    for i in 0..10000u64 {
        idx.insert(i, &random_vector(dim)).unwrap();
    }

    c.bench_function("flat_search_top10_from_10k", |b| {
        let query = random_vector(dim);
        b.iter(|| {
            black_box(idx.search(&query, 10).unwrap());
        })
    });
}

fn bench_hnsw_insert(c: &mut Criterion) {
    let dim = 128;
    c.bench_function("hnsw_insert_1k_128d", |b| {
        b.iter(|| {
            let idx = HnswIndex::new(dim, DistanceMetric::Cosine);
            for i in 0..1000u64 {
                idx.insert(i, &random_vector(dim)).unwrap();
            }
            black_box(&idx);
        })
    });
}

fn bench_hnsw_search(c: &mut Criterion) {
    let dim = 128;
    let idx = HnswIndex::new(dim, DistanceMetric::Cosine);
    for i in 0..10000u64 {
        idx.insert(i, &random_vector(dim)).unwrap();
    }

    c.bench_function("hnsw_search_top10_from_10k", |b| {
        let query = random_vector(dim);
        b.iter(|| {
            black_box(idx.search(&query, 10).unwrap());
        })
    });
}

fn bench_kv_store(c: &mut Criterion) {
    c.bench_function("kv_put_get_1000", |b| {
        b.iter(|| {
            let store = MemoryKvStore::new();
            for i in 0..1000 {
                store.put(&format!("key_{i}"), vec![0u8; 256]);
            }
            for i in 0..1000 {
                black_box(store.get(&format!("key_{i}")));
            }
        })
    });

    // Pre-populated store
    let store = MemoryKvStore::new();
    for i in 0..10000 {
        store.put(&format!("key_{i}"), vec![0u8; 256]);
    }
    c.bench_function("kv_contains_10k", |b| {
        b.iter(|| {
            for i in 0..1000 {
                black_box(store.contains(&format!("key_{i}")));
            }
        })
    });
}

criterion_group!(benches, bench_flat_insert, bench_flat_search, bench_hnsw_insert, bench_hnsw_search, bench_kv_store);
criterion_main!(benches);
