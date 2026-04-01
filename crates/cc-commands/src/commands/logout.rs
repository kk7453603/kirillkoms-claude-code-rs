use crate::types::*;

pub static LOGOUT: CommandDef = CommandDef {
    name: "logout",
    aliases: &[],
    description: "Logout from Anthropic",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            Ok(CommandOutput::message(
                "To logout, remove your API credentials:\n\n  \
                 unset ANTHROPIC_API_KEY\n  \
                 unset CLAUDE_AUTH_TOKEN\n\n\
                 Or remove them from your shell profile (~/.bashrc, ~/.zshrc, etc.).",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_logout() {
        let result = (LOGOUT.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("unset ANTHROPIC_API_KEY"));
    }
}
