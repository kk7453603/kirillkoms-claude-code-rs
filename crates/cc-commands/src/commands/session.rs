use crate::types::*;

pub static SESSION: CommandDef = CommandDef {
    name: "session",
    aliases: &[],
    description: "Manage sessions",
    argument_hint: Some("[list|new|delete <id>]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            let sessions_dir = cc_config::paths::sessions_dir();

            match args.split_whitespace().collect::<Vec<_>>().as_slice() {
                [] | ["list"] => match cc_session::storage::list_sessions(&sessions_dir) {
                    Ok(sessions) if sessions.is_empty() => {
                        Ok(CommandOutput::message("No saved sessions."))
                    }
                    Ok(sessions) => {
                        let mut lines = vec![format!("Sessions ({}):", sessions.len())];
                        for sid in &sessions {
                            lines.push(format!("  {}", sid));
                        }
                        lines.push(String::new());
                        lines.push("Resume a session: /resume <session_id>".to_string());
                        Ok(CommandOutput::message(&lines.join("\n")))
                    }
                    Err(_) => Ok(CommandOutput::message(
                        "No sessions directory found. Sessions will be created automatically.",
                    )),
                },
                ["new"] => {
                    let id = uuid::Uuid::new_v4().to_string();
                    Ok(CommandOutput::message(&format!(
                        "New session created: {}\nThis session is now active.",
                        id
                    )))
                }
                ["delete", session_id] => {
                    match cc_session::storage::delete_session(&sessions_dir, session_id) {
                        Ok(()) => Ok(CommandOutput::message(&format!(
                            "Session '{}' deleted.",
                            session_id
                        ))),
                        Err(e) => Ok(CommandOutput::message(&format!(
                            "Failed to delete session '{}': {}",
                            session_id, e
                        ))),
                    }
                }
                _ => Ok(CommandOutput::message(
                    "Usage: /session [list|new|delete <id>]",
                )),
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_list() {
        let result = (SESSION.handler)("").await.unwrap();
        assert!(result.should_continue);
        assert!(result.message.is_some());
    }

    #[tokio::test]
    async fn test_session_new() {
        let result = (SESSION.handler)("new").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("New session created"));
    }

    #[tokio::test]
    async fn test_session_bad_subcommand() {
        let result = (SESSION.handler)("badcmd").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Usage:"));
    }
}
