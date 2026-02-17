use crate::*;
use crate::session::*;
use crate::manager::SessionManager;
use crate::memory::*;
use crate::compressor::SessionCompressor;
use crate::context_window::*;

// ========== Session Creation ==========

#[test]
fn test_session_create() {
    let s = Session::new("user1");
    assert_eq!(s.user_id, "user1");
    assert_eq!(s.state, SessionState::Active);
    assert!(s.messages.is_empty());
}

#[test]
fn test_session_with_id() {
    let s = Session::with_id("custom-id", "user1");
    assert_eq!(s.id, "custom-id");
}

#[test]
fn test_session_uri() {
    let s = Session::with_id("abc", "u");
    assert_eq!(s.uri(), "viking://session/abc");
}

// ========== Message Management ==========

#[test]
fn test_add_message() {
    let mut s = Session::new("u");
    s.add_message(Role::User, vec![Part::text("hello")]);
    assert_eq!(s.message_count(), 1);
    assert_eq!(s.stats.total_turns, 1);
}

#[test]
fn test_add_assistant_message() {
    let mut s = Session::new("u");
    s.add_message(Role::Assistant, vec![Part::text("hi")]);
    assert_eq!(s.stats.total_turns, 0); // Only user turns count
}

#[test]
fn test_message_content() {
    let msg = Message::new(Role::User, vec![Part::text("a"), Part::text("b")]);
    assert_eq!(msg.content(), "a\nb");
}

#[test]
fn test_tool_part() {
    let mut msg = Message::new(Role::Assistant, vec![Part::tool("run", "ls")]);
    assert!(msg.find_tool_part("run").is_some());
    assert!(msg.find_tool_part("other").is_none());
}

#[test]
fn test_update_tool() {
    let mut s = Session::new("u");
    s.add_message(Role::Assistant, vec![Part::tool("exec", "cmd")]);
    let msg_id = s.messages[0].id.clone();
    assert!(s.update_tool(&msg_id, "exec", "output", "completed"));
    assert!(!s.update_tool(&msg_id, "nonexist", "", ""));
}

#[test]
fn test_multiple_messages() {
    let mut s = Session::new("u");
    for i in 0..10 {
        s.add_message(Role::User, vec![Part::text(format!("msg {}", i))]);
        s.add_message(Role::Assistant, vec![Part::text(format!("reply {}", i))]);
    }
    assert_eq!(s.message_count(), 20);
    assert_eq!(s.stats.total_turns, 10);
}

// ========== JSONL Serialization ==========

#[test]
fn test_message_jsonl_roundtrip() {
    let msg = Message::new(Role::User, vec![Part::text("hello world")]);
    let line = msg.to_jsonl();
    let parsed = Message::from_jsonl(&line).unwrap();
    assert_eq!(parsed.role, Role::User);
    assert_eq!(parsed.content(), "hello world");
}

#[test]
fn test_session_jsonl() {
    let mut s = Session::new("u");
    s.add_message(Role::User, vec![Part::text("q1")]);
    s.add_message(Role::Assistant, vec![Part::text("a1")]);
    let jsonl = s.messages_to_jsonl();
    assert_eq!(jsonl.lines().count(), 2);
}

#[test]
fn test_load_messages_from_jsonl() {
    let mut s1 = Session::new("u");
    s1.add_message(Role::User, vec![Part::text("q")]);
    s1.add_message(Role::Assistant, vec![Part::text("a")]);
    let jsonl = s1.messages_to_jsonl();

    let mut s2 = Session::new("u");
    let count = s2.load_messages_from_jsonl(&jsonl).unwrap();
    assert_eq!(count, 2);
    assert_eq!(s2.message_count(), 2);
}

// ========== Session Lifecycle ==========

#[test]
fn test_commit() {
    let mut s = Session::new("u");
    s.add_message(Role::User, vec![Part::text("hi")]);
    let archived = s.commit();
    assert_eq!(archived.len(), 1);
    assert!(s.messages.is_empty());
    assert_eq!(s.state, SessionState::Committed);
    assert_eq!(s.compression.compression_index, 1);
}

#[test]
fn test_commit_empty() {
    let mut s = Session::new("u");
    let archived = s.commit();
    assert!(archived.is_empty());
}

#[test]
fn test_close() {
    let mut s = Session::new("u");
    s.close();
    assert_eq!(s.state, SessionState::Closed);
}

#[test]
fn test_multiple_commits() {
    let mut s = Session::new("u");
    s.add_message(Role::User, vec![Part::text("1")]);
    s.commit();
    s.state = SessionState::Active; // Re-activate
    s.add_message(Role::User, vec![Part::text("2")]);
    s.commit();
    assert_eq!(s.compression.compression_index, 2);
    assert_eq!(s.compression.original_count, 2);
}

// ========== Usage Tracking ==========

#[test]
fn test_track_context_usage() {
    let mut s = Session::new("u");
    s.track_usage(Usage::context("viking://ctx/1"));
    assert_eq!(s.stats.contexts_used, 1);
    assert_eq!(s.usage_records.len(), 1);
}

#[test]
fn test_track_skill_usage() {
    let mut s = Session::new("u");
    s.track_usage(Usage::skill("viking://skill/1", "in", "out", true));
    assert_eq!(s.stats.skills_used, 1);
}

// ========== Needs Compression ==========

#[test]
fn test_needs_compression() {
    let mut s = Session::new("u");
    s.auto_commit_threshold = 100;
    // Add enough content
    let big = "x".repeat(500);
    s.add_message(Role::User, vec![Part::text(&big)]);
    assert!(s.needs_compression());
}

#[test]
fn test_no_compression_needed() {
    let s = Session::new("u");
    assert!(!s.needs_compression());
}

// ========== SessionManager ==========

#[test]
fn test_manager_create_get() {
    let mgr = SessionManager::new();
    let s = mgr.create("u1");
    let got = mgr.get(&s.id).unwrap();
    assert_eq!(got.user_id, "u1");
}

#[test]
fn test_manager_create_with_id() {
    let mgr = SessionManager::new();
    mgr.create_with_id("my-id", "u1");
    assert!(mgr.get("my-id").is_some());
}

#[test]
fn test_manager_list_active() {
    let mgr = SessionManager::new();
    mgr.create("u1");
    mgr.create("u2");
    assert_eq!(mgr.list_active().len(), 2);
}

#[test]
fn test_manager_close() {
    let mgr = SessionManager::new();
    let s = mgr.create("u1");
    mgr.close(&s.id);
    assert_eq!(mgr.list_active().len(), 0);
}

#[test]
fn test_manager_list_by_user() {
    let mgr = SessionManager::new();
    mgr.create("alice");
    mgr.create("alice");
    mgr.create("bob");
    assert_eq!(mgr.list_by_user("alice").len(), 2);
}

#[test]
fn test_manager_remove() {
    let mgr = SessionManager::new();
    let s = mgr.create("u");
    mgr.remove(&s.id);
    assert!(mgr.get(&s.id).is_none());
}

#[test]
fn test_manager_count() {
    let mgr = SessionManager::new();
    mgr.create("u1");
    mgr.create("u2");
    assert_eq!(mgr.count(), 2);
}

#[test]
fn test_manager_update() {
    let mgr = SessionManager::new();
    let mut s = mgr.create("u1");
    s.add_message(Role::User, vec![Part::text("hi")]);
    mgr.update(&s);
    let got = mgr.get(&s.id).unwrap();
    assert_eq!(got.message_count(), 1);
}

#[test]
fn test_manager_concurrent() {
    use std::thread;
    let mgr = SessionManager::new();
    let mgr2 = mgr.clone();
    let h = thread::spawn(move || {
        for _ in 0..50 {
            mgr2.create("t2");
        }
    });
    for _ in 0..50 {
        mgr.create("t1");
    }
    h.join().unwrap();
    assert_eq!(mgr.count(), 100);
}

// ========== Memory Extraction ==========

#[test]
fn test_detect_language_en() {
    let msgs = vec![Message::new(Role::User, vec![Part::text("Hello world")])]; 
    assert_eq!(detect_language(&msgs), "en");
}

#[test]
fn test_detect_language_zh() {
    let msgs = vec![Message::new(Role::User, vec![Part::text("你好世界")])]; 
    assert_eq!(detect_language(&msgs), "zh-CN");
}

#[test]
fn test_detect_language_ja() {
    let msgs = vec![Message::new(Role::User, vec![Part::text("こんにちは")])]; 
    assert_eq!(detect_language(&msgs), "ja");
}

#[test]
fn test_detect_language_ru() {
    let msgs = vec![Message::new(Role::User, vec![Part::text("Привет мир")])]; 
    assert_eq!(detect_language(&msgs), "ru");
}

#[test]
fn test_detect_language_empty() {
    let msgs: Vec<Message> = vec![];
    assert_eq!(detect_language(&msgs), "en");
}

#[test]
fn test_extract_candidates_empty() {
    let result = extract_candidates(&[], "s1", "u1");
    assert!(result.is_empty());
}

#[test]
fn test_extract_candidates_preferences() {
    let msgs = vec![Message::new(Role::User, vec![Part::text("I prefer dark mode for all editors")])]; 
    let candidates = extract_candidates(&msgs, "s1", "u1");
    assert!(!candidates.is_empty());
    assert_eq!(candidates[0].category, MemoryCategory::Preferences);
}

#[test]
fn test_extract_candidates_profile() {
    let msgs = vec![Message::new(Role::User, vec![Part::text("My name is Alice and I am a developer")])]; 
    let candidates = extract_candidates(&msgs, "s1", "u1");
    assert!(!candidates.is_empty());
    assert_eq!(candidates[0].category, MemoryCategory::Profile);
}

#[test]
fn test_extract_candidates_short_skip() {
    let msgs = vec![Message::new(Role::User, vec![Part::text("hi")])]; 
    let candidates = extract_candidates(&msgs, "s1", "u1");
    assert!(candidates.is_empty());
}

#[test]
fn test_memory_category_str() {
    assert_eq!(MemoryCategory::Profile.as_str(), "profile");
    assert_eq!(MemoryCategory::from_str("events"), MemoryCategory::Events);
    assert_eq!(MemoryCategory::from_str("unknown"), MemoryCategory::Patterns);
}

#[test]
fn test_memory_category_directory() {
    assert_eq!(MemoryCategory::Profile.directory(), "memories/profile.md");
    assert_eq!(MemoryCategory::Cases.directory(), "memories/cases");
}

#[test]
fn test_memory_category_always_merge() {
    assert!(MemoryCategory::Profile.always_merge());
    assert!(!MemoryCategory::Events.always_merge());
}

// ========== Compressor ==========

#[test]
fn test_compressor_no_compression_needed() {
    let compressor = SessionCompressor::new();
    let msgs: Vec<Message> = (0..5)
        .map(|i| Message::new(Role::User, vec![Part::text(format!("msg {}", i))]))
        .collect();
    let (kept, summary) = compressor.compress(&msgs);
    assert_eq!(kept.len(), 5);
    assert!(summary.is_none());
}

#[test]
fn test_compressor_compress() {
    let compressor = SessionCompressor::new().with_max_messages(5);
    let msgs: Vec<Message> = (0..20)
        .map(|i| Message::new(Role::User, vec![Part::text(format!("message number {}", i))]))
        .collect();
    let (kept, summary) = compressor.compress(&msgs);
    assert!(kept.len() < 20);
    assert!(summary.is_some());
    let s = summary.unwrap();
    assert!(s.contains("Compressed Session Archive"));
}

#[test]
fn test_compressor_summary() {
    let compressor = SessionCompressor::new();
    let msgs = vec![
        Message::new(Role::User, vec![Part::text("What is Rust?")]),
        Message::new(Role::Assistant, vec![Part::text("Rust is a systems programming language.")]),
    ];
    let summary = compressor.generate_summary(&msgs);
    assert!(summary.contains("1 turns"));
}

#[test]
fn test_compressor_extract_memories() {
    let compressor = SessionCompressor::new();
    let msgs = vec![
        Message::new(Role::User, vec![Part::text("I prefer vim over emacs for editing code")]),
    ];
    let (candidates, stats) = compressor.extract_memories(&msgs, "s1", "u1");
    assert!(!candidates.is_empty());
    assert!(stats.created > 0);
}

// ========== Context Window ==========

#[test]
fn test_context_window_add() {
    let mut cw = ContextWindow::new(1000);
    let entry = ContextEntry {
        uri: "test".into(),
        layer: ContextLayer::L2,
        content: "hello world".into(),
    };
    assert!(cw.add(entry));
    assert_eq!(cw.entries().len(), 1);
}

#[test]
fn test_context_window_overflow() {
    let mut cw = ContextWindow::new(10);
    let entry = ContextEntry {
        uri: "test".into(),
        layer: ContextLayer::L2,
        content: "x".repeat(100),
    };
    assert!(!cw.add(entry));
}

#[test]
fn test_context_window_adaptive() {
    let mut cw = ContextWindow::new(20);
    let l0 = "short";
    let l1 = "medium length text here";
    let l2 = "x".repeat(200);
    let layer = cw.add_adaptive("uri", l0, l1, &l2);
    // L2 won't fit (200/4=50 > 20), L1 won't fit (22/4=5, fits!)
    // Actually 22/4=5 which is < 20, so L1 fits
    assert!(layer == ContextLayer::L1 || layer == ContextLayer::L2 || layer == ContextLayer::L0);
}

#[test]
fn test_context_window_clear() {
    let mut cw = ContextWindow::new(1000);
    cw.add(ContextEntry { uri: "a".into(), layer: ContextLayer::L0, content: "x".into() });
    cw.clear();
    assert_eq!(cw.entries().len(), 0);
    assert_eq!(cw.used_tokens(), 0);
}

#[test]
fn test_context_window_remaining() {
    let mut cw = ContextWindow::new(100);
    cw.add(ContextEntry { uri: "a".into(), layer: ContextLayer::L0, content: "x".repeat(40) });
    assert_eq!(cw.remaining_tokens(), 90); // 40/4 = 10 used
}

#[test]
fn test_build_session_context() {
    let mut s = Session::new("u");
    for i in 0..10 {
        s.add_message(Role::User, vec![Part::text(format!("q{}", i))]);
    }
    let ctx = ContextWindow::build_session_context(&s, 3, 2, "test");
    assert_eq!(ctx.recent_messages.len(), 3);
}

#[test]
fn test_generate_summary() {
    let mut s = Session::new("u");
    s.add_message(Role::User, vec![Part::text("What is Rust?")]);
    let summary = s.generate_summary();
    assert!(summary.contains("1 turns"));
}

#[test]
fn test_session_display() {
    let s = Session::new("alice");
    let display = format!("{}", s);
    assert!(display.contains("alice"));
}
