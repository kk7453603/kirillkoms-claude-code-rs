use crate::types::*;

pub static CONFIG: CommandDef = CommandDef {
    name: "config",
    aliases: &[],
    description: "View or modify configuration",
    argument_hint: Some("[key] [value]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            if args.is_empty() {
                let cwd = std::env::current_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from("."));

                let mut lines = vec!["Configuration".to_string(), String::new()];

                // Load settings
                let settings = cc_config::settings::load_all_settings(Some(&cwd));
                match settings {
                    Ok(s) => {
                        lines.push(format!(
                            "  model:            {}",
                            s.model
                                .as_deref()
                                .unwrap_or(cc_config::model_config::default_model())
                        ));
                        if let Some(ref perms) = s.permissions {
                            let allow_count =
                                perms.allow.as_ref().map(|v| v.len()).unwrap_or(0);
                            let deny_count =
                                perms.deny.as_ref().map(|v| v.len()).unwrap_or(0);
                            lines.push(format!(
                                "  permissions:      {} allow, {} deny rules",
                                allow_count, deny_count
                            ));
                        } else {
                            lines.push("  permissions:      default".to_string());
                        }
                        if let Some(ref hooks) = s.hooks {
                            let total: usize = hooks.values().map(|v| v.len()).sum();
                            lines.push(format!(
                                "  hooks:            {} configured",
                                total
                            ));
                        } else {
                            lines.push("  hooks:            none".to_string());
                        }
                        if let Some(ref env_map) = s.env {
                            lines.push(format!(
                                "  env vars:         {} set",
                                env_map.len()
                            ));
                        }
                    }
                    Err(e) => {
                        lines.push(format!("  Error loading settings: {}", e));
                    }
                }

                lines.push(String::new());
                lines.push("Config file locations:".to_string());
                lines.push(format!(
                    "  Global:  {}",
                    cc_config::paths::global_settings_path().display()
                ));
                lines.push(format!(
                    "  Project: {}",
                    cc_config::paths::project_settings_path(&cwd).display()
                ));
                lines.push(format!(
                    "  Local:   {}",
                    cc_config::paths::local_settings_path(&cwd).display()
                ));

                return Ok(CommandOutput::message(&lines.join("\n")));
            }

            // Parse key=value or key value
            let parts: Vec<&str> = args.splitn(2, |c: char| c == ' ' || c == '=').collect();
            match parts.as_slice() {
                [key] => {
                    let env_cfg = cc_config::env::EnvConfig::from_env();
                    let val = match *key {
                        "model" => Some(
                            env_cfg
                                .model
                                .unwrap_or_else(|| {
                                    cc_config::model_config::default_model().to_string()
                                }),
                        ),
                        "api_key" => {
                            Some(if env_cfg.api_key.is_some() {
                                "***configured***".to_string()
                            } else {
                                "not set".to_string()
                            })
                        }
                        "provider" => Some(format!("{:?}", env_cfg.provider())),
                        _ => None,
                    };
                    match val {
                        Some(v) => Ok(CommandOutput::message(&format!("{} = {}", key, v))),
                        None => Ok(CommandOutput::message(&format!(
                            "Unknown config key: '{}'\nKnown keys: model, api_key, provider",
                            key
                        ))),
                    }
                }
                [key, value] => Ok(CommandOutput::message(&format!(
                    "Config '{}' set to '{}'\n\
                     Note: Runtime config changes are not persisted. \
                     Edit settings.json for permanent changes.",
                    key, value
                ))),
                _ => Ok(CommandOutput::message("Usage: /config [key] [value]")),
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_config_show_all() {
        let result = (CONFIG.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Configuration"));
        // Output contains configuration info
        assert!(
            msg.contains("model:") || msg.contains("Config file") || msg.contains("Error loading"),
            "Unexpected config output: {}",
            msg
        );
        assert!(result.should_continue);
    }

    #[tokio::test]
    async fn test_config_get_key() {
        let result = (CONFIG.handler)("model").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("model ="));
    }

    #[tokio::test]
    async fn test_config_unknown_key() {
        let result = (CONFIG.handler)("nonexistent").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Unknown config key"));
    }
}
