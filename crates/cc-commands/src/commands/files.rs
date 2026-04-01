use crate::types::*;

pub static FILES: CommandDef = CommandDef {
    name: "files",
    aliases: &[],
    description: "List recently modified files",
    argument_hint: Some("[count]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            let count: usize = args.parse().unwrap_or(20);

            if !cc_utils::git::is_git_repo(&cwd).await {
                // Fall back to filesystem listing
                let result = cc_utils::shell::execute_command(
                    "find",
                    &[
                        ".",
                        "-maxdepth",
                        "3",
                        "-type",
                        "f",
                        "-newer",
                        ".",
                        "-not",
                        "-path",
                        "./.git/*",
                    ],
                    &cwd,
                )
                .await;
                match result {
                    Ok(out) => {
                        let files: Vec<&str> = out.stdout.lines().take(count).collect();
                        if files.is_empty() {
                            return Ok(CommandOutput::message("No recently modified files found."));
                        }
                        let mut lines = vec![format!("Recent files (top {}):", files.len())];
                        for f in &files {
                            lines.push(format!("  {}", f));
                        }
                        Ok(CommandOutput::message(&lines.join("\n")))
                    }
                    Err(_) => Ok(CommandOutput::message("Could not list files.")),
                }
            } else {
                // Use git to find changed files
                match cc_utils::git::changed_files(&cwd).await {
                    Ok(changed) if changed.is_empty() => {
                        // Show recent commits' files instead
                        let log = cc_utils::shell::execute_command(
                            "git",
                            &["log", "--name-only", "--pretty=format:", "-10"],
                            &cwd,
                        )
                        .await;
                        match log {
                            Ok(out) => {
                                let files: Vec<&str> = out
                                    .stdout
                                    .lines()
                                    .filter(|l| !l.trim().is_empty())
                                    .collect::<std::collections::HashSet<_>>()
                                    .into_iter()
                                    .take(count)
                                    .collect();
                                if files.is_empty() {
                                    return Ok(CommandOutput::message(
                                        "No recently modified files.",
                                    ));
                                }
                                let mut lines = vec!["Recently committed files:".to_string()];
                                for f in &files {
                                    lines.push(format!("  {}", f));
                                }
                                Ok(CommandOutput::message(&lines.join("\n")))
                            }
                            Err(_) => Ok(CommandOutput::message("No recently modified files.")),
                        }
                    }
                    Ok(changed) => {
                        let mut lines = vec![format!("Changed files ({}):", changed.len())];
                        for f in changed.iter().take(count) {
                            lines.push(format!("  {}", f));
                        }
                        if changed.len() > count {
                            lines.push(format!("  ... and {} more", changed.len() - count));
                        }
                        Ok(CommandOutput::message(&lines.join("\n")))
                    }
                    Err(e) => Ok(CommandOutput::message(&format!(
                        "Failed to list files: {}",
                        e
                    ))),
                }
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_files_runs() {
        let result = (FILES.handler)("").await.unwrap();
        assert!(result.should_continue);
        assert!(result.message.is_some());
    }
}
