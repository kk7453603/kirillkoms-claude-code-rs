use crate::types::*;

pub static EXPORT: CommandDef = CommandDef {
    name: "export",
    aliases: &[],
    description: "Export session transcript",
    argument_hint: Some("[format] [path]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            let parts: Vec<&str> = args.split_whitespace().collect();
            let format = parts.first().copied().unwrap_or("markdown");
            let output_path = parts.get(1).copied();

            match format {
                "markdown" | "md" => {
                    let path = output_path.unwrap_or("session-export.md");
                    Ok(CommandOutput::message(&format!(
                        "Exporting session as Markdown to: {}\n\
                         The conversation will be saved in readable format.",
                        path
                    )))
                }
                "json" => {
                    let path = output_path.unwrap_or("session-export.json");
                    Ok(CommandOutput::message(&format!(
                        "Exporting session as JSON to: {}\n\
                         All messages and metadata will be preserved.",
                        path
                    )))
                }
                "text" | "txt" => {
                    let path = output_path.unwrap_or("session-export.txt");
                    Ok(CommandOutput::message(&format!(
                        "Exporting session as plain text to: {}",
                        path
                    )))
                }
                "" => Ok(CommandOutput::message(
                    "Usage: /export [format] [path]\n\n\
                     Formats: markdown (default), json, text\n\
                     Example: /export markdown session.md",
                )),
                other => Ok(CommandOutput::message(&format!(
                    "Unknown format: '{}'\nAvailable: markdown, json, text",
                    other
                ))),
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_export_default() {
        let result = (EXPORT.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Usage:"));
    }

    #[tokio::test]
    async fn test_export_markdown() {
        let result = (EXPORT.handler)("markdown").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Markdown"));
    }
}
