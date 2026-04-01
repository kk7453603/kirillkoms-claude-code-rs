use crate::types::*;

pub static STATS: CommandDef = CommandDef {
    name: "stats",
    aliases: &[],
    description: "Show session statistics",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            // Use a fresh tracker for demonstration; in production this would be shared
            let tracker = cc_cost::tracker::CostTracker::new(None);
            let usage = tracker.total_usage();
            let elapsed = tracker.elapsed();

            let mut lines = vec!["Session Statistics".to_string(), String::new()];
            lines.push(format!(
                "  Duration:        {}",
                cc_utils::format::format_duration(elapsed.as_millis() as u64)
            ));
            lines.push(format!(
                "  Total cost:      {}",
                cc_utils::format::format_cost(usage.cost_usd)
            ));
            lines.push(format!(
                "  Input tokens:    {}",
                cc_utils::format::format_tokens(usage.input_tokens)
            ));
            lines.push(format!(
                "  Output tokens:   {}",
                cc_utils::format::format_tokens(usage.output_tokens)
            ));
            lines.push(format!(
                "  Total tokens:    {}",
                cc_utils::format::format_tokens(usage.total_tokens())
            ));
            if usage.cache_read_input_tokens > 0 || usage.cache_creation_input_tokens > 0 {
                lines.push(format!(
                    "  Cache read:      {}",
                    cc_utils::format::format_tokens(usage.cache_read_input_tokens)
                ));
                lines.push(format!(
                    "  Cache created:   {}",
                    cc_utils::format::format_tokens(usage.cache_creation_input_tokens)
                ));
            }
            if usage.web_search_requests > 0 {
                lines.push(format!("  Web searches:    {}", usage.web_search_requests));
            }
            lines.push(format!(
                "  API time:        {}",
                cc_utils::format::format_duration(tracker.total_api_duration().as_millis() as u64)
            ));
            lines.push(format!(
                "  Tool time:       {}",
                cc_utils::format::format_duration(tracker.total_tool_duration().as_millis() as u64)
            ));

            if let Some(remaining) = tracker.remaining_budget() {
                lines.push(format!(
                    "  Budget left:     {}",
                    cc_utils::format::format_cost(remaining)
                ));
            }

            Ok(CommandOutput::message(&lines.join("\n")))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stats() {
        let result = (STATS.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Session Statistics"));
        assert!(msg.contains("Duration:"));
        assert!(msg.contains("Total cost:"));
    }
}
