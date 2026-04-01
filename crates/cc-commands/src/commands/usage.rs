use crate::types::*;

pub static USAGE: CommandDef = CommandDef {
    name: "usage",
    aliases: &[],
    description: "Show token usage details",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            let tracker = cc_cost::tracker::CostTracker::new(None);
            let by_model = tracker.usage_by_model();

            if by_model.is_empty() {
                return Ok(CommandOutput::message(
                    "Token Usage\n\n  No API calls made in this session.\n\n\
                     Usage will be tracked as you interact with Claude.",
                ));
            }

            let mut lines = vec!["Token Usage by Model:".to_string(), String::new()];
            for (model, usage) in by_model {
                lines.push(format!("  {}:", model));
                lines.push(format!(
                    "    Input:   {}",
                    cc_utils::format::format_tokens(usage.input_tokens)
                ));
                lines.push(format!(
                    "    Output:  {}",
                    cc_utils::format::format_tokens(usage.output_tokens)
                ));
                lines.push(format!(
                    "    Cost:    {}",
                    cc_utils::format::format_cost(usage.cost_usd)
                ));
            }

            let total = tracker.total_usage();
            lines.push(String::new());
            lines.push(format!(
                "Total: {} tokens, {}",
                cc_utils::format::format_tokens(total.total_tokens()),
                cc_utils::format::format_cost(total.cost_usd)
            ));

            Ok(CommandOutput::message(&lines.join("\n")))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_usage() {
        let result = (USAGE.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Token Usage"));
    }
}
