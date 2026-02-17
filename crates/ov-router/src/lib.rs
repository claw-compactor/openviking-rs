//! OpenViking Router — 14-dimension weighted classifier with multilingual support.

pub mod types;
pub mod rules;
pub mod config;
pub mod selector;

pub use types::*;
pub use rules::classify_by_rules;
pub use config::ROUTING_CONFIG;
pub use selector::*;

/// Routing profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingProfile {
    Eco,
    Auto,
    Premium,
}

/// Route a request to the best model.
pub fn route(
    prompt: &str,
    system_prompt: Option<&str>,
    max_output_tokens: usize,
    config: &RoutingConfig,
    profile: RoutingProfile,
) -> RoutingDecision {
    let full_text = format!("{} {}", system_prompt.unwrap_or(""), prompt);
    let estimated_tokens = full_text.len() / 4;

    let rule_result = classify_by_rules(prompt, system_prompt, estimated_tokens, &config.scoring);

    let (tier_configs, suffix) = match profile {
        RoutingProfile::Eco => (&config.eco_tiers, " | eco"),
        RoutingProfile::Premium => (&config.premium_tiers, " | premium"),
        RoutingProfile::Auto => {
            if rule_result.agentic_score >= 0.5 {
                (&config.agentic_tiers, " | agentic")
            } else {
                (&config.tiers, "")
            }
        }
    };

    if estimated_tokens > config.overrides.max_tokens_force_complex {
        return select_model(
            Tier::Complex,
            0.95,
            &format!("Input exceeds {} tokens{}", config.overrides.max_tokens_force_complex, suffix),
            tier_configs,
            estimated_tokens,
            max_output_tokens,
        );
    }

    let has_structured = system_prompt
        .map(|s| s.to_lowercase())
        .map(|s| s.contains("json") || s.contains("structured") || s.contains("schema"))
        .unwrap_or(false);

    let (mut tier, confidence, mut reasoning) = if let Some(t) = rule_result.tier {
        (t, rule_result.confidence, format!("score={:.2} | {}", rule_result.score, rule_result.signals.join(", ")))
    } else {
        let t = config.overrides.ambiguous_default_tier;
        (t, 0.5, format!("score={:.2} | {} | ambiguous -> default: {:?}", rule_result.score, rule_result.signals.join(", "), t))
    };

    if has_structured {
        let min_tier = config.overrides.structured_output_min_tier;
        if (tier as u8) < (min_tier as u8) {
            reasoning += &format!(" | upgraded to {:?} (structured output)", min_tier);
            tier = min_tier;
        }
    }

    reasoning += suffix;

    select_model(tier, confidence, &reasoning, tier_configs, estimated_tokens, max_output_tokens)
}

#[cfg(test)]
mod tests;
