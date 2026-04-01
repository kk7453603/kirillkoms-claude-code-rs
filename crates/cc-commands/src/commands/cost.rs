use crate::types::*;

pub static COST: CommandDef = CommandDef {
    name: "cost",
    aliases: &[],
    description: "Show token usage and cost for this session",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            // Create a fresh tracker to demonstrate the format.
            // In production, a SharedCostTracker would be injected via shared state.
            let tracker = cc_cost::tracker::CostTracker::new(None);
            let usage = tracker.total_usage();
            let elapsed = tracker.elapsed();

            let mut lines = vec!["Session Cost Summary".to_string()];
            lines.push(format!(
                "  Total cost:     {}",
                cc_utils::format::format_cost(usage.cost_usd)
            ));
            lines.push(format!(
                "  Input tokens:   {}",
                cc_utils::format::format_tokens(usage.input_tokens)
            ));
            lines.push(format!(
                "  Output tokens:  {}",
                cc_utils::format::format_tokens(usage.output_tokens)
            ));
            if usage.cache_read_input_tokens > 0 {
                lines.push(format!(
                    "  Cache read:     {}",
                    cc_utils::format::format_tokens(usage.cache_read_input_tokens)
                ));
            }
            if usage.cache_creation_input_tokens > 0 {
                lines.push(format!(
                    "  Cache created:  {}",
                    cc_utils::format::format_tokens(usage.cache_creation_input_tokens)
                ));
            }
            lines.push(format!(
                "  Duration:       {}",
                cc_utils::format::format_duration(elapsed.as_millis() as u64)
            ));

            Ok(CommandOutput::message(&lines.join("\n")))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cost_shows_summary() {
        let result = (COST.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Session Cost Summary"));
        assert!(msg.contains("Total cost:"));
        assert!(msg.contains("$0.00"));
        assert!(result.should_continue);
    }
}
