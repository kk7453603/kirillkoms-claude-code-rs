use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub commands: Vec<PluginCommand>,
    pub hooks: Option<Vec<PluginHook>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCommand {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHook {
    pub event: String,
    pub command: String,
}

#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub path: PathBuf,
    pub enabled: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),
    #[error("Plugin not found: {0}")]
    NotFound(String),
}

/// Load plugins from a directory. Each subdirectory is expected to contain a `plugin.json` manifest.
pub fn load_plugins(plugins_dir: &Path) -> Result<Vec<LoadedPlugin>, PluginError> {
    let mut plugins = Vec::new();

    if !plugins_dir.exists() {
        return Ok(plugins);
    }

    let entries = std::fs::read_dir(plugins_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let manifest_path = path.join("plugin.json");
        if !manifest_path.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&manifest_path)?;
        let manifest: PluginManifest = serde_json::from_str(&content).map_err(|e| {
            PluginError::InvalidManifest(format!(
                "{}: {}",
                manifest_path.display(),
                e
            ))
        })?;

        plugins.push(LoadedPlugin {
            manifest,
            path,
            enabled: true,
        });
    }

    plugins.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
    Ok(plugins)
}

/// Get plugin by name
pub fn find_plugin<'a>(plugins: &'a [LoadedPlugin], name: &str) -> Option<&'a LoadedPlugin> {
    plugins.iter().find(|p| p.manifest.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_plugin(dir: &Path, name: &str, version: &str) {
        let plugin_dir = dir.join(name);
        fs::create_dir_all(&plugin_dir).unwrap();
        let manifest = PluginManifest {
            name: name.to_string(),
            version: version.to_string(),
            description: Some(format!("Test plugin {}", name)),
            commands: vec![PluginCommand {
                name: "run".to_string(),
                description: "Run the plugin".to_string(),
            }],
            hooks: Some(vec![PluginHook {
                event: "on_start".to_string(),
                command: "init".to_string(),
            }]),
        };
        let json = serde_json::to_string_pretty(&manifest).unwrap();
        fs::write(plugin_dir.join("plugin.json"), json).unwrap();
    }

    #[test]
    fn test_load_plugins_from_directory() {
        let tmp = tempfile::tempdir().unwrap();
        create_test_plugin(tmp.path(), "alpha", "1.0.0");
        create_test_plugin(tmp.path(), "beta", "2.0.0");

        let plugins = load_plugins(tmp.path()).unwrap();
        assert_eq!(plugins.len(), 2);
        assert_eq!(plugins[0].manifest.name, "alpha");
        assert_eq!(plugins[1].manifest.name, "beta");
        assert!(plugins[0].enabled);
    }

    #[test]
    fn test_load_plugins_empty_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let plugins = load_plugins(tmp.path()).unwrap();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_load_plugins_nonexistent_directory() {
        let plugins = load_plugins(Path::new("/nonexistent/path/to/plugins")).unwrap();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_find_plugin_by_name() {
        let tmp = tempfile::tempdir().unwrap();
        create_test_plugin(tmp.path(), "my-plugin", "1.0.0");

        let plugins = load_plugins(tmp.path()).unwrap();
        let found = find_plugin(&plugins, "my-plugin");
        assert!(found.is_some());
        assert_eq!(found.unwrap().manifest.version, "1.0.0");

        let not_found = find_plugin(&plugins, "nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_invalid_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        let plugin_dir = tmp.path().join("bad-plugin");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(plugin_dir.join("plugin.json"), "{ invalid json }").unwrap();

        let result = load_plugins(tmp.path());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PluginError::InvalidManifest(_)));
    }
}
