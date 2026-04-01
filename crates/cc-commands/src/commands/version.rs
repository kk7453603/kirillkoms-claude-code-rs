use crate::types::*;

pub static VERSION: CommandDef = CommandDef {
    name: "version",
    aliases: &["v"],
    description: "Show version information",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            let version = env!("CARGO_PKG_VERSION");
            let msg = format!(
                "claude-code-rs v{}\nPlatform: {}-{}\nProfile: {}",
                version,
                std::env::consts::OS,
                std::env::consts::ARCH,
                if cfg!(debug_assertions) { "debug" } else { "release" },
            );
            Ok(CommandOutput::message(&msg))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_version() {
        let result = (VERSION.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("claude-code-rs v"));
        assert!(result.should_continue);
    }
}
