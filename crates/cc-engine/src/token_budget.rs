/// Token budget tracking for context window management.
#[derive(Debug, Clone)]
pub struct TokenBudget {
    pub context_window: usize,
    pub max_output_tokens: usize,
    pub system_prompt_tokens: usize,
    pub reserved_tokens: usize,
}

/// Compaction is triggered when usage exceeds this ratio.
const COMPACTION_THRESHOLD: f64 = 0.8;

impl TokenBudget {
    pub fn new(context_window: usize, max_output_tokens: usize) -> Self {
        Self {
            context_window,
            max_output_tokens,
            system_prompt_tokens: 0,
            reserved_tokens: 1000, // small buffer for overhead
        }
    }

    /// Available tokens for messages (context window minus output, system, and reserved).
    pub fn available_for_messages(&self) -> usize {
        self.context_window
            .saturating_sub(self.max_output_tokens)
            .saturating_sub(self.system_prompt_tokens)
            .saturating_sub(self.reserved_tokens)
    }

    /// Check usage ratio: current_tokens / available_for_messages.
    pub fn usage_ratio(&self, current_tokens: usize) -> f64 {
        let available = self.available_for_messages();
        if available == 0 {
            return 1.0;
        }
        current_tokens as f64 / available as f64
    }

    /// Whether compaction should be triggered.
    pub fn should_compact(&self, current_tokens: usize) -> bool {
        self.usage_ratio(current_tokens) >= COMPACTION_THRESHOLD
    }

    /// Tokens remaining before hitting the available-for-messages limit.
    pub fn remaining(&self, current_tokens: usize) -> usize {
        self.available_for_messages().saturating_sub(current_tokens)
    }
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self::new(200_000, 16_384)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_budget() {
        let budget = TokenBudget::default();
        assert_eq!(budget.context_window, 200_000);
        assert_eq!(budget.max_output_tokens, 16_384);
        assert_eq!(budget.system_prompt_tokens, 0);
        assert_eq!(budget.reserved_tokens, 1000);
    }

    #[test]
    fn test_available_for_messages() {
        let budget = TokenBudget {
            context_window: 100_000,
            max_output_tokens: 10_000,
            system_prompt_tokens: 5_000,
            reserved_tokens: 1_000,
        };
        // 100000 - 10000 - 5000 - 1000 = 84000
        assert_eq!(budget.available_for_messages(), 84_000);
    }

    #[test]
    fn test_available_for_messages_saturating() {
        let budget = TokenBudget {
            context_window: 100,
            max_output_tokens: 50,
            system_prompt_tokens: 40,
            reserved_tokens: 20,
        };
        // 100 - 50 - 40 - 20 = 0 (would underflow)
        assert_eq!(budget.available_for_messages(), 0);
    }

    #[test]
    fn test_usage_ratio() {
        let budget = TokenBudget {
            context_window: 100_000,
            max_output_tokens: 0,
            system_prompt_tokens: 0,
            reserved_tokens: 0,
        };
        let ratio = budget.usage_ratio(50_000);
        assert!((ratio - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_usage_ratio_zero_available() {
        let budget = TokenBudget {
            context_window: 100,
            max_output_tokens: 100,
            system_prompt_tokens: 100,
            reserved_tokens: 100,
        };
        assert_eq!(budget.usage_ratio(10), 1.0);
    }

    #[test]
    fn test_should_compact_below_threshold() {
        let budget = TokenBudget::new(100_000, 10_000);
        // available = 100000 - 10000 - 0 - 1000 = 89000
        // 50000 / 89000 ~ 0.56 < 0.8
        assert!(!budget.should_compact(50_000));
    }

    #[test]
    fn test_should_compact_above_threshold() {
        let budget = TokenBudget::new(100_000, 10_000);
        // available = 89000
        // 80000 / 89000 ~ 0.899 > 0.8
        assert!(budget.should_compact(80_000));
    }

    #[test]
    fn test_should_compact_at_threshold() {
        let budget = TokenBudget {
            context_window: 100,
            max_output_tokens: 0,
            system_prompt_tokens: 0,
            reserved_tokens: 0,
        };
        // 80 / 100 = 0.8, exactly at threshold
        assert!(budget.should_compact(80));
    }

    #[test]
    fn test_remaining() {
        let budget = TokenBudget {
            context_window: 100_000,
            max_output_tokens: 10_000,
            system_prompt_tokens: 0,
            reserved_tokens: 0,
        };
        // available = 90000
        assert_eq!(budget.remaining(50_000), 40_000);
    }

    #[test]
    fn test_remaining_saturating() {
        let budget = TokenBudget {
            context_window: 100_000,
            max_output_tokens: 10_000,
            system_prompt_tokens: 0,
            reserved_tokens: 0,
        };
        // available = 90000, current = 100000 -> 0
        assert_eq!(budget.remaining(100_000), 0);
    }

    #[test]
    fn test_with_system_prompt_tokens() {
        let mut budget = TokenBudget::new(200_000, 16_384);
        budget.system_prompt_tokens = 5_000;
        // 200000 - 16384 - 5000 - 1000 = 177616
        assert_eq!(budget.available_for_messages(), 177_616);
    }
}
