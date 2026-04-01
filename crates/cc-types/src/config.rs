use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level settings JSON structure (e.g., .claude/settings.json).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<PermissionSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<HashMap<String, Vec<HookSettings>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Permission allow/deny lists in settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow: Option<Vec<PermissionRuleConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny: Option<Vec<PermissionRuleConfig>>,
}

/// A permission rule as specified in configuration files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRuleConfig {
    pub tool: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
}

/// Hook configuration in settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookSettings {
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

/// Configuration for a specific model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub name: String,
    pub context_window: u64,
    pub max_output_tokens: u64,
    pub cost_per_input_token: f64,
    pub cost_per_output_token: f64,
    pub cost_per_cache_read_token: f64,
    pub cost_per_cache_creation_token: f64,
    pub supports_thinking: bool,
    pub supports_images: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_json_empty_roundtrip() {
        let settings = SettingsJson::default();
        let json = serde_json::to_string(&settings).unwrap();
        assert_eq!(json, "{}");
        let back: SettingsJson = serde_json::from_str(&json).unwrap();
        assert!(back.permissions.is_none());
        assert!(back.hooks.is_none());
        assert!(back.env.is_none());
        assert!(back.model.is_none());
    }

    #[test]
    fn settings_json_full_roundtrip() {
        let json = r#"{
            "permissions": {
                "allow": [
                    {"tool": "bash", "input": "git.*"},
                    {"tool": "read_file"}
                ],
                "deny": [
                    {"tool": "bash", "input": "rm -rf.*"}
                ]
            },
            "hooks": {
                "pre_tool_use": [
                    {"command": "echo hello", "timeout": 5000}
                ]
            },
            "env": {
                "MY_VAR": "value"
            },
            "model": "claude-opus-4-20250514"
        }"#;
        let settings: SettingsJson = serde_json::from_str(json).unwrap();
        assert_eq!(settings.model, Some("claude-opus-4-20250514".to_string()));

        let perms = settings.permissions.as_ref().unwrap();
        let allow = perms.allow.as_ref().unwrap();
        assert_eq!(allow.len(), 2);
        assert_eq!(allow[0].tool, "bash");
        assert_eq!(allow[0].input, Some("git.*".to_string()));
        assert!(allow[1].input.is_none());

        let deny = perms.deny.as_ref().unwrap();
        assert_eq!(deny.len(), 1);

        let hooks = settings.hooks.as_ref().unwrap();
        let pre = hooks.get("pre_tool_use").unwrap();
        assert_eq!(pre[0].command, "echo hello");
        assert_eq!(pre[0].timeout, Some(5000));

        let env = settings.env.as_ref().unwrap();
        assert_eq!(env.get("MY_VAR"), Some(&"value".to_string()));

        // Re-serialize and deserialize
        let json2 = serde_json::to_string(&settings).unwrap();
        let back: SettingsJson = serde_json::from_str(&json2).unwrap();
        assert_eq!(back.model, Some("claude-opus-4-20250514".to_string()));
    }

    #[test]
    fn permission_rule_config_minimal() {
        let json = r#"{"tool": "write_file"}"#;
        let rule: PermissionRuleConfig = serde_json::from_str(json).unwrap();
        assert_eq!(rule.tool, "write_file");
        assert!(rule.input.is_none());
    }

    #[test]
    fn permission_rule_config_with_input() {
        let rule = PermissionRuleConfig {
            tool: "bash".to_string(),
            input: Some("npm.*".to_string()),
        };
        let json = serde_json::to_string(&rule).unwrap();
        let back: PermissionRuleConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.tool, "bash");
        assert_eq!(back.input, Some("npm.*".to_string()));
    }

    #[test]
    fn hook_settings_roundtrip() {
        let hook = HookSettings {
            command: "/usr/bin/my-hook --check".to_string(),
            timeout: Some(10000),
        };
        let json = serde_json::to_string(&hook).unwrap();
        let back: HookSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.command, "/usr/bin/my-hook --check");
        assert_eq!(back.timeout, Some(10000));
    }

    #[test]
    fn hook_settings_no_timeout() {
        let json = r#"{"command": "echo test"}"#;
        let hook: HookSettings = serde_json::from_str(json).unwrap();
        assert_eq!(hook.command, "echo test");
        assert!(hook.timeout.is_none());
    }

    #[test]
    fn model_config_roundtrip() {
        let config = ModelConfig {
            id: "claude-opus-4-20250514".to_string(),
            name: "Claude Opus 4".to_string(),
            context_window: 200000,
            max_output_tokens: 16384,
            cost_per_input_token: 0.000015,
            cost_per_output_token: 0.000075,
            cost_per_cache_read_token: 0.0000015,
            cost_per_cache_creation_token: 0.00001875,
            supports_thinking: true,
            supports_images: true,
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: ModelConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "claude-opus-4-20250514");
        assert_eq!(back.context_window, 200000);
        assert!(back.supports_thinking);
        assert!(back.supports_images);
    }

    #[test]
    fn settings_json_partial_fields() {
        let json = r#"{"model": "claude-sonnet-4-20250514"}"#;
        let settings: SettingsJson = serde_json::from_str(json).unwrap();
        assert_eq!(settings.model, Some("claude-sonnet-4-20250514".to_string()));
        assert!(settings.permissions.is_none());
        assert!(settings.hooks.is_none());
        assert!(settings.env.is_none());
    }
}
