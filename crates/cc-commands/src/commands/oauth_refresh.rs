use crate::types::*;

pub static OAUTH_REFRESH: CommandDef = CommandDef {
    name: "oauth-refresh",
    aliases: &[],
    description: "Refresh OAuth tokens",
    argument_hint: None,
    hidden: true,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Refreshing OAuth tokens...\n\
                 Token refresh is handled automatically. If you're experiencing\n\
                 authentication issues, try /login to re-authenticate.",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_oauth_refresh() {
        let result = (OAUTH_REFRESH.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("OAuth"));
    }
}
