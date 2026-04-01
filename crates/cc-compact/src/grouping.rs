/// Represents a group of messages that can be compacted together.
#[derive(Debug, Clone)]
pub struct MessageGroup {
    pub start_index: usize,
    pub end_index: usize,
    pub estimated_tokens: usize,
    pub is_compactable: bool,
}

/// Group consecutive messages for compaction.
///
/// Creates groups of messages where each group's total tokens stays under
/// `max_tokens_per_group` (default 10000). Groups at the very end
/// (the last 2 messages) are marked as non-compactable to preserve
/// recent context.
pub fn group_messages(message_count: usize, token_estimates: &[usize]) -> Vec<MessageGroup> {
    if message_count == 0 || token_estimates.is_empty() {
        return vec![];
    }

    let count = message_count.min(token_estimates.len());
    let max_tokens_per_group = 10_000;
    let non_compactable_tail = 2;

    let mut groups = Vec::new();
    let mut start = 0;
    let mut current_tokens = 0;

    for i in 0..count {
        if current_tokens + token_estimates[i] > max_tokens_per_group && i > start {
            groups.push(MessageGroup {
                start_index: start,
                end_index: i - 1,
                estimated_tokens: current_tokens,
                is_compactable: true,
            });
            start = i;
            current_tokens = 0;
        }
        current_tokens += token_estimates[i];
    }

    // Push the final group
    if start < count {
        groups.push(MessageGroup {
            start_index: start,
            end_index: count - 1,
            estimated_tokens: current_tokens,
            is_compactable: true,
        });
    }

    // Mark the last `non_compactable_tail` groups as non-compactable
    let len = groups.len();
    for group in groups.iter_mut().skip(len.saturating_sub(non_compactable_tail)) {
        group.is_compactable = false;
    }

    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let groups = group_messages(0, &[]);
        assert!(groups.is_empty());
    }

    #[test]
    fn test_single_message() {
        let groups = group_messages(1, &[100]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].start_index, 0);
        assert_eq!(groups[0].end_index, 0);
        assert!(!groups[0].is_compactable); // last group is non-compactable
    }

    #[test]
    fn test_small_messages_single_group() {
        let tokens = vec![100, 200, 300];
        let groups = group_messages(3, &tokens);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].estimated_tokens, 600);
    }

    #[test]
    fn test_large_messages_split_into_groups() {
        let tokens = vec![5000, 5000, 5000, 5000, 1000];
        let groups = group_messages(5, &tokens);
        assert!(groups.len() >= 2);
        // Last two groups should be non-compactable
        let len = groups.len();
        for g in &groups[len.saturating_sub(2)..] {
            assert!(!g.is_compactable);
        }
    }

    #[test]
    fn test_many_groups_compactable_head() {
        // 10 messages of 5000 tokens each -> multiple groups
        let tokens = vec![5000; 10];
        let groups = group_messages(10, &tokens);
        assert!(groups.len() >= 3);
        // First groups should be compactable
        assert!(groups[0].is_compactable);
    }

    #[test]
    fn test_indices_cover_all_messages() {
        let tokens = vec![3000, 4000, 5000, 2000, 6000, 1000];
        let groups = group_messages(6, &tokens);
        // Verify all messages are covered
        assert_eq!(groups.first().unwrap().start_index, 0);
        assert_eq!(groups.last().unwrap().end_index, 5);
        // Verify no gaps
        for w in groups.windows(2) {
            assert_eq!(w[1].start_index, w[0].end_index + 1);
        }
    }
}
