use criterion::{black_box, criterion_group, criterion_main, Criterion};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use ov_server::{app_with_state, state::AppState};
use ov_core::context::{Context, ContextType};
use tokio::runtime::Runtime;

fn bench_http_health(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    c.bench_function("http_health_1000", |b| {
        b.iter(|| {
            rt.block_on(async {
                for _ in 0..1000 {
                    let app = app_with_state(AppState::new());
                    let req = Request::builder()
                        .uri("/health")
                        .body(Body::empty())
                        .unwrap();
                    let resp = app.oneshot(req).await.unwrap();
                    black_box(resp.status());
                }
            })
        })
    });
}

fn bench_http_context_crud(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("http_create_list_contexts_100", |b| {
        b.iter(|| {
            rt.block_on(async {
                let state = AppState::new();
                // Insert 100 contexts
                for i in 0..100 {
                    let app = app_with_state(state.clone());
                    let body = serde_json::json!({
                        "uri": format!("viking://memory/bench/{i}"),
                        "abstract": format!("Benchmark context {i}"),
                        "context_type": "memory",
                        "category": "benchmark"
                    });
                    let req = Request::builder()
                        .method("POST")
                        .uri("/api/v1/contexts")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&body).unwrap()))
                        .unwrap();
                    let resp = app.oneshot(req).await.unwrap();
                    black_box(resp.status());
                }
                // List
                let app = app_with_state(state.clone());
                let req = Request::builder()
                    .uri("/api/v1/contexts")
                    .body(Body::empty())
                    .unwrap();
                let resp = app.oneshot(req).await.unwrap();
                black_box(resp.status());
            })
        })
    });
}

criterion_group!(benches, bench_http_health, bench_http_context_crud);
criterion_main!(benches);
