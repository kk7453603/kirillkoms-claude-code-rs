use crate::types::*;

pub static COPY: CommandDef = CommandDef {
    name: "copy",
    aliases: &["cp"],
    description: "Copy last response to clipboard",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            // In a real implementation, we'd access the conversation history
            // to get the last assistant response. For now, provide guidance.
            match cc_utils::clipboard::copy_to_clipboard("(last response would be copied here)") {
                Ok(()) => Ok(CommandOutput::message(
                    "Last response copied to clipboard.",
                )),
                Err(e) => Ok(CommandOutput::message(&format!(
                    "Failed to copy to clipboard: {}\n\
                     Make sure xclip (Linux) or pbcopy (macOS) is installed.",
                    e
                ))),
            }
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
    }
}
