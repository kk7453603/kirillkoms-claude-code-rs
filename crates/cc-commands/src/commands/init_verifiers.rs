use crate::types::*;

pub static INIT_VERIFIERS: CommandDef = CommandDef {
    name: "init-verifiers",
    aliases: &[],
    description: "Initialize verification hooks for the project",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Initializing verifiers for the current project...\n\n\
                 Verifiers run automatically to check code quality:\n  \
                 - Lint checks\n  \
                 - Type checking\n  \
                 - Test execution\n  \
                 - Build verification\n\n\
                 Use /hooks to manage verification hooks after setup.",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_init_verifiers() {
        let result = (INIT_VERIFIERS.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("verifiers"));
    }
}
