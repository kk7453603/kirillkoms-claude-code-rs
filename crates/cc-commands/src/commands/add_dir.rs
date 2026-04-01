use crate::types::*;

pub static ADD_DIR: CommandDef = CommandDef {
    name: "add-dir",
    aliases: &["adddir"],
    description: "Add additional working directory",
    argument_hint: Some("<path>"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            if args.is_empty() {
                return Ok(CommandOutput::message(
                    "Usage: /add-dir <path>\n\
                     Adds an additional working directory for file operations.\n\n\
                     Example: /add-dir ../other-project",
                ));
            }

            let path = cc_utils::path::expand_tilde(&args);
            let resolved = if path.is_absolute() {
                path
            } else {
                let cwd = std::env::current_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from("."));
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

            Ok(CommandOutput::message(&format!(
                "Added working directory: {}",
                resolved.display()
            )))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_dir_no_args() {
        let result = (ADD_DIR.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Usage:"));
    }

    #[tokio::test]
    async fn test_add_dir_nonexistent() {
        let result = (ADD_DIR.handler)("/nonexistent/path/xyz123").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("not found"));
    }

    #[tokio::test]
    async fn test_add_dir_valid() {
        let result = (ADD_DIR.handler)("/tmp").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Added working directory"));
    }
}
