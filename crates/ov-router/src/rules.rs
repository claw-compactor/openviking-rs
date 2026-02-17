//! 14-dimension weighted classifier.

use crate::types::{ScoringConfig, ScoringResult, Tier};




struct DimensionScore {
    name: String,
    score: f64,
    signal: Option<String>,
}

fn score_token_count(tokens: usize, thresholds: (usize, usize)) -> DimensionScore {
    if tokens < thresholds.0 {
        DimensionScore { name: "tokenCount".into(), score: -1.0, signal: Some(format!("short ({} tokens)", tokens)) }
    } else if tokens > thresholds.1 {
        DimensionScore { name: "tokenCount".into(), score: 1.0, signal: Some(format!("long ({} tokens)", tokens)) }
    } else {
        DimensionScore { name: "tokenCount".into(), score: 0.0, signal: None }
    }
}

fn score_keywords(
    text: &str, keywords: &[String], name: &str, label: &str,
    thresholds: (usize, usize), scores: (f64, f64, f64),
) -> DimensionScore {
    let matches: Vec<&String> = keywords.iter().filter(|kw| text.contains(kw.to_lowercase().as_str())).collect();
    let count = matches.len();
    if count >= thresholds.1 {
        let top: Vec<_> = matches.iter().take(3).map(|s| s.as_str()).collect();
        DimensionScore { name: name.into(), score: scores.2, signal: Some(format!("{} ({})", label, top.join(", "))) }
    } else if count >= thresholds.0 {
        let top: Vec<_> = matches.iter().take(3).map(|s| s.as_str()).collect();
        DimensionScore { name: name.into(), score: scores.1, signal: Some(format!("{} ({})", label, top.join(", "))) }
    } else {
        DimensionScore { name: name.into(), score: scores.0, signal: None }
    }
}

fn score_multi_step(text: &str) -> DimensionScore {
    let patterns = ["first.*then", "step \\d", "\\d+\\.\\s"];
    let hits = patterns.iter().filter(|p| regex::Regex::new(p).map(|r| r.is_match(text)).unwrap_or(false)).count();
    if hits > 0 {
        DimensionScore { name: "multiStepPatterns".into(), score: 0.5, signal: Some("multi-step".into()) }
    } else {
        DimensionScore { name: "multiStepPatterns".into(), score: 0.0, signal: None }
    }
}

fn score_question_complexity(prompt: &str) -> DimensionScore {
    let count = prompt.matches('?').count();
    if count > 3 {
        DimensionScore { name: "questionComplexity".into(), score: 0.5, signal: Some(format!("{} questions", count)) }
    } else {
        DimensionScore { name: "questionComplexity".into(), score: 0.0, signal: None }
    }
}

fn score_agentic(text: &str, keywords: &[String]) -> (DimensionScore, f64) {
    let matches: Vec<&String> = keywords.iter().filter(|kw| text.contains(kw.to_lowercase().as_str())).collect();
    let count = matches.len();
    let top: Vec<_> = matches.iter().take(3).map(|s| s.as_str()).collect();
    let label = top.join(", ");

    if count >= 4 {
        (DimensionScore { name: "agenticTask".into(), score: 1.0, signal: Some(format!("agentic ({})", label)) }, 1.0)
    } else if count >= 3 {
        (DimensionScore { name: "agenticTask".into(), score: 0.6, signal: Some(format!("agentic ({})", label)) }, 0.6)
    } else if count >= 1 {
        (DimensionScore { name: "agenticTask".into(), score: 0.2, signal: Some(format!("agentic-light ({})", label)) }, 0.2)
    } else {
        (DimensionScore { name: "agenticTask".into(), score: 0.0, signal: None }, 0.0)
    }
}

fn sigmoid(distance: f64, steepness: f64) -> f64 {
    1.0 / (1.0 + (-steepness * distance).exp())
}

/// Classify by rules — 14 weighted dimensions.
pub fn classify_by_rules(
    prompt: &str,
    system_prompt: Option<&str>,
    estimated_tokens: usize,
    config: &ScoringConfig,
) -> ScoringResult {
    let text = format!("{} {}", system_prompt.unwrap_or(""), prompt).to_lowercase();
    let user_text = prompt.to_lowercase();

    let mut dimensions = vec![
        score_token_count(estimated_tokens, config.token_count_thresholds),
        score_keywords(&text, &config.code_keywords, "codePresence", "code", (1, 2), (0.0, 0.5, 1.0)),
        score_keywords(&user_text, &config.reasoning_keywords, "reasoningMarkers", "reasoning", (1, 2), (0.0, 0.7, 1.0)),
        score_keywords(&text, &config.technical_keywords, "technicalTerms", "technical", (2, 4), (0.0, 0.5, 1.0)),
        score_keywords(&text, &config.creative_keywords, "creativeMarkers", "creative", (1, 2), (0.0, 0.5, 0.7)),
        score_keywords(&text, &config.simple_keywords, "simpleIndicators", "simple", (1, 2), (0.0, -1.0, -1.0)),
        score_multi_step(&text),
        score_question_complexity(prompt),
        score_keywords(&text, &config.imperative_verbs, "imperativeVerbs", "imperative", (1, 2), (0.0, 0.3, 0.5)),
        score_keywords(&text, &config.constraint_indicators, "constraintCount", "constraints", (1, 3), (0.0, 0.3, 0.7)),
        score_keywords(&text, &config.output_format_keywords, "outputFormat", "format", (1, 2), (0.0, 0.4, 0.7)),
        score_keywords(&text, &config.reference_keywords, "referenceComplexity", "references", (1, 2), (0.0, 0.3, 0.5)),
        score_keywords(&text, &config.negation_keywords, "negationComplexity", "negation", (2, 3), (0.0, 0.3, 0.5)),
        score_keywords(&text, &config.domain_specific_keywords, "domainSpecificity", "domain-specific", (1, 2), (0.0, 0.5, 0.8)),
    ];

    let (agentic_dim, agentic_score) = score_agentic(&text, &config.agentic_task_keywords);
    dimensions.push(agentic_dim);

    let signals: Vec<String> = dimensions.iter().filter_map(|d| d.signal.clone()).collect();

    let mut weighted_score = 0.0;
    for d in &dimensions {
        let w = config.dimension_weights.get(&d.name).copied().unwrap_or(0.0);
        weighted_score += d.score * w;
    }

    // Reasoning override
    let reasoning_matches = config.reasoning_keywords.iter()
        .filter(|kw| user_text.contains(kw.to_lowercase().as_str()))
        .count();
    if reasoning_matches >= 2 {
        let conf = sigmoid(weighted_score.max(0.3), config.confidence_steepness).max(0.85);
        return ScoringResult { score: weighted_score, tier: Some(Tier::Reasoning), confidence: conf, signals, agentic_score };
    }

    let (sm, mc, cr) = config.tier_boundaries;
    let (tier, distance) = if weighted_score < sm {
        (Tier::Simple, sm - weighted_score)
    } else if weighted_score < mc {
        (Tier::Medium, (weighted_score - sm).min(mc - weighted_score))
    } else if weighted_score < cr {
        (Tier::Complex, (weighted_score - mc).min(cr - weighted_score))
    } else {
        (Tier::Reasoning, weighted_score - cr)
    };

    let confidence = sigmoid(distance, config.confidence_steepness);
    if confidence < config.confidence_threshold {
        return ScoringResult { score: weighted_score, tier: None, confidence, signals, agentic_score };
    }

    ScoringResult { score: weighted_score, tier: Some(tier), confidence, signals, agentic_score }
}
