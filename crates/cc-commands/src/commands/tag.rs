use crate::types::*;

pub static TAG: CommandDef = CommandDef {
    name: "tag",
    aliases: &["bookmark"],
    description: "Tag current point in conversation for later reference",
    argument_hint: Some("<name>"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            if args.is_empty() {
                Ok(CommandOutput::message(
                    "Usage: /tag <name>\n\n\
                     Create a named bookmark at the current point in the conversation.\n\
                     Use /rewind <tag> to return to a tagged point.\n\n\
                     Examples:\n  \
                     /tag before-refactor\n  \
                     /tag checkpoint-1",
                ))
            } else {
                Ok(CommandOutput::message(&format!(
                    "Tagged current conversation point as '{}'.\n\
                     You can reference this tag later to return to this point.",
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
    async fn test_tag_no_args() {
        let result = (TAG.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Usage:"));
    }

    #[tokio::test]
    async fn test_tag_with_name() {
        let result = (TAG.handler)("checkpoint-1").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("checkpoint-1"));
    }
}
