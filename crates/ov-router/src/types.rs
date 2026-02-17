use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complexity tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tier {
    Simple = 0,
    Medium = 1,
    Complex = 2,
    Reasoning = 3,
}

/// Scoring result from classifier.
#[derive(Debug, Clone)]
pub struct ScoringResult {
    pub score: f64,
    pub tier: Option<Tier>,
    pub confidence: f64,
    pub signals: Vec<String>,
    pub agentic_score: f64,
}

/// Routing decision.
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub model: String,
    pub tier: Tier,
    pub confidence: f64,
    pub method: String,
    pub reasoning: String,
    pub cost_estimate: f64,
    pub savings: f64,
}

/// Tier config — primary model + fallbacks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierConfig {
    pub primary: String,
    pub fallback: Vec<String>,
}

/// Scoring config.
#[derive(Debug, Clone)]
pub struct ScoringConfig {
    pub token_count_thresholds: (usize, usize), // (simple, complex)
    pub code_keywords: Vec<String>,
    pub reasoning_keywords: Vec<String>,
    pub simple_keywords: Vec<String>,
    pub technical_keywords: Vec<String>,
    pub creative_keywords: Vec<String>,
    pub imperative_verbs: Vec<String>,
    pub constraint_indicators: Vec<String>,
    pub output_format_keywords: Vec<String>,
    pub reference_keywords: Vec<String>,
    pub negation_keywords: Vec<String>,
    pub domain_specific_keywords: Vec<String>,
    pub agentic_task_keywords: Vec<String>,
    pub dimension_weights: HashMap<String, f64>,
    pub tier_boundaries: (f64, f64, f64), // (simple_medium, medium_complex, complex_reasoning)
    pub confidence_steepness: f64,
    pub confidence_threshold: f64,
}

/// Overrides config.
#[derive(Debug, Clone)]
pub struct OverridesConfig {
    pub max_tokens_force_complex: usize,
    pub structured_output_min_tier: Tier,
    pub ambiguous_default_tier: Tier,
}

/// Full routing config.
#[derive(Debug, Clone)]
pub struct RoutingConfig {
    pub scoring: ScoringConfig,
    pub tiers: HashMap<Tier, TierConfig>,
    pub eco_tiers: HashMap<Tier, TierConfig>,
    pub premium_tiers: HashMap<Tier, TierConfig>,
    pub agentic_tiers: HashMap<Tier, TierConfig>,
    pub overrides: OverridesConfig,
}
