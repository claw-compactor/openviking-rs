//! Default routing configuration with multilingual keywords.

use crate::types::*;
use std::collections::HashMap;

fn s(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn weights() -> HashMap<String, f64> {
    let mut m = HashMap::new();
    m.insert("tokenCount".into(), 0.08);
    m.insert("codePresence".into(), 0.15);
    m.insert("reasoningMarkers".into(), 0.18);
    m.insert("technicalTerms".into(), 0.10);
    m.insert("creativeMarkers".into(), 0.05);
    m.insert("simpleIndicators".into(), 0.02);
    m.insert("multiStepPatterns".into(), 0.12);
    m.insert("questionComplexity".into(), 0.05);
    m.insert("imperativeVerbs".into(), 0.03);
    m.insert("constraintCount".into(), 0.04);
    m.insert("outputFormat".into(), 0.03);
    m.insert("referenceComplexity".into(), 0.02);
    m.insert("negationComplexity".into(), 0.01);
    m.insert("domainSpecificity".into(), 0.02);
    m.insert("agenticTask".into(), 0.04);
    m
}

fn tier_map(configs: &[(Tier, &str, &[&str])]) -> HashMap<Tier, TierConfig> {
    configs.iter().map(|(tier, primary, fallback)| {
        (*tier, TierConfig {
            primary: primary.to_string(),
            fallback: fallback.iter().map(|s| s.to_string()).collect(),
        })
    }).collect()
}

/// Default routing configuration.
pub fn default_routing_config() -> RoutingConfig {
    RoutingConfig {
        scoring: ScoringConfig {
            token_count_thresholds: (50, 500),
            code_keywords: s(&[
                "function", "class", "import", "def", "select", "async", "await",
                "const", "let", "var", "return", "```",
                "\u{51fd}\u{6570}", "\u{7c7b}", "\u{5bfc}\u{5165}", "\u{5b9a}\u{4e49}",
                "\u{95a2}\u{6570}", "\u{30af}\u{30e9}\u{30b9}",
                "\u{0444}\u{0443}\u{043d}\u{043a}\u{0446}\u{0438}\u{044f}", "\u{043a}\u{043b}\u{0430}\u{0441}\u{0441}",
                "funktion", "klasse",
            ]),
            reasoning_keywords: s(&[
                "prove", "theorem", "derive", "step by step", "chain of thought",
                "formally", "mathematical", "proof", "logically",
                "\u{8bc1}\u{660e}", "\u{5b9a}\u{7406}", "\u{63a8}\u{5bfc}", "\u{9010}\u{6b65}", "\u{601d}\u{7ef4}\u{94fe}",
                "\u{8a3c}\u{660e}", "\u{5b9a}\u{7406}", "\u{5c0e}\u{51fa}",
                "\u{0434}\u{043e}\u{043a}\u{0430}\u{0437}\u{0430}\u{0442}\u{044c}", "\u{0442}\u{0435}\u{043e}\u{0440}\u{0435}\u{043c}\u{0430}", "\u{0448}\u{0430}\u{0433} \u{0437}\u{0430} \u{0448}\u{0430}\u{0433}\u{043e}\u{043c}",
                "beweisen", "theorem", "schritt f\u{00fc}r schritt",
            ]),
            simple_keywords: s(&[
                "what is", "define", "translate", "hello", "yes or no", "capital of",
                "\u{4ec0}\u{4e48}\u{662f}", "\u{5b9a}\u{4e49}", "\u{7ffb}\u{8bd1}", "\u{4f60}\u{597d}",
                "\u{3068}\u{306f}", "\u{5b9a}\u{7fa9}",
                "\u{0447}\u{0442}\u{043e} \u{0442}\u{0430}\u{043a}\u{043e}\u{0435}", "\u{043f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}",
                "was ist", "hallo",
            ]),
            technical_keywords: s(&[
                "algorithm", "optimize", "architecture", "distributed", "kubernetes", "microservice",
                "\u{7b97}\u{6cd5}", "\u{4f18}\u{5316}", "\u{67b6}\u{6784}", "\u{5206}\u{5e03}\u{5f0f}",
                "\u{30a2}\u{30eb}\u{30b4}\u{30ea}\u{30ba}\u{30e0}",
                "\u{0430}\u{043b}\u{0433}\u{043e}\u{0440}\u{0438}\u{0442}\u{043c}",
                "algorithmus",
            ]),
            creative_keywords: s(&[
                "story", "poem", "compose", "brainstorm", "creative", "imagine",
                "\u{6545}\u{4e8b}", "\u{8bd7}", "\u{521b}\u{4f5c}",
                "\u{7269}\u{8a9e}", "\u{8a69}",
                "\u{0438}\u{0441}\u{0442}\u{043e}\u{0440}\u{0438}\u{044f}",
                "geschichte",
            ]),
            imperative_verbs: s(&[
                "build", "create", "implement", "design", "develop", "deploy",
                "\u{6784}\u{5efa}", "\u{521b}\u{5efa}", "\u{5b9e}\u{73b0}", "\u{8bbe}\u{8ba1}",
                "\u{69cb}\u{7bc9}", "\u{4f5c}\u{6210}",
                "\u{0441}\u{043e}\u{0437}\u{0434}\u{0430}\u{0442}\u{044c}",
                "erstellen",
            ]),
            constraint_indicators: s(&[
                "under", "at most", "at least", "within", "maximum", "minimum",
                "\u{4e0d}\u{8d85}\u{8fc7}", "\u{81f3}\u{5c11}",
                "\u{4ee5}\u{4e0b}", "\u{6700}\u{5927}",
                "\u{043d}\u{0435} \u{0431}\u{043e}\u{043b}\u{0435}\u{0435}",
                "h\u{00f6}chstens",
            ]),
            output_format_keywords: s(&[
                "json", "yaml", "xml", "table", "csv", "markdown", "schema",
                "\u{8868}\u{683c}", "\u{30c6}\u{30fc}\u{30d6}\u{30eb}",
                "\u{0442}\u{0430}\u{0431}\u{043b}\u{0438}\u{0446}\u{0430}",
                "tabelle",
            ]),
            reference_keywords: s(&[
                "above", "below", "previous", "the docs", "the code",
                "\u{4e0a}\u{9762}", "\u{4e0b}\u{9762}", "\u{6587}\u{6863}",
                "\u{4e0a}\u{8a18}",
                "\u{0432}\u{044b}\u{0448}\u{0435}",
                "oben",
            ]),
            negation_keywords: s(&[
                "don't", "do not", "avoid", "never", "without", "except",
                "\u{4e0d}\u{8981}", "\u{907f}\u{514d}",
                "\u{3057}\u{306a}\u{3044}\u{3067}",
                "\u{043d}\u{0435} \u{0434}\u{0435}\u{043b}\u{0430}\u{0439}",
                "nicht",
            ]),
            domain_specific_keywords: s(&[
                "quantum", "fpga", "vlsi", "risc-v", "genomics",
                "\u{91cf}\u{5b50}", "\u{5149}\u{5b50}\u{5b66}",
                "\u{91cf}\u{5b50}",
                "\u{043a}\u{0432}\u{0430}\u{043d}\u{0442}\u{043e}\u{0432}\u{044b}\u{0439}",
                "quanten",
            ]),
            agentic_task_keywords: s(&[
                "read file", "look at", "check the", "edit", "modify", "execute", "deploy",
                "fix", "debug", "iterate", "verify",
                "\u{8bfb}\u{53d6}\u{6587}\u{4ef6}", "\u{67e5}\u{770b}", "\u{7f16}\u{8f91}", "\u{4fee}\u{590d}",
            ]),
            dimension_weights: weights(),
            tier_boundaries: (0.0, 0.30, 0.5),
            confidence_steepness: 12.0,
            confidence_threshold: 0.7,
        },
        tiers: tier_map(&[
            (Tier::Simple, "nvidia/kimi-k2.5", &["google/gemini-2.5-flash"]),
            (Tier::Medium, "xai/grok-code-fast-1", &["deepseek/deepseek-chat"]),
            (Tier::Complex, "google/gemini-3-pro-preview", &["anthropic/claude-sonnet-4"]),
            (Tier::Reasoning, "xai/grok-4-1-fast-reasoning", &["openai/o3"]),
        ]),
        eco_tiers: tier_map(&[
            (Tier::Simple, "nvidia/kimi-k2.5", &["deepseek/deepseek-chat"]),
            (Tier::Medium, "deepseek/deepseek-chat", &["google/gemini-2.5-flash"]),
            (Tier::Complex, "xai/grok-4-0709", &["deepseek/deepseek-chat"]),
            (Tier::Reasoning, "deepseek/deepseek-reasoner", &["xai/grok-4-fast-reasoning"]),
        ]),
        premium_tiers: tier_map(&[
            (Tier::Simple, "google/gemini-2.5-flash", &["openai/gpt-4o-mini"]),
            (Tier::Medium, "openai/gpt-4o", &["anthropic/claude-sonnet-4"]),
            (Tier::Complex, "anthropic/claude-opus-4.5", &["openai/gpt-5.2-pro"]),
            (Tier::Reasoning, "openai/o3", &["anthropic/claude-opus-4.5"]),
        ]),
        agentic_tiers: tier_map(&[
            (Tier::Simple, "moonshot/kimi-k2.5", &["anthropic/claude-haiku-4.5"]),
            (Tier::Medium, "xai/grok-code-fast-1", &["anthropic/claude-haiku-4.5"]),
            (Tier::Complex, "anthropic/claude-sonnet-4", &["anthropic/claude-opus-4.5"]),
            (Tier::Reasoning, "anthropic/claude-sonnet-4", &["anthropic/claude-opus-4.5"]),
        ]),
        overrides: OverridesConfig {
            max_tokens_force_complex: 100_000,
            structured_output_min_tier: Tier::Medium,
            ambiguous_default_tier: Tier::Medium,
        },
    }
}

/// The default config instance.
pub static ROUTING_CONFIG: std::sync::LazyLock<RoutingConfig> = std::sync::LazyLock::new(default_routing_config);
