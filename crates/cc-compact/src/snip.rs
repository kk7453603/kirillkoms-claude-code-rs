const SNIP_PREFIX: &str = "[SNIP] ";

/// Create a snip marker message.
///
/// The marker wraps a summary so the system knows this message replaced
/// compacted content.
pub fn create_snip_marker(summary: &str) -> String {
    format!("{}{}", SNIP_PREFIX, summary)
}

/// Check if a message is a snip marker.
pub fn is_snip_marker(text: &str) -> bool {
    text.starts_with(SNIP_PREFIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_snip_marker() {
        let marker = create_snip_marker("User discussed error handling");
        assert_eq!(marker, "[SNIP] User discussed error handling");
    }

    #[test]
    fn test_is_snip_marker_true() {
        assert!(is_snip_marker("[SNIP] some summary"));
    }

    #[test]
    fn test_is_snip_marker_false() {
        assert!(!is_snip_marker("regular message"));
        assert!(!is_snip_marker("[snip] lowercase"));
        assert!(!is_snip_marker(""));
    }

    #[test]
    fn test_roundtrip() {
        let summary = "The user refactored the auth module.";
        let marker = create_snip_marker(summary);
        assert!(is_snip_marker(&marker));
    }

    #[test]
    fn test_empty_summary() {
        let marker = create_snip_marker("");
        assert!(is_snip_marker(&marker));
        assert_eq!(marker, "[SNIP] ");
    }
}
