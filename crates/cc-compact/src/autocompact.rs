/// Configuration for auto-compaction
#[derive(Debug, Clone)]
pub struct AutoCompactConfig {
    pub token_threshold: usize,
    pub target_tokens: usize,
    pub min_messages_to_keep: usize,
}

impl Default for AutoCompactConfig {
    fn default() -> Self {
        Self {
            token_threshold: 90_000,
            target_tokens: 60_000,
            min_messages_to_keep: 4,
        }
    }
}

/// Check if compaction should be triggered
pub fn should_compact(total_tokens: usize, config: &AutoCompactConfig) -> bool {
    total_tokens >= config.token_threshold
}

/// Determine how many messages to compact.
///
/// Works backward from the end, keeping messages until we have accumulated
/// enough tokens to stay under the target. Returns the number of messages
/// from the beginning that should be compacted.
pub fn messages_to_compact(
    total_messages: usize,
    token_estimates: &[usize],
    config: &AutoCompactConfig,
) -> usize {
    if total_messages <= config.min_messages_to_keep {
        return 0;
    }

    let count = total_messages.min(token_estimates.len());
    let total_tokens: usize = token_estimates[..count].iter().sum();

    if total_tokens <= config.target_tokens {
        return 0;
    }

    // Keep messages from the end until we reach target_tokens
    let mut kept_tokens = 0;
    let mut keep_count = 0;

    for i in (0..count).rev() {
        if kept_tokens + token_estimates[i] > config.target_tokens
            && keep_count >= config.min_messages_to_keep
        {
            break;
        }
        kept_tokens += token_estimates[i];
        keep_count += 1;
    }

    // Ensure we keep at least min_messages_to_keep
    keep_count = keep_count.max(config.min_messages_to_keep);
    if keep_count >= count {
        return 0;
    }

    count - keep_count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AutoCompactConfig::default();
        assert_eq!(config.token_threshold, 90_000);
        assert_eq!(config.target_tokens, 60_000);
        assert_eq!(config.min_messages_to_keep, 4);
    }

    #[test]
    fn test_should_compact_below_threshold() {
        let config = AutoCompactConfig::default();
        assert!(!should_compact(50_000, &config));
    }

    #[test]
    fn test_should_compact_at_threshold() {
        let config = AutoCompactConfig::default();
        assert!(should_compact(90_000, &config));
    }

    #[test]
    fn test_should_compact_above_threshold() {
        let config = AutoCompactConfig::default();
        assert!(should_compact(100_000, &config));
    }

    #[test]
    fn test_messages_to_compact_under_target() {
        let config = AutoCompactConfig::default();
        let tokens = vec![1000; 10]; // 10k total, well under 60k target
        assert_eq!(messages_to_compact(10, &tokens, &config), 0);
    }

    #[test]
    fn test_messages_to_compact_over_target() {
        let config = AutoCompactConfig {
            token_threshold: 100,
            target_tokens: 50,
            min_messages_to_keep: 2,
        };
        let tokens = vec![20, 20, 20, 20, 20]; // 100 total, target 50
        let to_compact = messages_to_compact(5, &tokens, &config);
        // Should compact some messages from the front
        assert!(to_compact > 0);
        assert!(to_compact <= 3); // must keep at least 2
    }

    #[test]
    fn test_messages_to_compact_too_few_messages() {
        let config = AutoCompactConfig {
            token_threshold: 10,
            target_tokens: 5,
            min_messages_to_keep: 4,
        };
        let tokens = vec![10, 10, 10]; // only 3 messages, min_messages_to_keep is 4
        assert_eq!(messages_to_compact(3, &tokens, &config), 0);
    }

    #[test]
    fn test_messages_to_compact_preserves_min_messages() {
        let config = AutoCompactConfig {
            token_threshold: 100,
            target_tokens: 10,
            min_messages_to_keep: 4,
        };
        let tokens = vec![100; 6]; // 600 total, target 10, but must keep 4
        let to_compact = messages_to_compact(6, &tokens, &config);
        assert!(to_compact <= 2); // 6 - 4 = at most 2 can be compacted
    }

    #[test]
    fn test_messages_to_compact_empty() {
        let config = AutoCompactConfig::default();
        assert_eq!(messages_to_compact(0, &[], &config), 0);
    }
}
