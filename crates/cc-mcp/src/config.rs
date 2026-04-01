use crate::types::McpServerConfig;
use std::path::Path;

/// Load MCP server configs from settings JSON value.
///
/// Expects a JSON object with a "mcpServers" key containing a map of server configs.
pub fn load_mcp_configs(settings: &serde_json::Value) -> Vec<McpServerConfig> {
    let servers = match settings.get("mcpServers").and_then(|v| v.as_object()) {
        Some(obj) => obj,
        None => return vec![],
    };

    let mut configs = Vec::new();
    for (name, value) in servers {
        let command = value
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let args: Vec<String> = value
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let env: std::collections::HashMap<String, String> = value
            .get("env")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        let enabled = value
            .get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        configs.push(McpServerConfig {
            name: name.clone(),
            command,
            args,
            env,
            enabled,
        });
    }
    configs
}

/// Load MCP configs from a directory of JSON files.
///
/// Each JSON file in the directory is expected to be a settings file
/// with an "mcpServers" key.
pub fn load_mcp_configs_from_dir(dir: &Path) -> Vec<McpServerConfig> {
    let mut configs = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return configs,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json")
            && let Ok(content) = std::fs::read_to_string(&path)
            && let Ok(value) = serde_json::from_str::<serde_json::Value>(&content)
        {
            configs.extend(load_mcp_configs(&value));
        }
    }
    configs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_mcp_configs_basic() {
        let settings = serde_json::json!({
            "mcpServers": {
                "github": {
                    "command": "npx",
                    "args": ["-y", "@github/mcp-server"],
                    "env": {
                        "GITHUB_TOKEN": "abc"
                    }
                }
            }
        });
        let configs = load_mcp_configs(&settings);
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].name, "github");
        assert_eq!(configs[0].command, "npx");
        assert_eq!(configs[0].args, vec!["-y", "@github/mcp-server"]);
        assert_eq!(configs[0].env.get("GITHUB_TOKEN").unwrap(), "abc");
        assert!(configs[0].enabled); // defaults to true
    }

    #[test]
    fn test_load_mcp_configs_empty() {
        let settings = serde_json::json!({});
        let configs = load_mcp_configs(&settings);
        assert!(configs.is_empty());
    }

    #[test]
    fn test_load_mcp_configs_disabled() {
        let settings = serde_json::json!({
            "mcpServers": {
                "srv": {
                    "command": "cmd",
                    "args": [],
                    "enabled": false
                }
            }
        });
        let configs = load_mcp_configs(&settings);
        assert_eq!(configs.len(), 1);
        assert!(!configs[0].enabled);
    }

    #[test]
    fn test_load_mcp_configs_multiple() {
        let settings = serde_json::json!({
            "mcpServers": {
                "a": {"command": "a_cmd", "args": []},
                "b": {"command": "b_cmd", "args": ["x"]}
            }
        });
        let configs = load_mcp_configs(&settings);
        assert_eq!(configs.len(), 2);
    }

    #[test]
    fn test_load_mcp_configs_from_dir_nonexistent() {
        let configs = load_mcp_configs_from_dir(Path::new("/nonexistent/path"));
        assert!(configs.is_empty());
    }
}
