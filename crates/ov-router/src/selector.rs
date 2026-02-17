//! Model selection from tier.

use crate::types::{Tier, TierConfig, RoutingDecision};
use std::collections::HashMap;

/// Select model for tier.
pub fn select_model(
    tier: Tier,
    confidence: f64,
    reasoning: &str,
    tier_configs: &HashMap<Tier, TierConfig>,
    estimated_input_tokens: usize,
    max_output_tokens: usize,
) -> RoutingDecision {
    let config = &tier_configs[&tier];
    RoutingDecision {
        model: config.primary.clone(),
        tier,
        confidence,
        method: "rules".into(),
        reasoning: reasoning.to_string(),
        cost_estimate: 0.0, // Simplified: real pricing would be here
        savings: 0.0,
    }
}

/// Get fallback chain.
pub fn get_fallback_chain(tier: Tier, tier_configs: &HashMap<Tier, TierConfig>) -> Vec<String> {
    let config = &tier_configs[&tier];
    let mut chain = vec![config.primary.clone()];
    chain.extend(config.fallback.clone());
    chain
}
