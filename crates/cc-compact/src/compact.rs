/// Build compaction prompt for the LLM.
///
/// Returns a system-style prompt that asks the LLM to summarize the given
/// conversation messages into a compact form.
pub fn build_compaction_prompt(messages_text: &str) -> String {
    format!(
        "Please provide a concise summary of the following conversation. \
         Preserve key decisions, code changes, file paths, and important context. \
         Omit redundant details and verbose tool output.\n\n\
         --- CONVERSATION ---\n\
         {}\n\
         --- END CONVERSATION ---\n\n\
         Provide a compact summary:",
        messages_text
    )
}

/// Parse compacted summary from LLM response.
///
/// Trims whitespace and returns the cleaned summary text.
pub fn parse_compaction_response(response: &str) -> String {
    response.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_compaction_prompt_contains_messages() {
        let prompt = build_compaction_prompt("User asked about Rust lifetimes.");
        assert!(prompt.contains("User asked about Rust lifetimes."));
        assert!(prompt.contains("CONVERSATION"));
        assert!(prompt.contains("summary"));
    }

    #[test]
    fn test_build_compaction_prompt_empty_messages() {
        let prompt = build_compaction_prompt("");
        assert!(prompt.contains("--- CONVERSATION ---"));
        assert!(prompt.contains("--- END CONVERSATION ---"));
    }

    #[test]
    fn test_parse_compaction_response_trims() {
        let result = parse_compaction_response("  summary text  \n");
        assert_eq!(result, "summary text");
    }

    #[test]
    fn test_parse_compaction_response_preserves_content() {
        let input = "The user modified main.rs to add error handling.";
        let result = parse_compaction_response(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_parse_compaction_response_empty() {
        let result = parse_compaction_response("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_parse_compaction_response_multiline() {
        let input = "Line 1\nLine 2\nLine 3";
        let result = parse_compaction_response(input);
        assert_eq!(result, input);
    }
}
