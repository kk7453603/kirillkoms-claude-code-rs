use crate::types::*;

pub static HOOKS: CommandDef = CommandDef {
    name: "hooks",
    aliases: &[],
    description: "Manage event hooks",
    argument_hint: Some("[list]"),
    hidden: true,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

            match args.as_str() {
                "" | "list" => {
                    let settings = cc_config::settings::load_all_settings(Some(&cwd));
                    match settings {
                        Ok(s) => {
                            if let Some(ref hooks) = s.hooks {
                                if hooks.is_empty() {
                                    return Ok(CommandOutput::message("No hooks configured."));
                                }
                                let mut lines = vec!["Configured hooks:".to_string()];
                                for (event, hook_list) in hooks {
                                    lines.push(format!("\n  {}:", event));
                                    for hook in hook_list {
                                        lines.push(format!("    - {}", hook.command));
                                        if let Some(timeout) = hook.timeout {
                                            lines.push(format!("      timeout: {}ms", timeout));
                                        }
                                    }
                                }
                                Ok(CommandOutput::message(&lines.join("\n")))
                            } else {
                                Ok(CommandOutput::message(
                                    "No hooks configured.\n\
                                     Add hooks in .claude/settings.json under the \"hooks\" key.\n\n\
                                     Available events:\n  \
                                     PreToolUse, PostToolUse, PostToolUseFailure,\n  \
                                     UserPromptSubmit, SessionStart, Setup,\n  \
                                     SubagentStart, FileChanged, CwdChanged,\n  \
                                     WorktreeCreate, PermissionRequest,\n  \
                                     PermissionDenied, Notification",
                                ))
                            }
                        }
                        Err(e) => Ok(CommandOutput::message(&format!(
                            "Failed to load settings: {}",
                            e
                        ))),
                    }
                }
                _ => Ok(CommandOutput::message(
                    "Usage: /hooks [list]\n\
                     Configure hooks in .claude/settings.json.",
                )),
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hooks_list() {
        let result = (HOOKS.handler)("").await.unwrap();
        assert!(result.should_continue);
        assert!(result.message.is_some());
    }
}
