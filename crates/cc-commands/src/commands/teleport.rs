use crate::types::*;

pub static TELEPORT: CommandDef = CommandDef {
    name: "teleport",
    aliases: &["cd"],
    description: "Change working directory",
    argument_hint: Some("<path>"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            if args.is_empty() {
                let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                return Ok(CommandOutput::message(&format!(
                    "Current directory: {}\n\nUsage: /teleport <path>",
                    cwd.display()
                )));
            }

            let path = cc_utils::path::expand_tilde(&args);
            let resolved = if path.is_absolute() {
                path
            } else {
                let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                cwd.join(&path)
            };

            if !resolved.exists() {
                return Ok(CommandOutput::message(&format!(
                    "Directory not found: {}",
                    resolved.display()
                )));
            }
            if !resolved.is_dir() {
                return Ok(CommandOutput::message(&format!(
                    "Not a directory: {}",
                    resolved.display()
                )));
            }

            // Canonicalize the path
            let canonical = resolved.canonicalize().unwrap_or(resolved);

            if let Err(e) = std::env::set_current_dir(&canonical) {
                return Ok(CommandOutput::message(&format!(
                    "Failed to change directory to {}: {}",
                    canonical.display(),
                    e
                )));
            }

            Ok(CommandOutput::message(&format!(
                "Working directory changed to: {}",
                canonical.display()
            )))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_teleport_no_args() {
        let result = (TELEPORT.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Current directory:"));
    }

    #[tokio::test]
    async fn test_teleport_valid() {
        let result = (TELEPORT.handler)("/tmp").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Working directory changed"));
    }

    #[tokio::test]
    async fn test_teleport_invalid() {
        let result = (TELEPORT.handler)("/nonexistent/xyz123").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("not found"));
    }
}
