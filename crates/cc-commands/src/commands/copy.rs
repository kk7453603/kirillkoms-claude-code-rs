use crate::types::*;

pub static COPY: CommandDef = CommandDef {
    name: "copy",
    aliases: &["cp"],
    description: "Copy last response to clipboard",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            Ok(CommandOutput::message(
                "Copy is not available in TUI mode. \
                 Select text with your terminal's mouse selection.",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_copy_runs() {
        let result = (COPY.handler)("").await.unwrap();
        assert!(result.should_continue);
        assert!(result.message.is_some());
        let msg = result.message.unwrap();
        assert!(msg.contains("not available in TUI mode"));
    }
}
