use crate::types::*;

pub static ULTRAPLAN: CommandDef = CommandDef {
    name: "ultraplan",
    aliases: &[],
    description: "Create a comprehensive implementation plan using extended thinking",
    argument_hint: Some("<goal>"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            if args.is_empty() {
                Ok(CommandOutput::message(
                    "Usage: /ultraplan <goal>\n\n\
                     Create a detailed implementation plan using extended thinking.\n\
                     Ultraplan goes deeper than /plan by using maximum thinking budget\n\
                     to produce a thorough, step-by-step implementation strategy.\n\n\
                     Examples:\n  \
                     /ultraplan migrate the database from PostgreSQL to CockroachDB\n  \
                     /ultraplan implement real-time collaboration features",
                ))
            } else {
                Ok(CommandOutput::message(&format!(
                    "Ultra-planning: {}\n\n\
                     Engaging extended thinking for comprehensive analysis...\n\
                     This may take longer than a standard /plan but produces more detailed results.",
                    args
                )))
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ultraplan_empty() {
        let result = (ULTRAPLAN.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Usage:"));
    }

    #[tokio::test]
    async fn test_ultraplan_with_goal() {
        let result = (ULTRAPLAN.handler)("migrate database").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Ultra-planning"));
    }
}
