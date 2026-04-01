use crate::types::*;

pub static ENV_CMD: CommandDef = CommandDef {
    name: "env",
    aliases: &[],
    description: "Show environment variables relevant to Claude Code",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async move {
            let vars = [
                "CLAUDE_CODE_MAX_TOKENS",
                "CLAUDE_CODE_USE_BEDROCK",
                "CLAUDE_CODE_USE_VERTEX",
                "ANTHROPIC_API_KEY",
                "ANTHROPIC_MODEL",
                "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
                "CLAUDE_CODE_SKIP_OTEL",
                "DISABLE_PROMPT_CACHING",
                "DISABLE_STREAMING",
                "HTTP_PROXY",
                "HTTPS_PROXY",
                "NO_PROXY",
                "CLAUDE_CODE_SANDBOX",
            ];

            let mut lines = vec!["Claude Code environment variables:\n".to_string()];
            for var in &vars {
                let val = std::env::var(var).unwrap_or_else(|_| "(not set)".to_string());
                lines.push(format!("  {} = {}", var, val));
            }
            lines.push(String::new());
            lines.push("Set these in your shell profile or .env file.".to_string());

            Ok(CommandOutput::message(&lines.join("\n")))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_env_cmd() {
        let result = (ENV_CMD.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("environment variables"));
    }
}
