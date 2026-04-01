use crate::types::*;

pub static DOCTOR: CommandDef = CommandDef {
    name: "doctor",
    aliases: &[],
    description: "Check system health and configuration",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            let mut lines = vec!["System Health Check".to_string(), String::new()];

            // Check API key
            let api_key_status = if std::env::var("ANTHROPIC_API_KEY")
                .ok()
                .filter(|v| !v.is_empty())
                .is_some()
            {
                "configured"
            } else {
                "NOT SET"
            };
            lines.push(format!("  API key:      {}", api_key_status));

            // Check git
            let git_ok = cc_utils::shell::execute_command(
                "git",
                &["--version"],
                std::path::Path::new("/tmp"),
            )
            .await;
            let git_status = match git_ok {
                Ok(out) if out.exit_code == 0 => {
                    format!("available ({})", out.stdout.trim())
                }
                _ => "NOT FOUND".to_string(),
            };
            lines.push(format!("  Git:          {}", git_status));

            // Check ripgrep
            let rg_ok = cc_utils::shell::execute_command(
                "rg",
                &["--version"],
                std::path::Path::new("/tmp"),
            )
            .await;
            let rg_status = match rg_ok {
                Ok(out) if out.exit_code == 0 => {
                    let ver = out.stdout.lines().next().unwrap_or("unknown").to_string();
                    format!("available ({})", ver)
                }
                _ => "NOT FOUND (optional, used for search)".to_string(),
            };
            lines.push(format!("  Ripgrep:      {}", rg_status));

            // Check shell
            let shell_ok = cc_utils::shell::execute_command(
                "sh",
                &["-c", "echo ok"],
                std::path::Path::new("/tmp"),
            )
            .await;
            let shell_status = match shell_ok {
                Ok(out) if out.exit_code == 0 => "available",
                _ => "NOT FOUND",
            };
            lines.push(format!("  Shell:        {}", shell_status));

            // Check working directory is git repo
            let cwd =
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            let in_repo = cc_utils::git::is_git_repo(&cwd).await;
            lines.push(format!(
                "  Git repo:     {}",
                if in_repo { "yes" } else { "no" }
            ));

            // Check config directory
            let config_dir = cc_config::paths::config_dir();
            lines.push(format!(
                "  Config dir:   {} ({})",
                config_dir.display(),
                if config_dir.exists() {
                    "exists"
                } else {
                    "not created yet"
                }
            ));

            // Check CLAUDE.md
            let has_claude_md = cc_config::claude_md::has_claude_md(&cwd);
            lines.push(format!(
                "  CLAUDE.md:    {}",
                if has_claude_md {
                    "found"
                } else {
                    "not found"
                }
            ));

            // Platform info
            lines.push(String::new());
            lines.push(format!(
                "  Platform:     {}-{}",
                std::env::consts::OS,
                std::env::consts::ARCH
            ));
            lines.push(format!(
                "  Version:      {}",
                env!("CARGO_PKG_VERSION")
            ));

            Ok(CommandOutput::message(&lines.join("\n")))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_doctor_runs() {
        let result = (DOCTOR.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("System Health Check"));
        assert!(msg.contains("API key:"));
        assert!(msg.contains("Git:"));
        assert!(msg.contains("Platform:"));
        assert!(result.should_continue);
    }
}
