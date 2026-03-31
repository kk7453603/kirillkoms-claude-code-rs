use cc_types::content::ContentBlock;
use cc_types::message::Message;

/// Estimate token count for text (approximate: ~4 chars per token).
pub fn estimate_tokens(text: &str) -> usize {
    // A rough heuristic: ~4 characters per token for English text.
    // This accounts for whitespace and common subword patterns.
    let char_count = text.len();
    (char_count + 3) / 4 // ceiling division
}

/// Estimate token count for a message.
pub fn estimate_message_tokens(message: &Message) -> usize {
    match message {
        Message::User(user) => estimate_content_blocks(&user.message),
        Message::Assistant(assistant) => estimate_content_blocks(&assistant.message),
        Message::System(system) => estimate_tokens(&system.message),
        Message::Result(result) => {
            let json_str = result.content.to_string();
            estimate_tokens(&json_str)
        }
        Message::Progress(progress) => {
            let json_str = progress.content.to_string();
            estimate_tokens(&json_str)
        }
    }
}

fn estimate_content_blocks(blocks: &[ContentBlock]) -> usize {
    blocks
        .iter()
        .map(|block| match block {
            ContentBlock::Text { text } => estimate_tokens(text),
            ContentBlock::Image { .. } => {
                // Images typically use a fixed token budget (~1600 tokens for a typical image)
                1600
            }
            ContentBlock::ToolUse { name, input, .. } => {
                estimate_tokens(name) + estimate_tokens(&input.to_string())
            }
            ContentBlock::ToolResult { content, .. } => match content {
                cc_types::content::ToolResultContent::Text(text) => estimate_tokens(text),
                cc_types::content::ToolResultContent::Blocks(inner_blocks) => {
                    estimate_content_blocks(inner_blocks)
                }
            },
            ContentBlock::Thinking { thinking, .. } => estimate_tokens(thinking),
        })
        .sum()
}

/// Check if content exceeds token budget.
pub fn exceeds_budget(text: &str, budget: usize) -> bool {
    estimate_tokens(text) > budget
}

/// Truncate text to approximately fit within token budget.
/// Truncates on a character boundary and appends "..." if truncated.
pub fn truncate_to_tokens(text: &str, max_tokens: usize) -> String {
    if max_tokens == 0 {
        return String::new();
    }

    let estimated = estimate_tokens(text);
    if estimated <= max_tokens {
        return text.to_string();
    }

    // Target character count: max_tokens * 4, minus room for "..."
    let target_chars = max_tokens * 4;
    let target_chars = target_chars.saturating_sub(3); // room for "..."

    if target_chars == 0 {
        return "...".to_string();
    }

    // Find a valid char boundary at or before target_chars
    let mut end = target_chars.min(text.len());
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }

    let mut result = text[..end].to_string();
    result.push_str("...");
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_types::content::ContentBlock;
    use cc_types::message::{UserMessage, AssistantMessage, SystemMessage};

    #[test]
    fn test_estimate_tokens_empty() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_estimate_tokens_short() {
        // 4 chars = ~1 token
        assert_eq!(estimate_tokens("abcd"), 1);
    }

    #[test]
    fn test_estimate_tokens_medium() {
        // 100 chars => ~25 tokens
        let text = "a".repeat(100);
        assert_eq!(estimate_tokens(&text), 25);
    }

    #[test]
    fn test_estimate_tokens_not_multiple_of_4() {
        // 5 chars => ceil(5/4) = 2
        assert_eq!(estimate_tokens("abcde"), 2);
    }

    #[test]
    fn test_exceeds_budget_true() {
        let text = "a".repeat(100); // ~25 tokens
        assert!(exceeds_budget(&text, 10));
    }

    #[test]
    fn test_exceeds_budget_false() {
        let text = "a".repeat(20); // ~5 tokens
        assert!(!exceeds_budget(&text, 10));
    }

    #[test]
    fn test_truncate_to_tokens_no_truncation() {
        let text = "hello";
        let result = truncate_to_tokens(text, 100);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_truncate_to_tokens_truncated() {
        let text = "a".repeat(100);
        let result = truncate_to_tokens(&text, 5);
        // 5 tokens * 4 chars - 3 for "..." = 17 chars + "..."
        assert!(result.ends_with("..."));
        assert!(result.len() < 100);
    }

    #[test]
    fn test_truncate_to_tokens_zero() {
        let result = truncate_to_tokens("hello", 0);
        assert_eq!(result, "");
    }

    #[test]
    fn test_estimate_message_tokens_user() {
        let msg = Message::User(UserMessage {
            uuid: "u1".to_string(),
            message: vec![ContentBlock::Text {
                text: "Hello, world!".to_string(),
            }],
            tool_use_result: None,
        });
        let tokens = estimate_message_tokens(&msg);
        assert!(tokens > 0);
    }

    #[test]
    fn test_estimate_message_tokens_assistant() {
        let msg = Message::Assistant(AssistantMessage {
            uuid: "a1".to_string(),
            message: vec![ContentBlock::Text {
                text: "This is a response.".to_string(),
            }],
            model: "test".to_string(),
            cost_usd: 0.0,
            duration_ms: 0,
            stop_reason: None,
        });
        let tokens = estimate_message_tokens(&msg);
        assert!(tokens > 0);
    }

    #[test]
    fn test_estimate_message_tokens_system() {
        let msg = Message::System(SystemMessage {
            uuid: "s1".to_string(),
            message: "System info".to_string(),
            system_message_type: cc_types::message::SystemMessageType::Info,
        });
        let tokens = estimate_message_tokens(&msg);
        assert!(tokens > 0);
    }

    #[test]
    fn test_truncate_unicode_safety() {
        // Ensure truncation doesn't split a multi-byte character
        let text = "a".repeat(10) + &"\u{1f600}".repeat(20); // emoji
        let result = truncate_to_tokens(&text, 3);
        assert!(result.ends_with("..."));
        // Should be valid UTF-8
        let _ = result.as_bytes();
    }
}
