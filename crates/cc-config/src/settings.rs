use std::path::Path;

use cc_types::config::SettingsJson;

/// Errors that can occur when loading or processing settings.
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Invalid settings: {message}")]
    Invalid { message: String },
}

/// Load settings from a JSON file, returning default if file doesn't exist.
pub fn load_settings_file(path: &Path) -> Result<SettingsJson, SettingsError> {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let settings: SettingsJson = serde_json::from_str(&content)?;
            Ok(settings)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(SettingsJson::default()),
        Err(e) => Err(SettingsError::Io(e)),
    }
}

/// Merge multiple settings layers. Later settings override earlier ones.
///
/// Permission rules are concatenated (not overridden) so that all layers
/// contribute their allow/deny lists. Scalar values like `model` are
/// overridden by later layers.
pub fn merge_settings(settings: &[SettingsJson]) -> SettingsJson {
    let mut merged = SettingsJson::default();

    for layer in settings {
        // Merge permission allow rules (concatenate)
        if let Some(ref perms) = layer.permissions {
            let merged_perms = merged.permissions.get_or_insert_with(Default::default);
            if let Some(ref allow) = perms.allow {
                let merged_allow = merged_perms.allow.get_or_insert_with(Vec::new);
                merged_allow.extend(allow.iter().cloned());
            }
            if let Some(ref deny) = perms.deny {
                let merged_deny = merged_perms.deny.get_or_insert_with(Vec::new);
                merged_deny.extend(deny.iter().cloned());
            }
        }

        // Merge hooks (concatenate per event)
        if let Some(ref hooks) = layer.hooks {
            let merged_hooks = merged.hooks.get_or_insert_with(Default::default);
            for (event, hook_list) in hooks {
                let merged_list = merged_hooks.entry(event.clone()).or_default();
                merged_list.extend(hook_list.iter().cloned());
            }
        }

        // Merge env vars (later overrides earlier)
        if let Some(ref env_map) = layer.env {
            let merged_env = merged.env.get_or_insert_with(Default::default);
            for (k, v) in env_map {
                merged_env.insert(k.clone(), v.clone());
            }
        }

        // Override scalar model value
        if layer.model.is_some() {
            merged.model = layer.model.clone();
        }
    }

    merged
}

/// Load all settings layers: global -> project -> local.
///
/// Settings are merged in order so that local settings override project
/// settings which override global settings. Permission rules from all
/// layers are concatenated.
pub fn load_all_settings(project_root: Option<&Path>) -> Result<SettingsJson, SettingsError> {
    let mut layers = Vec::new();

    // Global settings
    let global_path = crate::paths::global_settings_path();
    layers.push(load_settings_file(&global_path)?);

    // Project and local settings
    if let Some(root) = project_root {
        let project_path = crate::paths::project_settings_path(root);
        layers.push(load_settings_file(&project_path)?);

        let local_path = crate::paths::local_settings_path(root);
        layers.push(load_settings_file(&local_path)?);
    }

    Ok(merge_settings(&layers))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_types::config::{HookSettings, PermissionRuleConfig, PermissionSettings};
    use std::collections::HashMap;

    #[test]
    fn load_nonexistent_file_returns_default() {
        let result = load_settings_file(Path::new("/nonexistent/path/settings.json"));
        assert!(result.is_ok());
        let settings = result.unwrap();
        assert!(settings.permissions.is_none());
        assert!(settings.model.is_none());
    }

    #[test]
    fn load_valid_settings_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{"model": "claude-opus-4-6", "permissions": {"allow": [{"tool": "bash"}]}}"#,
        )
        .unwrap();

        let settings = load_settings_file(&path).unwrap();
        assert_eq!(settings.model, Some("claude-opus-4-6".to_string()));
        let perms = settings.permissions.unwrap();
        let allow = perms.allow.unwrap();
        assert_eq!(allow.len(), 1);
        assert_eq!(allow[0].tool, "bash");
    }

    #[test]
    fn load_invalid_json_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, "not valid json {{{").unwrap();

        let result = load_settings_file(&path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SettingsError::Json(_)));
    }

    #[test]
    fn load_empty_object_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, "{}").unwrap();

        let settings = load_settings_file(&path).unwrap();
        assert!(settings.model.is_none());
        assert!(settings.permissions.is_none());
    }

    #[test]
    fn merge_empty_settings() {
        let merged = merge_settings(&[]);
        assert!(merged.permissions.is_none());
        assert!(merged.model.is_none());
    }

    #[test]
    fn merge_single_settings() {
        let s = SettingsJson {
            model: Some("claude-opus-4-6".to_string()),
            ..Default::default()
        };
        let merged = merge_settings(&[s]);
        assert_eq!(merged.model, Some("claude-opus-4-6".to_string()));
    }

    #[test]
    fn merge_model_override() {
        let s1 = SettingsJson {
            model: Some("claude-sonnet-4-6".to_string()),
            ..Default::default()
        };
        let s2 = SettingsJson {
            model: Some("claude-opus-4-6".to_string()),
            ..Default::default()
        };
        let merged = merge_settings(&[s1, s2]);
        assert_eq!(merged.model, Some("claude-opus-4-6".to_string()));
    }

    #[test]
    fn merge_model_not_overridden_by_none() {
        let s1 = SettingsJson {
            model: Some("claude-opus-4-6".to_string()),
            ..Default::default()
        };
        let s2 = SettingsJson::default();
        let merged = merge_settings(&[s1, s2]);
        assert_eq!(merged.model, Some("claude-opus-4-6".to_string()));
    }

    #[test]
    fn merge_permissions_concatenated() {
        let s1 = SettingsJson {
            permissions: Some(PermissionSettings {
                allow: Some(vec![PermissionRuleConfig {
                    tool: "bash".to_string(),
                    input: None,
                }]),
                deny: None,
            }),
            ..Default::default()
        };
        let s2 = SettingsJson {
            permissions: Some(PermissionSettings {
                allow: Some(vec![PermissionRuleConfig {
                    tool: "read_file".to_string(),
                    input: None,
                }]),
                deny: Some(vec![PermissionRuleConfig {
                    tool: "write_file".to_string(),
                    input: Some("/etc/.*".to_string()),
                }]),
            }),
            ..Default::default()
        };

        let merged = merge_settings(&[s1, s2]);
        let perms = merged.permissions.unwrap();
        let allow = perms.allow.unwrap();
        assert_eq!(allow.len(), 2);
        assert_eq!(allow[0].tool, "bash");
        assert_eq!(allow[1].tool, "read_file");
        let deny = perms.deny.unwrap();
        assert_eq!(deny.len(), 1);
        assert_eq!(deny[0].tool, "write_file");
    }

    #[test]
    fn merge_hooks_concatenated() {
        let s1 = SettingsJson {
            hooks: Some({
                let mut m = HashMap::new();
                m.insert(
                    "pre_tool_use".to_string(),
                    vec![HookSettings {
                        command: "echo hook1".to_string(),
                        timeout: None,
                    }],
                );
                m
            }),
            ..Default::default()
        };
        let s2 = SettingsJson {
            hooks: Some({
                let mut m = HashMap::new();
                m.insert(
                    "pre_tool_use".to_string(),
                    vec![HookSettings {
                        command: "echo hook2".to_string(),
                        timeout: Some(5000),
                    }],
                );
                m
            }),
            ..Default::default()
        };

        let merged = merge_settings(&[s1, s2]);
        let hooks = merged.hooks.unwrap();
        let pre = hooks.get("pre_tool_use").unwrap();
        assert_eq!(pre.len(), 2);
        assert_eq!(pre[0].command, "echo hook1");
        assert_eq!(pre[1].command, "echo hook2");
    }

    #[test]
    fn merge_env_vars_overridden() {
        let s1 = SettingsJson {
            env: Some({
                let mut m = HashMap::new();
                m.insert("KEY1".to_string(), "val1".to_string());
                m.insert("KEY2".to_string(), "old".to_string());
                m
            }),
            ..Default::default()
        };
        let s2 = SettingsJson {
            env: Some({
                let mut m = HashMap::new();
                m.insert("KEY2".to_string(), "new".to_string());
                m.insert("KEY3".to_string(), "val3".to_string());
                m
            }),
            ..Default::default()
        };

        let merged = merge_settings(&[s1, s2]);
        let env = merged.env.unwrap();
        assert_eq!(env.get("KEY1"), Some(&"val1".to_string()));
        assert_eq!(env.get("KEY2"), Some(&"new".to_string()));
        assert_eq!(env.get("KEY3"), Some(&"val3".to_string()));
    }

    #[test]
    fn load_all_settings_no_project() {
        // Point global config to a temp dir so we don't read the real one.
        let dir = tempfile::tempdir().unwrap();
        unsafe { std::env::set_var("CLAUDE_CONFIG_DIR", dir.path().as_os_str()); }
        let result = load_all_settings(None);
        unsafe { std::env::remove_var("CLAUDE_CONFIG_DIR"); }
        assert!(result.is_ok());
    }

    #[test]
    fn load_all_settings_with_project() {
        let global_dir = tempfile::tempdir().unwrap();
        unsafe { std::env::set_var("CLAUDE_CONFIG_DIR", global_dir.path().as_os_str()); }

        let project_dir = tempfile::tempdir().unwrap();
        let claude_dir = project_dir.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(
            claude_dir.join("settings.json"),
            r#"{"model": "claude-sonnet-4-6"}"#,
        )
        .unwrap();
        std::fs::write(
            claude_dir.join("settings.local.json"),
            r#"{"model": "claude-opus-4-6"}"#,
        )
        .unwrap();

        let settings = load_all_settings(Some(project_dir.path())).unwrap();
        unsafe { std::env::remove_var("CLAUDE_CONFIG_DIR"); }
        // Local settings override project settings
        assert_eq!(settings.model, Some("claude-opus-4-6".to_string()));
    }

    #[test]
    fn settings_error_display() {
        let e = SettingsError::Invalid {
            message: "bad value".to_string(),
        };
        assert_eq!(e.to_string(), "Invalid settings: bad value");
    }

    #[test]
    fn settings_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "no access");
        let e: SettingsError = io_err.into();
        assert!(matches!(e, SettingsError::Io(_)));
    }
}
