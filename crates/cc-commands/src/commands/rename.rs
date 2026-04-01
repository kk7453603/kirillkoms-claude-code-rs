use crate::types::*;

pub static RENAME: CommandDef = CommandDef {
    name: "rename",
    aliases: &[],
    description: "Rename current session",
    argument_hint: Some("<name>"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            if args.is_empty() {
                Ok(CommandOutput::message(
                    "Usage: /rename <name>\nGive the current session a descriptive name.",
                ))
            } else {
                Ok(CommandOutput::message(&format!(
                    "Session renamed to: '{}'",
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
    async fn test_rename_no_args() {
        let result = (RENAME.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Usage:"));
    }

    #[tokio::test]
    async fn test_rename_with_name() {
        let result = (RENAME.handler)("my feature work").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("my feature work"));
    }
}
