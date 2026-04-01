use crate::types::*;

pub static MODEL: CommandDef = CommandDef {
    name: "model",
    aliases: &[],
    description: "View or change the current model",
    argument_hint: Some("[model_name]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            if args.is_empty() {
                let current = cc_config::model_config::default_model();
                let config = cc_config::model_config::get_model_config(current);
                let mut lines = vec![format!("Current model: {}", current)];
                if let Some(cfg) = config {
                    lines.push(format!("  Name:           {}", cfg.name));
                    lines.push(format!(
                        "  Context window: {} tokens",
                        cc_utils::format::format_tokens(cfg.context_window as u64)
                    ));
                    lines.push(format!(
                        "  Max output:     {} tokens",
                        cc_utils::format::format_tokens(cfg.max_output_tokens as u64)
                    ));
                    lines.push(format!(
                        "  Thinking:       {}",
                        if cfg.supports_thinking { "yes" } else { "no" }
                    ));
                }
                lines.push(String::new());
                lines.push("Available models:".to_string());
                for model_id in cc_config::model_config::known_models() {
                    let marker = if model_id == current { " (active)" } else { "" };
                    if let Some(mc) = cc_config::model_config::get_model_config(model_id) {
                        lines.push(format!("  {} - {}{}", model_id, mc.name, marker));
                    }
                }
                lines.push(String::new());
                lines.push("Aliases: opus, sonnet, haiku".to_string());
                return Ok(CommandOutput::message(&lines.join("\n")));
            }

            match cc_config::model_config::resolve_model_alias(&args) {
                Some(model_id) => {
                    let config = cc_config::model_config::get_model_config(model_id);
                    let name = config.map(|c| c.name).unwrap_or_default();
                    Ok(CommandOutput::message(&format!(
                        "Model set to: {} ({})",
                        model_id, name
                    )))
                }
                None => Ok(CommandOutput::message(&format!(
                    "Unknown model: '{}'\nAvailable: {}",
                    args,
                    cc_config::model_config::known_models().join(", ")
                ))),
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_model_show_current() {
        let result = (MODEL.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Current model:"));
        assert!(msg.contains("Available models:"));
        assert!(result.should_continue);
    }

    #[tokio::test]
    async fn test_model_set_alias() {
        let result = (MODEL.handler)("opus").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Model set to:"));
        assert!(msg.contains("opus"));
    }

    #[tokio::test]
    async fn test_model_unknown() {
        let result = (MODEL.handler)("gpt-5").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Unknown model"));
    }
}
