/// Cost rates for a model (per token in USD).
#[derive(Debug, Clone)]
pub struct ModelCostRates {
    pub input_per_token: f64,
    pub output_per_token: f64,
    pub cache_read_per_token: f64,
    pub cache_creation_per_token: f64,
}

// Helper: convert $/MTok to $/token
const fn per_mtok(dollars: f64) -> f64 {
    dollars / 1_000_000.0
}

/// Rates for Claude Opus 4.
fn opus_rates() -> ModelCostRates {
    ModelCostRates {
        input_per_token: per_mtok(15.0),
        output_per_token: per_mtok(75.0),
        cache_read_per_token: per_mtok(1.50),
        cache_creation_per_token: per_mtok(18.75),
    }
}

/// Rates for Claude Sonnet 4.
fn sonnet_rates() -> ModelCostRates {
    ModelCostRates {
        input_per_token: per_mtok(3.0),
        output_per_token: per_mtok(15.0),
        cache_read_per_token: per_mtok(0.30),
        cache_creation_per_token: per_mtok(3.75),
    }
}

/// Rates for Claude Haiku 4.5.
fn haiku_rates() -> ModelCostRates {
    ModelCostRates {
        input_per_token: per_mtok(0.80),
        output_per_token: per_mtok(4.0),
        cache_read_per_token: per_mtok(0.08),
        cache_creation_per_token: per_mtok(1.0),
    }
}

/// Get cost rates for a known model.
///
/// First tries exact matches, then falls back to partial matching on
/// "opus", "sonnet", "haiku" substrings.
pub fn get_cost_rates(model_id: &str) -> Option<ModelCostRates> {
    // Exact matches
    match model_id {
        "claude-opus-4-6" => return Some(opus_rates()),
        "claude-sonnet-4-6" => return Some(sonnet_rates()),
        "claude-haiku-4-5-20251001" => return Some(haiku_rates()),
        _ => {}
    }

    // Partial / fuzzy matches
    let lower = model_id.to_lowercase();
    if lower.contains("opus") {
        Some(opus_rates())
    } else if lower.contains("sonnet") {
        Some(sonnet_rates())
    } else if lower.contains("haiku") {
        Some(haiku_rates())
    } else {
        None
    }
}

/// Calculate the USD cost from token counts for a given model.
///
/// Returns 0.0 if the model is unknown.
pub fn calculate_cost(
    model_id: &str,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_creation_tokens: u64,
) -> f64 {
    let rates = match get_cost_rates(model_id) {
        Some(r) => r,
        None => return 0.0,
    };

    (input_tokens as f64) * rates.input_per_token
        + (output_tokens as f64) * rates.output_per_token
        + (cache_read_tokens as f64) * rates.cache_read_per_token
        + (cache_creation_tokens as f64) * rates.cache_creation_per_token
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opus_exact_match() {
        let rates = get_cost_rates("claude-opus-4-6").unwrap();
        assert!((rates.input_per_token - 15.0 / 1_000_000.0).abs() < 1e-15);
        assert!((rates.output_per_token - 75.0 / 1_000_000.0).abs() < 1e-15);
    }

    #[test]
    fn sonnet_exact_match() {
        let rates = get_cost_rates("claude-sonnet-4-6").unwrap();
        assert!((rates.input_per_token - 3.0 / 1_000_000.0).abs() < 1e-15);
        assert!((rates.output_per_token - 15.0 / 1_000_000.0).abs() < 1e-15);
    }

    #[test]
    fn haiku_exact_match() {
        let rates = get_cost_rates("claude-haiku-4-5-20251001").unwrap();
        assert!((rates.input_per_token - 0.80 / 1_000_000.0).abs() < 1e-15);
        assert!((rates.output_per_token - 4.0 / 1_000_000.0).abs() < 1e-15);
    }

    #[test]
    fn partial_match_opus() {
        let rates = get_cost_rates("claude-opus-4-20250514").unwrap();
        assert!((rates.input_per_token - 15.0 / 1_000_000.0).abs() < 1e-15);
    }

    #[test]
    fn partial_match_sonnet() {
        let rates = get_cost_rates("my-custom-sonnet-model").unwrap();
        assert!((rates.input_per_token - 3.0 / 1_000_000.0).abs() < 1e-15);
    }

    #[test]
    fn partial_match_haiku() {
        let rates = get_cost_rates("claude-haiku-3-latest").unwrap();
        assert!((rates.input_per_token - 0.80 / 1_000_000.0).abs() < 1e-15);
    }

    #[test]
    fn unknown_model_returns_none() {
        assert!(get_cost_rates("gpt-4").is_none());
    }

    #[test]
    fn calculate_cost_opus() {
        let cost = calculate_cost("claude-opus-4-6", 1_000_000, 1_000_000, 0, 0);
        // $15 input + $75 output = $90
        assert!((cost - 90.0).abs() < 1e-6);
    }

    #[test]
    fn calculate_cost_sonnet_with_cache() {
        let cost = calculate_cost("claude-sonnet-4-6", 500_000, 100_000, 200_000, 50_000);
        // input: 500k * 3/1M = 1.5
        // output: 100k * 15/1M = 1.5
        // cache_read: 200k * 0.30/1M = 0.06
        // cache_creation: 50k * 3.75/1M = 0.1875
        let expected = 1.5 + 1.5 + 0.06 + 0.1875;
        assert!((cost - expected).abs() < 1e-6);
    }

    #[test]
    fn calculate_cost_haiku() {
        let cost = calculate_cost("claude-haiku-4-5-20251001", 1_000_000, 1_000_000, 0, 0);
        // $0.80 input + $4 output = $4.80
        assert!((cost - 4.80).abs() < 1e-6);
    }

    #[test]
    fn calculate_cost_unknown_model_returns_zero() {
        let cost = calculate_cost("unknown-model", 1_000_000, 1_000_000, 0, 0);
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn calculate_cost_zero_tokens() {
        let cost = calculate_cost("claude-opus-4-6", 0, 0, 0, 0);
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn calculate_cost_only_cache_read() {
        let cost = calculate_cost("claude-opus-4-6", 0, 0, 1_000_000, 0);
        assert!((cost - 1.50).abs() < 1e-6);
    }

    #[test]
    fn calculate_cost_only_cache_creation() {
        let cost = calculate_cost("claude-opus-4-6", 0, 0, 0, 1_000_000);
        assert!((cost - 18.75).abs() < 1e-6);
    }
}
