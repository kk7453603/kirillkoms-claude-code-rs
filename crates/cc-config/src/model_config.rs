use cc_types::config::ModelConfig;
use cc_types::cost::ModelUsage;

pub const CLAUDE_OPUS: &str = "claude-opus-4-6";
pub const CLAUDE_SONNET: &str = "claude-sonnet-4-6";
pub const CLAUDE_HAIKU: &str = "claude-haiku-4-5-20251001";

/// Static table of known Claude models with their configurations.
fn model_table() -> Vec<ModelConfig> {
    vec![
        ModelConfig {
            id: CLAUDE_OPUS.to_string(),
            name: "Claude Opus 4".to_string(),
            context_window: 200_000,
            max_output_tokens: 32_768,
            cost_per_input_token: 15.0 / 1_000_000.0,
            cost_per_output_token: 75.0 / 1_000_000.0,
            cost_per_cache_read_token: 1.5 / 1_000_000.0,
            cost_per_cache_creation_token: 18.75 / 1_000_000.0,
            supports_thinking: true,
            supports_images: true,
        },
        ModelConfig {
            id: CLAUDE_SONNET.to_string(),
            name: "Claude Sonnet 4".to_string(),
            context_window: 200_000,
            max_output_tokens: 16_384,
            cost_per_input_token: 3.0 / 1_000_000.0,
            cost_per_output_token: 15.0 / 1_000_000.0,
            cost_per_cache_read_token: 0.3 / 1_000_000.0,
            cost_per_cache_creation_token: 3.75 / 1_000_000.0,
            supports_thinking: true,
            supports_images: true,
        },
        ModelConfig {
            id: CLAUDE_HAIKU.to_string(),
            name: "Claude Haiku 4.5".to_string(),
            context_window: 200_000,
            max_output_tokens: 8_192,
            cost_per_input_token: 0.80 / 1_000_000.0,
            cost_per_output_token: 4.0 / 1_000_000.0,
            cost_per_cache_read_token: 0.08 / 1_000_000.0,
            cost_per_cache_creation_token: 1.0 / 1_000_000.0,
            supports_thinking: false,
            supports_images: true,
        },
    ]
}

/// Get model config by model ID.
pub fn get_model_config(model_id: &str) -> Option<ModelConfig> {
    model_table().into_iter().find(|m| m.id == model_id)
}

/// Get default model ID.
pub fn default_model() -> &'static str {
    CLAUDE_SONNET
}

/// Resolve a model alias to a full model ID.
///
/// Supported aliases:
/// - "opus" -> claude-opus-4-6
/// - "sonnet" -> claude-sonnet-4-6
/// - "haiku" -> claude-haiku-4-5-20251001
///
/// Also accepts full model IDs directly.
pub fn resolve_model_alias(alias: &str) -> Option<&'static str> {
    match alias.to_lowercase().as_str() {
        "opus" | "claude-opus" => Some(CLAUDE_OPUS),
        "sonnet" | "claude-sonnet" => Some(CLAUDE_SONNET),
        "haiku" | "claude-haiku" => Some(CLAUDE_HAIKU),
        s if s == CLAUDE_OPUS => Some(CLAUDE_OPUS),
        s if s == CLAUDE_SONNET => Some(CLAUDE_SONNET),
        s if s == CLAUDE_HAIKU => Some(CLAUDE_HAIKU),
        _ => None,
    }
}

/// Get all known model IDs.
pub fn known_models() -> Vec<&'static str> {
    vec![CLAUDE_OPUS, CLAUDE_SONNET, CLAUDE_HAIKU]
}

/// Calculate cost for given token usage.
///
/// Returns the cost in USD. Returns 0.0 if the model is unknown.
pub fn calculate_cost(model_id: &str, usage: &ModelUsage) -> f64 {
    let config = match get_model_config(model_id) {
        Some(c) => c,
        None => return 0.0,
    };

    let input_cost = usage.input_tokens as f64 * config.cost_per_input_token;
    let output_cost = usage.output_tokens as f64 * config.cost_per_output_token;
    let cache_read_cost = usage.cache_read_input_tokens as f64 * config.cost_per_cache_read_token;
    let cache_creation_cost =
        usage.cache_creation_input_tokens as f64 * config.cost_per_cache_creation_token;

    input_cost + output_cost + cache_read_cost + cache_creation_cost
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_opus_config() {
        let config = get_model_config(CLAUDE_OPUS).unwrap();
        assert_eq!(config.id, CLAUDE_OPUS);
        assert_eq!(config.name, "Claude Opus 4");
        assert_eq!(config.context_window, 200_000);
        assert!(config.supports_thinking);
        assert!(config.supports_images);
    }

    #[test]
    fn get_sonnet_config() {
        let config = get_model_config(CLAUDE_SONNET).unwrap();
        assert_eq!(config.id, CLAUDE_SONNET);
        assert_eq!(config.name, "Claude Sonnet 4");
        assert!(config.supports_thinking);
    }

    #[test]
    fn get_haiku_config() {
        let config = get_model_config(CLAUDE_HAIKU).unwrap();
        assert_eq!(config.id, CLAUDE_HAIKU);
        assert_eq!(config.name, "Claude Haiku 4.5");
        assert!(!config.supports_thinking);
        assert!(config.supports_images);
    }

    #[test]
    fn get_unknown_model_returns_none() {
        assert!(get_model_config("claude-unknown-model").is_none());
    }

    #[test]
    fn default_model_is_sonnet() {
        assert_eq!(default_model(), CLAUDE_SONNET);
    }

    #[test]
    fn resolve_alias_opus() {
        assert_eq!(resolve_model_alias("opus"), Some(CLAUDE_OPUS));
        assert_eq!(resolve_model_alias("Opus"), Some(CLAUDE_OPUS));
        assert_eq!(resolve_model_alias("claude-opus"), Some(CLAUDE_OPUS));
    }

    #[test]
    fn resolve_alias_sonnet() {
        assert_eq!(resolve_model_alias("sonnet"), Some(CLAUDE_SONNET));
        assert_eq!(resolve_model_alias("SONNET"), Some(CLAUDE_SONNET));
        assert_eq!(resolve_model_alias("claude-sonnet"), Some(CLAUDE_SONNET));
    }

    #[test]
    fn resolve_alias_haiku() {
        assert_eq!(resolve_model_alias("haiku"), Some(CLAUDE_HAIKU));
        assert_eq!(resolve_model_alias("claude-haiku"), Some(CLAUDE_HAIKU));
    }

    #[test]
    fn resolve_full_model_id() {
        assert_eq!(resolve_model_alias(CLAUDE_OPUS), Some(CLAUDE_OPUS));
        assert_eq!(resolve_model_alias(CLAUDE_SONNET), Some(CLAUDE_SONNET));
        assert_eq!(resolve_model_alias(CLAUDE_HAIKU), Some(CLAUDE_HAIKU));
    }

    #[test]
    fn resolve_unknown_alias() {
        assert_eq!(resolve_model_alias("gpt-4"), None);
        assert_eq!(resolve_model_alias(""), None);
        assert_eq!(resolve_model_alias("claude-unknown"), None);
    }

    #[test]
    fn known_models_contains_all() {
        let models = known_models();
        assert_eq!(models.len(), 3);
        assert!(models.contains(&CLAUDE_OPUS));
        assert!(models.contains(&CLAUDE_SONNET));
        assert!(models.contains(&CLAUDE_HAIKU));
    }

    #[test]
    fn calculate_cost_opus() {
        let usage = ModelUsage {
            input_tokens: 1_000_000,
            output_tokens: 100_000,
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
            web_search_requests: 0,
            cost_usd: 0.0,
        };
        let cost = calculate_cost(CLAUDE_OPUS, &usage);
        // input: 1M * 15/1M = 15.0
        // output: 100K * 75/1M = 7.5
        let expected = 15.0 + 7.5;
        assert!(
            (cost - expected).abs() < 0.001,
            "Expected ~{}, got {}",
            expected,
            cost
        );
    }

    #[test]
    fn calculate_cost_sonnet() {
        let usage = ModelUsage {
            input_tokens: 1_000_000,
            output_tokens: 100_000,
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
            web_search_requests: 0,
            cost_usd: 0.0,
        };
        let cost = calculate_cost(CLAUDE_SONNET, &usage);
        // input: 1M * 3/1M = 3.0
        // output: 100K * 15/1M = 1.5
        let expected = 3.0 + 1.5;
        assert!(
            (cost - expected).abs() < 0.001,
            "Expected ~{}, got {}",
            expected,
            cost
        );
    }

    #[test]
    fn calculate_cost_with_cache() {
        let usage = ModelUsage {
            input_tokens: 100_000,
            output_tokens: 10_000,
            cache_read_input_tokens: 50_000,
            cache_creation_input_tokens: 20_000,
            web_search_requests: 0,
            cost_usd: 0.0,
        };
        let cost = calculate_cost(CLAUDE_OPUS, &usage);
        // input: 100K * 15/1M = 1.5
        // output: 10K * 75/1M = 0.75
        // cache_read: 50K * 1.5/1M = 0.075
        // cache_create: 20K * 18.75/1M = 0.375
        let expected = 1.5 + 0.75 + 0.075 + 0.375;
        assert!(
            (cost - expected).abs() < 0.001,
            "Expected ~{}, got {}",
            expected,
            cost
        );
    }

    #[test]
    fn calculate_cost_zero_usage() {
        let usage = ModelUsage::default();
        let cost = calculate_cost(CLAUDE_OPUS, &usage);
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn calculate_cost_unknown_model() {
        let usage = ModelUsage {
            input_tokens: 1000,
            output_tokens: 500,
            ..Default::default()
        };
        let cost = calculate_cost("unknown-model", &usage);
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn model_config_pricing_sanity() {
        // Opus should be more expensive than Sonnet
        let opus = get_model_config(CLAUDE_OPUS).unwrap();
        let sonnet = get_model_config(CLAUDE_SONNET).unwrap();
        assert!(opus.cost_per_input_token > sonnet.cost_per_input_token);
        assert!(opus.cost_per_output_token > sonnet.cost_per_output_token);

        // Sonnet should be more expensive than Haiku
        let haiku = get_model_config(CLAUDE_HAIKU).unwrap();
        assert!(sonnet.cost_per_input_token > haiku.cost_per_input_token);
        assert!(sonnet.cost_per_output_token > haiku.cost_per_output_token);
    }

    #[test]
    fn model_config_context_windows() {
        for model_id in known_models() {
            let config = get_model_config(model_id).unwrap();
            assert_eq!(config.context_window, 200_000);
            assert!(config.max_output_tokens > 0);
        }
    }
}
