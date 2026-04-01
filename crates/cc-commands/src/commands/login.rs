use crate::types::*;

pub static LOGIN: CommandDef = CommandDef {
    name: "login",
    aliases: &[],
    description: "Login to Anthropic",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            let env_cfg = cc_config::env::EnvConfig::from_env();

            if env_cfg.api_key.is_some() {
                return Ok(CommandOutput::message(
                    "Already authenticated via ANTHROPIC_API_KEY environment variable.\n\
                     To use a different key, update the ANTHROPIC_API_KEY env var.",
                ));
            }

            if env_cfg.auth_token.is_some() {
                return Ok(CommandOutput::message(
                    "Already authenticated via CLAUDE_AUTH_TOKEN.\n\
                     To re-authenticate, update the CLAUDE_AUTH_TOKEN env var.",
                ));
            }

            let provider = env_cfg.provider();
            match provider {
                cc_config::env::ApiProvider::Bedrock => {
                    Ok(CommandOutput::message(
                        "Using AWS Bedrock provider.\n\
                         Configure authentication via AWS credentials (AWS_PROFILE, etc.).",
                    ))
                }
                cc_config::env::ApiProvider::Vertex => {
                    Ok(CommandOutput::message(
                        "Using Google Vertex AI provider.\n\
                         Configure via gcloud auth or GOOGLE_APPLICATION_CREDENTIALS.",
                    ))
                }
                _ => {
                    Ok(CommandOutput::message(
                        "Not authenticated.\n\n\
                         Set your API key:\n  \
                         export ANTHROPIC_API_KEY=sk-ant-...\n\n\
                         Or visit https://console.anthropic.com/settings/keys to get one.",
                    ))
                }
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_login_runs() {
        let result = (LOGIN.handler)("").await.unwrap();
        assert!(result.should_continue);
        assert!(result.message.is_some());
    }
}
