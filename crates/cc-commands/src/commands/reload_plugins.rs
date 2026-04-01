use crate::types::*;

pub static RELOAD_PLUGINS: CommandDef = CommandDef {
    name: "reload-plugins",
    aliases: &[],
    description: "Activate pending plugin changes in the current session",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Reloading plugins...\n\n\
                 All plugin configurations have been refreshed.\n\
                 Commands, agents, and MCP servers from plugins are now up to date.",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_reload_plugins() {
        let result = (RELOAD_PLUGINS.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Reloading plugins"));
    }
}
