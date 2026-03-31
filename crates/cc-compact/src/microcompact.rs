/// Identify tool result indices that are candidates for micro-compaction.
///
/// Returns indices of messages that are tool results and whose size exceeds
/// `min_size`, making them candidates for replacement with a compact
/// placeholder.
pub fn identify_compactable_results(
    message_count: usize,
    is_tool_result: &[bool],
    result_sizes: &[usize],
    min_size: usize,
) -> Vec<usize> {
    let count = message_count
        .min(is_tool_result.len())
        .min(result_sizes.len());

    let mut compactable = Vec::new();
    for i in 0..count {
        if is_tool_result[i] && result_sizes[i] >= min_size {
            compactable.push(i);
        }
    }
    compactable
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_tool_results() {
        let result = identify_compactable_results(
            3,
            &[false, false, false],
            &[1000, 2000, 3000],
            500,
        );
        assert!(result.is_empty());
    }

    #[test]
    fn test_tool_results_below_min_size() {
        let result = identify_compactable_results(
            3,
            &[true, true, true],
            &[100, 200, 300],
            500,
        );
        assert!(result.is_empty());
    }

    #[test]
    fn test_tool_results_above_min_size() {
        let result = identify_compactable_results(
            4,
            &[false, true, false, true],
            &[100, 1000, 200, 2000],
            500,
        );
        assert_eq!(result, vec![1, 3]);
    }

    #[test]
    fn test_all_compactable() {
        let result = identify_compactable_results(
            3,
            &[true, true, true],
            &[1000, 2000, 3000],
            500,
        );
        assert_eq!(result, vec![0, 1, 2]);
    }

    #[test]
    fn test_empty() {
        let result = identify_compactable_results(0, &[], &[], 500);
        assert!(result.is_empty());
    }

    #[test]
    fn test_mismatched_lengths() {
        // message_count larger than slices - should use the min of all lengths
        let result = identify_compactable_results(
            10,
            &[true, true],
            &[1000, 500],
            500,
        );
        assert_eq!(result, vec![0, 1]);
    }

    #[test]
    fn test_exact_min_size() {
        let result = identify_compactable_results(
            1,
            &[true],
            &[500],
            500,
        );
        assert_eq!(result, vec![0]);
    }
}
