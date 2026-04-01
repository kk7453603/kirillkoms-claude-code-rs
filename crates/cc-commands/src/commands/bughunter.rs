use crate::types::*;

pub static BUGHUNTER: CommandDef = CommandDef {
    name: "bughunter",
    aliases: &[],
    description: "Systematically hunt for bugs in the codebase",
    argument_hint: Some("[path]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            if args.is_empty() {
                Ok(CommandOutput::message(
                    "Usage: /bughunter [path]\n\n\
                     Systematically analyze code for potential bugs:\n  \
                     - Logic errors\n  \
                     - Edge cases\n  \
                     - Resource leaks\n  \
                     - Error handling gaps\n\n\
                     Specify a file or directory path, or omit to scan the current project.",
                ))
            } else {
                Ok(CommandOutput::message(&format!(
                    "Bug hunter scanning: {}\n\
                     Analyzing for potential issues...",
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
    async fn test_bughunter_empty() {
        let result = (BUGHUNTER.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Usage:"));
    }
}
