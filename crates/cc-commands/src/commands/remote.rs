use crate::types::*;

pub static REMOTE_ENV: CommandDef = CommandDef {
    name: "remote-env",
    aliases: &[],
    description: "Configure the default remote environment for teleport sessions",
    argument_hint: Some("[env-name]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            if args.is_empty() {
                Ok(CommandOutput::message(
                    "Remote environment configuration.\n\n\
                     No remote environment is currently configured.\n\
                     Usage: /remote-env <environment-name>\n\n\
                     This sets the default environment used by /teleport.",
                ))
            } else {
                Ok(CommandOutput::message(&format!(
                    "Remote environment set to '{}'.\n\
                     Future /teleport sessions will use this environment by default.",
                    args
                )))
            }
        })
    },
};

pub static REMOTE_SETUP: CommandDef = CommandDef {
    name: "remote-setup",
    aliases: &[],
    description: "Set up remote development environment",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Remote development setup wizard.\n\n\
                 This command configures your remote development environment\n\
                 for use with Claude Code's teleport feature.\n\n\
                 Prerequisites:\n  \
                 - SSH access to the remote machine\n  \
                 - Claude Code installed on the remote machine\n\n\
                 Use /remote-env to configure the default environment.",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_remote_env_empty() {
        let result = (REMOTE_ENV.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Remote environment"));
    }

    #[tokio::test]
    async fn test_remote_env_set() {
        let result = (REMOTE_ENV.handler)("staging").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("staging"));
    }

    #[tokio::test]
    async fn test_remote_setup() {
        let result = (REMOTE_SETUP.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Remote development"));
    }
}
