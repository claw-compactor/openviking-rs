use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ov_router::{route, RoutingProfile, config::default_routing_config};

fn bench_route_queries(c: &mut Criterion) {
    let config = default_routing_config();
    let queries = vec![
        ("what is rust?", None, "simple"),
        ("Write a function to sort a list using quicksort algorithm with O(n log n) complexity", None, "medium"),
        ("Prove that the halting problem is undecidable using a diagonalization argument step by step", None, "complex"),
        ("hello", None, "trivial"),
        ("Design a distributed microservice architecture for a real-time trading platform with kubernetes orchestration", Some("You are an expert software architect. Respond with structured JSON."), "complex_structured"),
        ("翻译这段话", None, "simple_zh"),
        ("Write a poem about the ocean", None, "creative"),
        ("Implement an async task scheduler with priority queues and backpressure in Rust", None, "technical"),
    ];

    c.bench_function("route_1000_mixed_queries", |b| {
        b.iter(|| {
            for _ in 0..125 {
                for (prompt, sys, _label) in &queries {
                    black_box(route(prompt, *sys, 4096, &config, RoutingProfile::Auto));
                }
            }
        })
    });

    c.bench_function("route_1000_simple", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                black_box(route("what is rust?", None, 256, &config, RoutingProfile::Eco));
            }
        })
    });

    c.bench_function("route_1000_complex", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                black_box(route(
                    "Design and implement a comprehensive distributed system",
                    Some("Expert architect. Respond in JSON."),
                    8192,
                    &config,
                    RoutingProfile::Premium,
                ));
            }
        })
    });
}

criterion_group!(benches, bench_route_queries);
criterion_main!(benches);
