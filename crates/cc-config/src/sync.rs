use cc_types::config::SettingsJson;

/// Check if local settings match remote by comparing serialized forms.
pub fn settings_need_sync(local: &SettingsJson, remote: &SettingsJson) -> bool {
    let local_json = serde_json::to_string(local).unwrap_or_default();
    let remote_json = serde_json::to_string(remote).unwrap_or_default();
    local_json != remote_json
}

/// Merge remote settings into local (remote takes precedence for conflicting keys).
pub fn sync_settings(local: &SettingsJson, remote: &SettingsJson) -> SettingsJson {
    SettingsJson {
        permissions: remote
            .permissions
            .clone()
            .or_else(|| local.permissions.clone()),
        hooks: remote.hooks.clone().or_else(|| local.hooks.clone()),
        env: match (&local.env, &remote.env) {
            (Some(local_env), Some(remote_env)) => {
                let mut merged = local_env.clone();
                for (k, v) in remote_env {
                    merged.insert(k.clone(), v.clone());
                }
                Some(merged)
            }
            (None, Some(remote_env)) => Some(remote_env.clone()),
            (Some(local_env), None) => Some(local_env.clone()),
            (None, None) => None,
        },
        model: remote.model.clone().or_else(|| local.model.clone()),
    }
}

/// Serialize settings diff for display. Returns a list of human-readable change descriptions.
pub fn settings_diff(old: &SettingsJson, new: &SettingsJson) -> Vec<String> {
    let mut diffs = Vec::new();

    // Check model changes
    if old.model != new.model {
        diffs.push(format!(
            "model: {:?} -> {:?}",
            old.model.as_deref().unwrap_or("<none>"),
            new.model.as_deref().unwrap_or("<none>"),
        ));
    }

    // Check env changes
    let old_env = old.env.clone().unwrap_or_default();
    let new_env = new.env.clone().unwrap_or_default();

    for (k, v) in &new_env {
        match old_env.get(k) {
            Some(old_v) if old_v != v => {
                diffs.push(format!("env.{}: {:?} -> {:?}", k, old_v, v));
            }
            None => {
                diffs.push(format!("env.{}: <none> -> {:?}", k, v));
            }
            _ => {}
        }
    }
    for k in old_env.keys() {
        if !new_env.contains_key(k) {
            diffs.push(format!("env.{}: {:?} -> <removed>", k, old_env[k]));
        }
    }

    // Check permissions changes
    let old_perms_json = old
        .permissions
        .as_ref()
        .and_then(|p| serde_json::to_string(p).ok())
        .unwrap_or_default();
    let new_perms_json = new
        .permissions
        .as_ref()
        .and_then(|p| serde_json::to_string(p).ok())
        .unwrap_or_default();
    if old_perms_json != new_perms_json {
        diffs.push("permissions: <changed>".to_string());
    }

    // Check hooks changes
    let old_hooks_json = old
        .hooks
        .as_ref()
        .and_then(|h| serde_json::to_string(h).ok())
        .unwrap_or_default();
    let new_hooks_json = new
        .hooks
        .as_ref()
        .and_then(|h| serde_json::to_string(h).ok())
        .unwrap_or_default();
    if old_hooks_json != new_hooks_json {
        diffs.push("hooks: <changed>".to_string());
    }

    diffs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn identical_settings_no_sync_needed() {
        let settings = SettingsJson {
            model: Some("claude-opus-4-20250514".to_string()),
            ..Default::default()
        };
        assert!(!settings_need_sync(&settings, &settings));
    }

    #[test]
    fn different_model_needs_sync() {
        let local = SettingsJson {
            model: Some("claude-sonnet-4-20250514".to_string()),
            ..Default::default()
        };
        let remote = SettingsJson {
            model: Some("claude-opus-4-20250514".to_string()),
            ..Default::default()
        };
        assert!(settings_need_sync(&local, &remote));
    }

    #[test]
    fn sync_remote_model_wins() {
        let local = SettingsJson {
            model: Some("local-model".to_string()),
            ..Default::default()
        };
        let remote = SettingsJson {
            model: Some("remote-model".to_string()),
            ..Default::default()
        };
        let merged = sync_settings(&local, &remote);
        assert_eq!(merged.model, Some("remote-model".to_string()));
    }

    #[test]
    fn sync_merges_env_vars() {
        let mut local_env = HashMap::new();
        local_env.insert("A".to_string(), "1".to_string());
        local_env.insert("B".to_string(), "2".to_string());

        let mut remote_env = HashMap::new();
        remote_env.insert("B".to_string(), "3".to_string());
        remote_env.insert("C".to_string(), "4".to_string());

        let local = SettingsJson {
            env: Some(local_env),
            ..Default::default()
        };
        let remote = SettingsJson {
            env: Some(remote_env),
            ..Default::default()
        };

        let merged = sync_settings(&local, &remote);
        let env = merged.env.unwrap();
        assert_eq!(env.get("A"), Some(&"1".to_string())); // kept from local
        assert_eq!(env.get("B"), Some(&"3".to_string())); // remote wins
        assert_eq!(env.get("C"), Some(&"4".to_string())); // added from remote
    }

    #[test]
    fn diff_detects_model_change() {
        let old = SettingsJson {
            model: Some("old-model".to_string()),
            ..Default::default()
        };
        let new = SettingsJson {
            model: Some("new-model".to_string()),
            ..Default::default()
        };
        let diffs = settings_diff(&old, &new);
        assert!(!diffs.is_empty());
        assert!(diffs[0].contains("model"));
        assert!(diffs[0].contains("old-model"));
        assert!(diffs[0].contains("new-model"));
    }

    #[test]
    fn diff_empty_when_identical() {
        let settings = SettingsJson::default();
        let diffs = settings_diff(&settings, &settings);
        assert!(diffs.is_empty());
    }
}
