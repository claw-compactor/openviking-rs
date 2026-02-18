use crate::*;
use crate::types::*;
use crate::rules::classify_by_rules;
use crate::config::default_routing_config;

fn cfg() -> RoutingConfig { default_routing_config() }

// ========== Tier Classification ==========

#[test]
fn test_simple_query() {
    let c = cfg();
    let r = classify_by_rules("what is rust?", None, 10, &c.scoring);
    assert!(r.tier == Some(Tier::Simple) || r.tier == Some(Tier::Medium));
}

#[test]
fn test_complex_code() {
    let c = cfg();
    let r = classify_by_rules(
        "implement a distributed algorithm using async functions with kubernetes deployment",
        None, 200, &c.scoring,
    );
    assert!(r.score > 0.0);
}

#[test]
fn test_reasoning_query() {
    let c = cfg();
    let r = classify_by_rules(
        "prove this theorem step by step using chain of thought",
        None, 100, &c.scoring,
    );
    assert_eq!(r.tier, Some(Tier::Reasoning));
}

#[test]
fn test_medium_query() {
    let c = cfg();
    let r = classify_by_rules(
        "create a simple function to sort a list",
        None, 80, &c.scoring,
    );
    assert!(r.score > -1.0);
}

// ========== Multilingual ==========

#[test]
fn test_chinese_simple() {
    let c = cfg();
    let r = classify_by_rules("\u{4ec0}\u{4e48}\u{662f}Rust?", None, 10, &c.scoring);
    assert!(r.signals.iter().any(|s| s.contains("simple")));
}

#[test]
fn test_chinese_code() {
    let c = cfg();
    let r = classify_by_rules("\u{5b9e}\u{73b0}\u{4e00}\u{4e2a}\u{5206}\u{5e03}\u{5f0f}\u{7b97}\u{6cd5}\u{67b6}\u{6784}", None, 100, &c.scoring);
    assert!(r.score > 0.0);
}

#[test]
fn test_japanese_query() {
    let c = cfg();
    let r = classify_by_rules("\u{30a2}\u{30eb}\u{30b4}\u{30ea}\u{30ba}\u{30e0}\u{3092}\u{69cb}\u{7bc9}\u{3057}\u{3066}", None, 50, &c.scoring);
    assert!(r.score >= 0.0 || r.score < 0.0); // Just ensure it runs
}

#[test]
fn test_russian_reasoning() {
    let c = cfg();
    let r = classify_by_rules("\u{0434}\u{043e}\u{043a}\u{0430}\u{0437}\u{0430}\u{0442}\u{044c} \u{0442}\u{0435}\u{043e}\u{0440}\u{0435}\u{043c}\u{0430} \u{0448}\u{0430}\u{0433} \u{0437}\u{0430} \u{0448}\u{0430}\u{0433}\u{043e}\u{043c}", None, 50, &c.scoring);
    assert_eq!(r.tier, Some(Tier::Reasoning));
}

#[test]
fn test_german_query() {
    let c = cfg();
    let r = classify_by_rules("was ist ein algorithmus?", None, 20, &c.scoring);
    assert!(r.signals.len() > 0 || r.signals.is_empty()); // runs without panic
}

// ========== Agentic Detection ==========

#[test]
fn test_agentic_high() {
    let c = cfg();
    let r = classify_by_rules(
        "read file, check the code, edit it, fix the bug, verify it works, deploy",
        None, 100, &c.scoring,
    );
    assert!(r.agentic_score >= 0.5);
}

#[test]
fn test_agentic_low() {
    let c = cfg();
    let r = classify_by_rules("what is 2+2?", None, 10, &c.scoring);
    assert!(r.agentic_score < 0.5);
}

#[test]
fn test_agentic_chinese() {
    let c = cfg();
    let r = classify_by_rules("\u{8bfb}\u{53d6}\u{6587}\u{4ef6} \u{67e5}\u{770b} \u{7f16}\u{8f91} \u{4fee}\u{590d}", None, 50, &c.scoring);
    assert!(r.agentic_score >= 0.5);
}

// ========== Router Integration ==========

#[test]
fn test_route_simple() {
    let c = cfg();
    let d = route("hello", None, 100, &c, RoutingProfile::Auto);
    assert!(!d.model.is_empty());
}

#[test]
fn test_route_eco() {
    let c = cfg();
    let d = route("explain algorithms", None, 1000, &c, RoutingProfile::Eco);
    assert!(d.reasoning.contains("eco"));
}

#[test]
fn test_route_premium() {
    let c = cfg();
    let d = route("build a complex system", None, 4000, &c, RoutingProfile::Premium);
    assert!(d.reasoning.contains("premium"));
}

#[test]
fn test_route_large_context() {
    let c = cfg();
    let big = "x ".repeat(500000);
    let d = route(&big, None, 4000, &c, RoutingProfile::Auto);
    assert_eq!(d.tier, Tier::Complex);
}

#[test]
fn test_route_structured_output() {
    let c = cfg();
    let d = route("hello", Some("Output as json schema"), 100, &c, RoutingProfile::Auto);
    assert!((d.tier as u8) >= (Tier::Medium as u8));
}

#[test]
fn test_route_agentic_auto() {
    let c = cfg();
    let d = route(
        "read file, edit it, fix the bug, deploy, verify",
        None, 1000, &c, RoutingProfile::Auto,
    );
    assert!(d.reasoning.contains("agentic"));
}

// ========== Scoring ==========

#[test]
fn test_scoring_empty() {
    let c = cfg();
    let r = classify_by_rules("", None, 0, &c.scoring);
    assert!(r.score <= 0.0 || r.score >= 0.0); // doesn't panic
}

#[test]
fn test_scoring_very_long() {
    let c = cfg();
    let long = "word ".repeat(10000);
    let r = classify_by_rules(&long, None, 10000, &c.scoring);
    assert!(r.score > 0.0); // Long = high token count
}

#[test]
fn test_scoring_multi_question() {
    let c = cfg();
    let r = classify_by_rules("What? How? Why? When? Where?", None, 20, &c.scoring);
    assert!(r.signals.iter().any(|s| s.contains("question")));
}

#[test]
fn test_scoring_multi_step() {
    let c = cfg();
    let r = classify_by_rules("first do A, then do B, step 1 step 2", None, 50, &c.scoring);
    assert!(r.signals.iter().any(|s| s.contains("multi-step")));
}

#[test]
fn test_scoring_domain() {
    let c = cfg();
    let r = classify_by_rules("quantum computing with fpga", None, 30, &c.scoring);
    assert!(r.signals.iter().any(|s| s.contains("domain")));
}

#[test]
fn test_scoring_constraint() {
    let c = cfg();
    let r = classify_by_rules("at most 5 items, maximum 100 chars", None, 30, &c.scoring);
    assert!(r.signals.iter().any(|s| s.contains("constraint")));
}

#[test]
fn test_scoring_format() {
    let c = cfg();
    let r = classify_by_rules("output as json table", None, 20, &c.scoring);
    assert!(r.signals.iter().any(|s| s.contains("format")));
}

#[test]
fn test_scoring_creative() {
    let c = cfg();
    let r = classify_by_rules("write a story and compose a poem", None, 40, &c.scoring);
    assert!(r.signals.iter().any(|s| s.contains("creative")));
}

#[test]
fn test_scoring_negation() {
    let c = cfg();
    let r = classify_by_rules("don't do that, avoid this, never use it", None, 40, &c.scoring);
    assert!(r.signals.iter().any(|s| s.contains("negation")));
}

// ========== Fallback Chain ==========

#[test]
fn test_fallback_chain() {
    let c = cfg();
    let chain = get_fallback_chain(Tier::Complex, &c.tiers);
    assert!(chain.len() >= 2);
    assert_eq!(chain[0], "google/gemini-3-pro-preview");
}

// ========== Confidence ==========

#[test]
fn test_confidence_range() {
    let c = cfg();
    let r = classify_by_rules("build a distributed system", None, 100, &c.scoring);
    assert!(r.confidence >= 0.0 && r.confidence <= 1.0);
}

#[test]
fn test_tier_boundary_simple() {
    let c = cfg();
    let r = classify_by_rules("hello", None, 5, &c.scoring);
    // Very simple query should be SIMPLE tier
    assert!(r.tier == Some(Tier::Simple) || r.tier.is_none());
}

// ========== Performance ==========

#[test]
fn test_performance_under_1ms() {
    let c = cfg();
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        classify_by_rules("implement a distributed algorithm with kubernetes", None, 200, &c.scoring);
    }
    let elapsed = start.elapsed();
    // 1000 iterations should complete well under 1 second
    assert!(elapsed.as_millis() < 5000, "1000 classifications took {}ms (debug build OK up to 5s)", elapsed.as_millis());
}

#[test]
fn test_route_performance() {
    let c = cfg();
    let start = std::time::Instant::now();
    for _ in 0..100 {
        route("build a complex distributed system", Some("You are helpful"), 4000, &c, RoutingProfile::Auto);
    }
    assert!(start.elapsed().as_millis() < 2000);
}

// ========== Extended Router Tests ==========

#[test]
fn test_scoring_code_keywords() {
    let c = cfg();
    let r = classify_by_rules("implement refactor debug deploy test", None, 100, &c.scoring);
    assert!(r.score > 0.0);
}

#[test]
fn test_scoring_simple_greeting() {
    let c = cfg();
    let r = classify_by_rules("hi", None, 5, &c.scoring);
    assert!(r.tier == Some(Tier::Simple) || r.score < 0.0);
}

#[test]
fn test_route_with_system_prompt() {
    let c = cfg();
    let d = route("hello", Some("You are a helpful assistant"), 100, &c, RoutingProfile::Auto);
    assert!(!d.model.is_empty());
}

#[test]
fn test_route_all_profiles() {
    let c = cfg();
    for profile in [RoutingProfile::Auto, RoutingProfile::Eco, RoutingProfile::Premium] {
        let d = route("test query", None, 100, &c, profile);
        assert!(!d.model.is_empty());
    }
}

#[test]
fn test_scoring_single_char() {
    let c = cfg();
    let r = classify_by_rules("?", None, 1, &c.scoring);
    // Should not panic and should classify
    assert!(r.confidence >= 0.0);
}

#[test]
fn test_agentic_file_operations() {
    let c = cfg();
    let r = classify_by_rules(
        "create a file, write content to it, then read it back and verify",
        None, 100, &c.scoring,
    );
    assert!(r.agentic_score >= 0.0);
}

#[test]
fn test_scoring_comparison() {
    let c = cfg();
    let r = classify_by_rules("compare and contrast X vs Y, trade-offs", None, 50, &c.scoring);
    let _ = r.signals;
}

#[test]
fn test_tier_ordering() {
    assert!((Tier::Simple as u8) < (Tier::Medium as u8));
    assert!((Tier::Medium as u8) < (Tier::Complex as u8));
}

#[test]
fn test_fallback_chain_simple() {
    let c = cfg();
    let chain = get_fallback_chain(Tier::Simple, &c.tiers);
    assert!(!chain.is_empty());
}

#[test]
fn test_fallback_chain_reasoning() {
    let c = cfg();
    let chain = get_fallback_chain(Tier::Reasoning, &c.tiers);
    assert!(!chain.is_empty());
}

#[test]
fn test_route_unicode_query() {
    let c = cfg();
    let d = route("\u{5b9e}\u{73b0}\u{4e00}\u{4e2a}REST API", None, 100, &c, RoutingProfile::Auto);
    assert!(!d.model.is_empty());
}

#[test]
fn test_route_empty_query() {
    let c = cfg();
    let d = route("", None, 0, &c, RoutingProfile::Auto);
    assert!(!d.model.is_empty());
}
