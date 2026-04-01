use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Token usage and cost for a single model invocation or accumulated total.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub web_search_requests: u64,
    pub cost_usd: f64,
}

impl ModelUsage {
    /// Accumulate another usage record into this one.
    pub fn accumulate(&mut self, other: &ModelUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cache_read_input_tokens += other.cache_read_input_tokens;
        self.cache_creation_input_tokens += other.cache_creation_input_tokens;
        self.web_search_requests += other.web_search_requests;
        self.cost_usd += other.cost_usd;
    }

    /// Total tokens (input + output).
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

/// Accumulates usage across multiple models and tracks durations.
#[derive(Debug, Clone, Default)]
pub struct UsageAccumulator {
    pub by_model: HashMap<String, ModelUsage>,
    pub total_api_duration_ms: u64,
    pub total_tool_duration_ms: u64,
}

impl UsageAccumulator {
    /// Record usage for a specific model.
    pub fn record(&mut self, model: &str, usage: &ModelUsage) {
        self.by_model
            .entry(model.to_string())
            .or_default()
            .accumulate(usage);
    }

    /// Get the total usage across all models.
    pub fn total_usage(&self) -> ModelUsage {
        let mut total = ModelUsage::default();
        for usage in self.by_model.values() {
            total.accumulate(usage);
        }
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_usage_default() {
        let usage = ModelUsage::default();
        assert_eq!(usage.input_tokens, 0);
        assert_eq!(usage.output_tokens, 0);
        assert_eq!(usage.total_tokens(), 0);
        assert_eq!(usage.cost_usd, 0.0);
    }

    #[test]
    fn model_usage_total_tokens() {
        let usage = ModelUsage {
            input_tokens: 100,
            output_tokens: 50,
            ..Default::default()
        };
        assert_eq!(usage.total_tokens(), 150);
    }

    #[test]
    fn model_usage_accumulate() {
        let mut a = ModelUsage {
            input_tokens: 100,
            output_tokens: 50,
            cache_read_input_tokens: 10,
            cache_creation_input_tokens: 5,
            web_search_requests: 1,
            cost_usd: 0.01,
        };
        let b = ModelUsage {
            input_tokens: 200,
            output_tokens: 100,
            cache_read_input_tokens: 20,
            cache_creation_input_tokens: 10,
            web_search_requests: 2,
            cost_usd: 0.02,
        };
        a.accumulate(&b);
        assert_eq!(a.input_tokens, 300);
        assert_eq!(a.output_tokens, 150);
        assert_eq!(a.cache_read_input_tokens, 30);
        assert_eq!(a.cache_creation_input_tokens, 15);
        assert_eq!(a.web_search_requests, 3);
        assert!((a.cost_usd - 0.03).abs() < f64::EPSILON);
    }

    #[test]
    fn model_usage_serde_roundtrip() {
        let usage = ModelUsage {
            input_tokens: 500,
            output_tokens: 200,
            cache_read_input_tokens: 50,
            cache_creation_input_tokens: 25,
            web_search_requests: 0,
            cost_usd: 0.005,
        };
        let json = serde_json::to_string(&usage).unwrap();
        let back: ModelUsage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.input_tokens, 500);
        assert_eq!(back.output_tokens, 200);
        assert_eq!(back.total_tokens(), 700);
    }

    #[test]
    fn usage_accumulator_record_and_total() {
        let mut acc = UsageAccumulator::default();
        let u1 = ModelUsage {
            input_tokens: 100,
            output_tokens: 50,
            cost_usd: 0.01,
            ..Default::default()
        };
        let u2 = ModelUsage {
            input_tokens: 200,
            output_tokens: 100,
            cost_usd: 0.02,
            ..Default::default()
        };
        let u3 = ModelUsage {
            input_tokens: 50,
            output_tokens: 25,
            cost_usd: 0.005,
            ..Default::default()
        };
        acc.record("claude-opus-4-20250514", &u1);
        acc.record("claude-sonnet-4-20250514", &u2);
        acc.record("claude-opus-4-20250514", &u3);

        assert_eq!(acc.by_model.len(), 2);
        let opus = acc.by_model.get("claude-opus-4-20250514").unwrap();
        assert_eq!(opus.input_tokens, 150);
        assert_eq!(opus.output_tokens, 75);

        let total = acc.total_usage();
        assert_eq!(total.input_tokens, 350);
        assert_eq!(total.output_tokens, 175);
    }

    #[test]
    fn usage_accumulator_default() {
        let acc = UsageAccumulator::default();
        assert!(acc.by_model.is_empty());
        assert_eq!(acc.total_api_duration_ms, 0);
        assert_eq!(acc.total_tool_duration_ms, 0);
        let total = acc.total_usage();
        assert_eq!(total.total_tokens(), 0);
    }
}
