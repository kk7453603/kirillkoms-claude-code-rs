use crate::types::*;

pub static RESUME: CommandDef = CommandDef {
    name: "resume",
    aliases: &[],
    description: "Resume a previous session",
    argument_hint: Some("[session_id]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            let sessions_dir = cc_config::paths::sessions_dir();

            if args.is_empty() {
                // Show recent sessions to pick from
                let history_path = cc_config::paths::history_path();
                let recent =
                    cc_session::history::read_recent_history(&history_path, 10).unwrap_or_default();

                if recent.is_empty() {
                    // Fall back to listing session dirs
                    match cc_session::storage::list_sessions(&sessions_dir) {
                        Ok(sessions) if sessions.is_empty() => {
                            return Ok(CommandOutput::message(
                                "No previous sessions found.\nUsage: /resume <session_id>",
                            ));
                        }
                        Ok(sessions) => {
                            let mut lines = vec!["Recent sessions:".to_string()];
                            for sid in sessions.iter().rev().take(10) {
                                lines.push(format!("  {}", sid));
                            }
                            lines.push(String::new());
                            lines.push("Resume with: /resume <session_id>".to_string());
                            return Ok(CommandOutput::message(&lines.join("\n")));
                        }
                        Err(_) => {
                            return Ok(CommandOutput::message(
                                "No previous sessions found.\nUsage: /resume <session_id>",
                            ));
                        }
                    }
                }

                let mut lines = vec!["Recent sessions:".to_string()];
                for entry in recent.iter().rev() {
                    let project = entry.project_root.as_deref().unwrap_or("unknown");
                    lines.push(format!(
                        "  {} - \"{}\" ({})",
                        entry.session_id,
                        cc_utils::format::truncate_str(&entry.prompt, 40),
                        project
                    ));
                }
                lines.push(String::new());
                lines.push("Resume with: /resume <session_id>".to_string());
                return Ok(CommandOutput::message(&lines.join("\n")));
            }

            // Try to load the session
            match cc_session::resume::load_resume_data(&sessions_dir, &args) {
                Ok(data) => {
                    let project = data.project_root.as_deref().unwrap_or("unknown");
                    Ok(CommandOutput::message(&format!(
                        "Resuming session: {}\n  Messages: {}\n  Project: {}",
                        data.session_id,
                        data.messages.len(),
                        project
                    )))
                }
                Err(e) => Ok(CommandOutput::message(&format!(
                    "Failed to resume session '{}': {}",
                    args, e
                ))),
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resume_no_args() {
        let result = (RESUME.handler)("").await.unwrap();
        assert!(result.should_continue);
        assert!(result.message.is_some());
    }

    #[tokio::test]
    async fn test_resume_nonexistent_session() {
        let result = (RESUME.handler)("nonexistent-id-xyz").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Failed to resume"));
    }
}
