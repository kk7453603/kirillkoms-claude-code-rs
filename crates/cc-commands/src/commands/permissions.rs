use crate::types::*;

pub static PERMISSIONS: CommandDef = CommandDef {
    name: "permissions",
    aliases: &["perms"],
    description: "View or manage tool permissions",
    argument_hint: Some("[mode]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            if args.is_empty() {
                let cwd = std::env::current_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from("."));
                let settings = cc_config::settings::load_all_settings(Some(&cwd));

                let mut lines = vec!["Permission Settings".to_string(), String::new()];

                if let Ok(ref s) = settings {
                    if let Some(ref perms) = s.permissions {
                        if let Some(ref allow) = perms.allow {
                            lines.push("  Allow rules:".to_string());
                            for rule in allow {
                                let input_str = rule
                                    .input
                                    .as_deref()
                                    .map(|i| format!(" (input: {})", i))
                                    .unwrap_or_default();
                                lines.push(format!("    - {}{}", rule.tool, input_str));
                            }
                        }
                        if let Some(ref deny) = perms.deny {
                            lines.push("  Deny rules:".to_string());
                            for rule in deny {
                                let input_str = rule
                                    .input
                                    .as_deref()
                                    .map(|i| format!(" (input: {})", i))
                                    .unwrap_or_default();
                                lines.push(format!("    - {}{}", rule.tool, input_str));
                            }
                        }
                    } else {
                        lines.push("  No permission rules configured (using defaults).".to_string());
                    }
                } else {
                    lines.push("  Using default permissions.".to_string());
                }

                lines.push(String::new());
                lines.push("Available modes:".to_string());
                lines.push("  default          - Ask before write operations".to_string());
                lines.push("  auto             - Auto-approve safe operations".to_string());
                lines.push("  plan             - Read-only mode".to_string());
                lines.push("  acceptEdits      - Auto-approve file edits".to_string());
                lines.push("  bypassPermissions - Approve all operations".to_string());
                lines.push(String::new());
                lines.push("Set mode: /permissions <mode>".to_string());

                return Ok(CommandOutput::message(&lines.join("\n")));
            }

            match cc_permissions::modes::PermissionMode::from_str_opt(&args) {
                Some(mode) => Ok(CommandOutput::message(&format!(
                    "Permission mode set to: {}\n  Read-only: {}\n  Auto-approve edits: {}\n  Auto-approve all: {}",
                    mode.as_str(),
                    mode.is_read_only_mode(),
                    mode.allows_edits(),
                    mode.allows_all(),
                ))),
                None => Ok(CommandOutput::message(&format!(
                    "Unknown permission mode: '{}'\n\
                     Available: default, auto, plan, acceptEdits, bypassPermissions, dontAsk",
                    args
                ))),
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_permissions_show() {
        let result = (PERMISSIONS.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Permission Settings"));
        assert!(msg.contains("Available modes:"));
        assert!(result.should_continue);
    }

    #[tokio::test]
    async fn test_permissions_set_mode() {
        let result = (PERMISSIONS.handler)("plan").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Permission mode set to: plan"));
        assert!(msg.contains("Read-only: true"));
    }

    #[tokio::test]
    async fn test_permissions_unknown_mode() {
        let result = (PERMISSIONS.handler)("badmode").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Unknown permission mode"));
    }
}
