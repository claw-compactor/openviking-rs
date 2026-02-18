use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ov_session::session::{Session, Message, Part, Role};
use ov_session::manager::SessionManager;

fn bench_session_create_close(c: &mut Criterion) {
    c.bench_function("session_create_1000", |b| {
        b.iter(|| {
            let mgr = SessionManager::new();
            for i in 0..1000 {
                black_box(mgr.create(format!("user_{i}")));
            }
        })
    });

    c.bench_function("session_create_close_1000", |b| {
        b.iter(|| {
            let mgr = SessionManager::new();
            for i in 0..1000 {
                let s = mgr.create(format!("user_{i}"));
                mgr.close(&s.id);
            }
        })
    });
}

fn bench_add_messages(c: &mut Criterion) {
    c.bench_function("session_add_100_messages", |b| {
        b.iter(|| {
            let mut session = Session::new("bench_user");
            for i in 0..100 {
                session.add_message(Role::User, vec![Part::text(format!("Message {i}: What is the meaning of life? This is a longer message to simulate realistic conversation patterns."))]);
                session.add_message(Role::Assistant, vec![Part::text(format!("Response {i}: The meaning of life is a philosophical question that has been debated for centuries. Here is a comprehensive answer with multiple paragraphs."))]);
            }
            black_box(&session);
        })
    });
}

fn bench_jsonl_roundtrip(c: &mut Criterion) {
    // Build a session with messages
    let mut session = Session::new("bench_user");
    for i in 0..50 {
        session.add_message(Role::User, vec![Part::text(format!("User message {i}"))]);
        session.add_message(Role::Assistant, vec![Part::text(format!("Assistant response {i}"))]);
    }

    c.bench_function("message_to_jsonl_1000", |b| {
        let msg = &session.messages[0];
        b.iter(|| {
            for _ in 0..1000 {
                black_box(msg.to_jsonl());
            }
        })
    });

    let jsonl = session.messages[0].to_jsonl();
    c.bench_function("message_from_jsonl_1000", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                black_box(Message::from_jsonl(&jsonl).unwrap());
            }
        })
    });

    // Session serialization
    let session_json = serde_json::to_string(&session).unwrap();
    c.bench_function("session_serialize_100msg", |b| {
        b.iter(|| black_box(serde_json::to_string(&session).unwrap()))
    });
    c.bench_function("session_deserialize_100msg", |b| {
        b.iter(|| {
            let s: Session = serde_json::from_str(&session_json).unwrap();
            black_box(s);
        })
    });
}

criterion_group!(benches, bench_session_create_close, bench_add_messages, bench_jsonl_roundtrip);
criterion_main!(benches);
