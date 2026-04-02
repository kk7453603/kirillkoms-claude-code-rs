use crate::types::*;

pub static REWIND: CommandDef = CommandDef {
    name: "rewind",
    aliases: &["undo"],
    description: "Undo the last conversation turn",
    argument_hint: Some("[n]"),
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            Ok(CommandOutput::message(
                "Rewind is not yet supported in TUI mode. Use /clear to start fresh.",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rewind_default() {
        let result = (REWIND.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("not yet supported"));
    }

    #[tokio::test]
    async fn test_rewind_n() {
        let result = (REWIND.handler)("3").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("not yet supported"));
    }

    #[tokio::test]
    async fn test_rewind_invalid() {
        let result = (REWIND.handler)("abc").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("not yet supported"));
    }
}
