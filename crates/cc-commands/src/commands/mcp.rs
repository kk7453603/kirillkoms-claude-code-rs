use crate::types::*;

pub static MCP: CommandDef = CommandDef {
    name: "mcp",
    aliases: &[],
    description: "Manage MCP servers",
    argument_hint: Some("[list|add|remove]"),
    hidden: true,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            let cwd =
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

            match args.as_str() {
                "" | "list" => {
                    // Try to load MCP configs from project settings
                    let settings_path = cc_config::paths::project_settings_path(&cwd);
                    let settings_content = std::fs::read_to_string(&settings_path)
                        .unwrap_or_else(|_| "{}".to_string());
                    let settings_val: serde_json::Value =
                        serde_json::from_str(&settings_content).unwrap_or_default();
                    let configs = cc_mcp::config::load_mcp_configs(&settings_val);

                    // Also check global settings
                    let global_path = cc_config::paths::global_settings_path();
                    let global_content = std::fs::read_to_string(&global_path)
                        .unwrap_or_else(|_| "{}".to_string());
                    let global_val: serde_json::Value =
                        serde_json::from_str(&global_content).unwrap_or_default();
                    let global_configs = cc_mcp::config::load_mcp_configs(&global_val);

                    let all_configs: Vec<_> =
                        configs.iter().chain(global_configs.iter()).collect();

                    if all_configs.is_empty() {
                        return Ok(CommandOutput::message(
                            "No MCP servers configured.\n\n\
                             Add servers in .claude/settings.json:\n\
                             {\n  \
                               \"mcpServers\": {\n    \
                                 \"server-name\": {\n      \
                                   \"command\": \"npx\",\n      \
                                   \"args\": [\"-y\", \"@example/mcp-server\"]\n    \
                                 }\n  \
                               }\n\
                             }",
                        ));
                    }

                    let mut lines = vec![format!(
                        "MCP Servers ({}):",
                        all_configs.len()
                    )];
                    for cfg in &all_configs {
                        let status = if cfg.enabled {
                            "enabled"
                        } else {
                            "disabled"
                        };
                        lines.push(format!(
                            "\n  {} [{}]",
                            cfg.name, status
                        ));
                        lines.push(format!(
                            "    command: {} {}",
                            cfg.command,
                            cfg.args.join(" ")
                        ));
                        if !cfg.env.is_empty() {
                            lines.push(format!(
                                "    env: {} vars",
                                cfg.env.len()
                            ));
                        }
                    }
                    Ok(CommandOutput::message(&lines.join("\n")))
                }
                _ => Ok(CommandOutput::message(
                    "Usage: /mcp [list]\n\n\
                     MCP server management:\n  \
                     /mcp list    - Show configured servers\n\n\
                     Configure servers in .claude/settings.json under \"mcpServers\".",
                )),
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_list() {
        let result = (MCP.handler)("").await.unwrap();
        assert!(result.should_continue);
        assert!(result.message.is_some());
    }
}
