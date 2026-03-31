use similar::{ChangeTag, TextDiff};

/// Generate a unified diff between two texts.
pub fn unified_diff(old: &str, new: &str, context_lines: usize) -> String {
    let diff = TextDiff::from_lines(old, new);
    let mut output = String::new();

    for hunk in diff.unified_diff().context_radius(context_lines).iter_hunks() {
        output.push_str(&format!("{}", hunk));
    }

    output
}

/// Count lines added and removed.
pub fn diff_stats(old: &str, new: &str) -> DiffStats {
    let diff = TextDiff::from_lines(old, new);
    let mut stats = DiffStats::default();

    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Insert => stats.lines_added += 1,
            ChangeTag::Delete => stats.lines_removed += 1,
            ChangeTag::Equal => {}
        }
    }
    stats.lines_changed = stats.lines_added.min(stats.lines_removed);
    stats
}

#[derive(Debug, Clone, Default)]
pub struct DiffStats {
    pub lines_added: usize,
    pub lines_removed: usize,
    pub lines_changed: usize,
}

/// Apply a string replacement to content.
pub fn apply_edit(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Result<String, EditError> {
    if old_string == new_string {
        return Err(EditError::NoChange);
    }

    let count = content.matches(old_string).count();

    if count == 0 {
        return Err(EditError::NotFound);
    }

    if !replace_all && count > 1 {
        return Err(EditError::MultipleMatches { count });
    }

    if replace_all {
        Ok(content.replace(old_string, new_string))
    } else {
        // Replace only the first occurrence
        Ok(content.replacen(old_string, new_string, 1))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EditError {
    #[error("Old string not found in content")]
    NotFound,
    #[error("Old string found {count} times, expected exactly 1 (use replace_all for multiple)")]
    MultipleMatches { count: usize },
    #[error("New string is the same as old string")]
    NoChange,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_diff_no_changes() {
        let result = unified_diff("hello\n", "hello\n", 3);
        assert!(result.is_empty());
    }

    #[test]
    fn test_unified_diff_addition() {
        let old = "line1\nline2\n";
        let new = "line1\nline2\nline3\n";
        let result = unified_diff(old, new, 3);
        assert!(result.contains("+line3"));
    }

    #[test]
    fn test_unified_diff_removal() {
        let old = "line1\nline2\nline3\n";
        let new = "line1\nline3\n";
        let result = unified_diff(old, new, 3);
        assert!(result.contains("-line2"));
    }

    #[test]
    fn test_unified_diff_modification() {
        let old = "hello world\n";
        let new = "hello rust\n";
        let result = unified_diff(old, new, 3);
        assert!(result.contains("-hello world"));
        assert!(result.contains("+hello rust"));
    }

    #[test]
    fn test_diff_stats_no_changes() {
        let stats = diff_stats("same\n", "same\n");
        assert_eq!(stats.lines_added, 0);
        assert_eq!(stats.lines_removed, 0);
        assert_eq!(stats.lines_changed, 0);
    }

    #[test]
    fn test_diff_stats_additions() {
        let stats = diff_stats("a\n", "a\nb\nc\n");
        assert_eq!(stats.lines_added, 2);
        assert_eq!(stats.lines_removed, 0);
    }

    #[test]
    fn test_diff_stats_removals() {
        let stats = diff_stats("a\nb\nc\n", "a\n");
        assert_eq!(stats.lines_removed, 2);
        assert_eq!(stats.lines_added, 0);
    }

    #[test]
    fn test_diff_stats_changes() {
        let stats = diff_stats("old line\n", "new line\n");
        assert_eq!(stats.lines_added, 1);
        assert_eq!(stats.lines_removed, 1);
        assert_eq!(stats.lines_changed, 1);
    }

    #[test]
    fn test_apply_edit_single_match() {
        let content = "hello world";
        let result = apply_edit(content, "world", "rust", false).unwrap();
        assert_eq!(result, "hello rust");
    }

    #[test]
    fn test_apply_edit_not_found() {
        let content = "hello world";
        let result = apply_edit(content, "xyz", "abc", false);
        assert!(matches!(result, Err(EditError::NotFound)));
    }

    #[test]
    fn test_apply_edit_multiple_matches_error() {
        let content = "aaa bbb aaa";
        let result = apply_edit(content, "aaa", "ccc", false);
        assert!(matches!(result, Err(EditError::MultipleMatches { count: 2 })));
    }

    #[test]
    fn test_apply_edit_replace_all() {
        let content = "aaa bbb aaa";
        let result = apply_edit(content, "aaa", "ccc", true).unwrap();
        assert_eq!(result, "ccc bbb ccc");
    }

    #[test]
    fn test_apply_edit_no_change() {
        let content = "hello world";
        let result = apply_edit(content, "hello", "hello", false);
        assert!(matches!(result, Err(EditError::NoChange)));
    }

    #[test]
    fn test_apply_edit_multiline() {
        let content = "line1\nline2\nline3\n";
        let result = apply_edit(content, "line2\nline3", "lineA\nlineB", false).unwrap();
        assert_eq!(result, "line1\nlineA\nlineB\n");
    }

    #[test]
    fn test_apply_edit_replace_all_single() {
        let content = "hello world";
        let result = apply_edit(content, "world", "rust", true).unwrap();
        assert_eq!(result, "hello rust");
    }
}
