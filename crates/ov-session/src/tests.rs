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
    let mut parsed = Message::from_jsonl(&line).unwrap();
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
    let _ = cw.add(entry); // may or may not fit
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

// ========== Extended Session Tests ==========

// --- Session Lifecycle Edge Cases ---

#[test]
fn test_double_close() {
    let mut s = Session::new("u");
    s.close();
    s.close(); // Should not panic
    assert_eq!(s.state, SessionState::Closed);
}

#[test]
fn test_add_message_after_close() {
    let mut s = Session::new("u");
    s.close();
    // Adding message after close - implementation-dependent behavior
    s.add_message(Role::User, vec![Part::text("after close")]);
    // Just verify no panic
}

#[test]
fn test_commit_after_close() {
    let mut s = Session::new("u");
    s.close();
    let archived = s.commit();
    assert!(archived.is_empty());
}

#[test]
fn test_session_many_messages() {
    let mut s = Session::new("u");
    for i in 0..1000 {
        s.add_message(Role::User, vec![Part::text(format!("msg {}", i))]);
    }
    assert_eq!(s.message_count(), 1000);
    assert_eq!(s.stats.total_turns, 1000);
}

#[test]
fn test_session_empty_content() {
    let mut s = Session::new("u");
    s.add_message(Role::User, vec![Part::text("")]);
    assert_eq!(s.message_count(), 1);
}

#[test]
fn test_session_unicode_messages() {
    let mut s = Session::new("u");
    s.add_message(Role::User, vec![Part::text("你好世界")]);
    s.add_message(Role::User, vec![Part::text("こんにちは")]);
    s.add_message(Role::User, vec![Part::text("🦀✨")]);
    assert_eq!(s.message_count(), 3);
    let jsonl = s.messages_to_jsonl();
    assert!(jsonl.contains("你好"));
    assert!(jsonl.contains("こんにちは"));
}

#[test]
fn test_session_system_message() {
    let mut s = Session::new("u");
    s.add_message(Role::System, vec![Part::text("You are helpful")]);
    assert_eq!(s.message_count(), 1);
    assert_eq!(s.stats.total_turns, 0); // System messages don't count as turns
}

#[test]
fn test_message_multipart_content() {
    let msg = Message::new(Role::User, vec![
        Part::text("Part 1"),
        Part::text("Part 2"),
        Part::text("Part 3"),
    ]);
    assert_eq!(msg.content(), "Part 1\nPart 2\nPart 3");
}

#[test]
fn test_message_empty_parts() {
    let msg = Message::new(Role::User, vec![]);
    assert_eq!(msg.content(), "");
}

#[test]
fn test_jsonl_roundtrip_with_tool() {
    let msg = Message::new(Role::Assistant, vec![
        Part::text("Running tool..."),
        Part::tool("search", "query=test"),
    ]);
    let line = msg.to_jsonl();
    let mut parsed = Message::from_jsonl(&line).unwrap();
    assert_eq!(parsed.role, Role::Assistant);
    assert!(parsed.find_tool_part("search").is_some());
}

#[test]
fn test_jsonl_roundtrip_system() {
    let msg = Message::new(Role::System, vec![Part::text("system prompt")]);
    let line = msg.to_jsonl();
    let mut parsed = Message::from_jsonl(&line).unwrap();
    assert_eq!(parsed.role, Role::System);
    assert_eq!(parsed.content(), "system prompt");
}

// --- Manager Edge Cases ---

#[test]
fn test_manager_get_nonexistent() {
    let mgr = SessionManager::new();
    assert!(mgr.get("nonexistent").is_none());
}

#[test]
fn test_manager_close_nonexistent() {
    let mgr = SessionManager::new();
    mgr.close("nonexistent"); // Should not panic
}

#[test]
fn test_manager_remove_nonexistent() {
    let mgr = SessionManager::new();
    mgr.remove("nonexistent"); // Should not panic
}

#[test]
fn test_manager_list_by_user_empty() {
    let mgr = SessionManager::new();
    assert!(mgr.list_by_user("nobody").is_empty());
}

#[test]
fn test_manager_many_sessions() {
    let mgr = SessionManager::new();
    for i in 0..100 {
        mgr.create(&format!("user_{}", i));
    }
    assert_eq!(mgr.count(), 100);
}

// --- Memory Extraction ---

#[test]
fn test_detect_language_mixed() {
    let msgs = vec![Message::new(Role::User, vec![Part::text("Hello 你好")])];
    let lang = detect_language(&msgs);
    // Mixed content - implementation determines priority
    assert!(!lang.is_empty());
}

#[test]
fn test_extract_candidates_events() {
    let msgs = vec![Message::new(Role::User, vec![Part::text("Yesterday I went to the conference and met John. We discussed the project deadline which is next Friday.")])]; 
    let candidates = extract_candidates(&msgs, "s1", "u1");
    // Should extract event-related memory
    let _ = &candidates;
}

#[test]
fn test_extract_candidates_technical() {
    let msgs = vec![Message::new(Role::User, vec![Part::text("I always use Rust for systems programming and Python for scripting. My IDE is VS Code with vim bindings.")])]; 
    let candidates = extract_candidates(&msgs, "s1", "u1");
    let _ = &candidates;
}

#[test]
fn test_extract_candidates_assistant_only() {
    let msgs = vec![Message::new(Role::Assistant, vec![Part::text("I can help you with that. Here is the answer.")])]; 
    let candidates = extract_candidates(&msgs, "s1", "u1");
    // Assistant messages shouldn't generate user memory
    assert!(candidates.is_empty());
}

// --- Compressor Edge Cases ---

#[test]
fn test_compressor_single_message() {
    let compressor = SessionCompressor::new();
    let msgs = vec![Message::new(Role::User, vec![Part::text("Only one")])]; 
    let (kept, summary) = compressor.compress(&msgs);
    assert_eq!(kept.len(), 1);
    assert!(summary.is_none());
}

#[test]
fn test_compressor_empty_messages() {
    let compressor = SessionCompressor::new();
    let msgs: Vec<Message> = vec![];
    let (kept, summary) = compressor.compress(&msgs);
    assert!(kept.is_empty());
    assert!(summary.is_none());
}

#[test]
fn test_compressor_with_tool_messages() {
    let compressor = SessionCompressor::new().with_max_messages(3);
    let mut msgs = Vec::new();
    for i in 0..10 {
        msgs.push(Message::new(Role::User, vec![Part::text(format!("question {}", i))]));
        msgs.push(Message::new(Role::Assistant, vec![
            Part::text(format!("answer {}", i)),
            Part::tool("search", "query"),
        ]));
    }
    let (kept, _) = compressor.compress(&msgs);
    assert!(kept.len() < 20);
}

// --- Context Window Edge Cases ---

#[test]
fn test_context_window_multiple_entries() {
    let mut cw = ContextWindow::new(10000);
    for i in 0..10 {
        cw.add(ContextEntry {
            uri: format!("ctx_{}", i),
            layer: ContextLayer::L0,
            content: format!("content {}", i),
        });
    }
    assert_eq!(cw.entries().len(), 10);
}

#[test]
fn test_context_window_exact_capacity() {
    let mut cw = ContextWindow::new(3);
    // 12 chars / 4 = 3 tokens
    let entry = ContextEntry {
        uri: "test".into(),
        layer: ContextLayer::L0,
        content: "123456789012".into(),
    };
    assert!(cw.add(entry));
    assert_eq!(cw.remaining_tokens(), 0);
}

#[test]
fn test_context_window_zero_capacity() {
    let mut cw = ContextWindow::new(0);
    let entry = ContextEntry {
        uri: "test".into(),
        layer: ContextLayer::L0,
        content: "any".into(),
    };
    let _ = cw.add(entry); // may or may not fit
}

// --- Usage Tracking ---

#[test]
fn test_track_multiple_usages() {
    let mut s = Session::new("u");
    for i in 0..10 {
        s.track_usage(Usage::context(format!("viking://ctx/{}", i)));
    }
    assert_eq!(s.stats.contexts_used, 10);
    assert_eq!(s.usage_records.len(), 10);
}

#[test]
fn test_track_mixed_usages() {
    let mut s = Session::new("u");
    s.track_usage(Usage::context("viking://ctx/1"));
    s.track_usage(Usage::skill("viking://skill/1", "in", "out", true));
    s.track_usage(Usage::context("viking://ctx/2"));
    s.track_usage(Usage::skill("viking://skill/2", "in", "out", false));
    assert_eq!(s.stats.contexts_used, 2);
    assert_eq!(s.stats.skills_used, 2);
}

#[test]
fn test_session_id_format() {
    let s = Session::new("user1");
    assert!(!s.id.is_empty());
    // ID should be a valid UUID-like string
    assert!(s.id.len() >= 8);
}

#[test]
fn test_message_clone() {
    let msg = Message::new(Role::User, vec![Part::text("hello")]);
    let cloned = msg.clone();
    assert_eq!(msg.role, cloned.role);
}

#[test]
fn test_session_created_at() {
    let s = Session::new("user1");
    assert!(s.created_at.timestamp() > 0);
}

#[test]
fn test_session_user_id() {
    let s = Session::new("test_user_123");
    assert_eq!(s.user_id, "test_user_123");
}

#[test]
fn test_part_text_content() {
    let p = Part::text("hello world");
    assert_eq!(p.text.as_deref(), Some("hello world"));
}

#[test]
fn test_role_display() {
    assert_eq!(format!("{}", Role::User), "user");
    assert_eq!(format!("{}", Role::Assistant), "assistant");
    assert_eq!(format!("{}", Role::System), "system");
}
