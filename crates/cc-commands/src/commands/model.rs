use crate::types::*;

/// Determine the current model from environment variables or the compiled-in default.
///
/// Priority: OPENAI_MODEL > ANTHROPIC_MODEL > CLAUDE_MODEL > default
fn current_model() -> String {
    if let Ok(m) = std::env::var("OPENAI_MODEL")
        && !m.is_empty()
    {
        return m;
    }
    if let Ok(m) = std::env::var("ANTHROPIC_MODEL")
        && !m.is_empty()
    {
        return m;
    }
    if let Ok(m) = std::env::var("CLAUDE_MODEL")
        && !m.is_empty()
    {
        return m;
    }
    cc_config::model_config::default_model().to_string()
}

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
                let current = current_model();
                let config = cc_config::model_config::get_model_config(&current);
                let mut lines = vec![format!("Current model: {}", current)];
                if let Some(cfg) = config {
                    lines.push(format!("  Name:           {}", cfg.name));
                    lines.push(format!(
                        "  Context window: {} tokens",
                        cc_utils::format::format_tokens(cfg.context_window)
                    ));
                    lines.push(format!(
                        "  Max output:     {} tokens",
                        cc_utils::format::format_tokens(cfg.max_output_tokens)
                    ));
                    lines.push(format!(
                        "  Thinking:       {}",
                        if cfg.supports_thinking { "yes" } else { "no" }
                    ));
                } else {
                    lines.push(format!(
                        "  Custom model: {} (no pricing data available)",
                        current
                    ));
                }
                lines.push(String::new());
                lines.push("Available Anthropic models:".to_string());
                for model_id in cc_config::model_config::known_models() {
                    let marker = if model_id == current.as_str() {
                        " (active)"
                    } else {
                        ""
                    };
                    if let Some(mc) = cc_config::model_config::get_model_config(model_id) {
                        lines.push(format!("  {} - {}{}", model_id, mc.name, marker));
                    }
                }
                lines.push(String::new());
                lines.push("Aliases: opus, sonnet, haiku".to_string());
                lines.push(String::new());
                lines.push(
                    "Tip: to use a different model, restart with --model <name> or set OPENAI_MODEL=<name>"
                        .to_string(),
                );
                return Ok(CommandOutput::message(&lines.join("\n")));
            }

            // Resolve known aliases; accept unknown names as-is (pass-through for OpenAI models).
            let resolved = cc_config::model_config::resolve_model_alias(&args)
                .unwrap_or(args.as_str())
                .to_string();

            let mut lines = vec![format!("Model set to: {}", resolved)];
            if let Some(cfg) = cc_config::model_config::get_model_config(&resolved) {
                lines.push(format!("  ({})", cfg.name));
            }
            lines.push(String::new());
            lines.push(format!(
                "To change model, restart with: --model {} or set OPENAI_MODEL={}",
                resolved, resolved
            ));
            Ok(CommandOutput::message(&lines.join("\n")))
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
        assert!(msg.contains("Available Anthropic models:"));
        assert!(msg.contains("Tip:"));
        assert!(result.should_continue);
    }

    #[tokio::test]
    async fn test_model_set_known_alias() {
        let result = (MODEL.handler)("opus").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Model set to:"));
        assert!(msg.contains("claude-opus"));
        assert!(msg.contains("To change model, restart with:"));
    }

    #[tokio::test]
    async fn test_model_set_unknown_passthrough() {
        let result = (MODEL.handler)("qwen3.5:9b").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Model set to: qwen3.5:9b"));
        assert!(msg.contains("To change model, restart with:"));
        assert!(msg.contains("OPENAI_MODEL=qwen3.5:9b"));
    }

    #[tokio::test]
    async fn test_model_set_openai_model() {
        let result = (MODEL.handler)("gpt-4o").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Model set to: gpt-4o"));
        assert!(msg.contains("To change model, restart with:"));
    }

    #[tokio::test]
    async fn test_model_custom_shown_in_status() {
        // When OPENAI_MODEL is set to an unknown model, show custom model message.
        // SAFETY: single-threaded test; no other thread reads this env var concurrently.
        unsafe {
            std::env::set_var("OPENAI_MODEL", "llama3:8b");
        }
        let result = (MODEL.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        unsafe {
            std::env::remove_var("OPENAI_MODEL");
        }
        assert!(msg.contains("Current model: llama3:8b"));
        assert!(msg.contains("Custom model: llama3:8b (no pricing data available)"));
    }
}
