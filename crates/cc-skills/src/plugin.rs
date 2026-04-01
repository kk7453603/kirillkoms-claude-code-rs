use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub commands: Vec<PluginCommand>,
    #[serde(skip_serializing_if = "Option::is_none")]
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

/// Load plugins from a directory. Each subdirectory should contain a `plugin.json` manifest.
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
            PluginError::InvalidManifest(format!("{}: {}", manifest_path.display(), e))
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
    use tempfile::TempDir;

    fn create_plugin_dir(base: &Path, name: &str, manifest: &PluginManifest) -> PathBuf {
        let dir = base.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let manifest_json = serde_json::to_string_pretty(manifest).unwrap();
        std::fs::write(dir.join("plugin.json"), manifest_json).unwrap();
        dir
    }

    fn sample_manifest(name: &str) -> PluginManifest {
        PluginManifest {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: Some(format!("A test plugin: {}", name)),
            commands: vec![PluginCommand {
                name: "run".to_string(),
                description: "Run the plugin".to_string(),
            }],
            hooks: None,
        }
    }

    #[test]
    fn load_plugins_from_empty_dir() {
        let tmp = TempDir::new().unwrap();
        let plugins = load_plugins(tmp.path()).unwrap();
        assert!(plugins.is_empty());
    }

    #[test]
    fn load_plugins_nonexistent_dir() {
        let plugins = load_plugins(Path::new("/nonexistent/path/to/plugins")).unwrap();
        assert!(plugins.is_empty());
    }

    #[test]
    fn load_single_plugin() {
        let tmp = TempDir::new().unwrap();
        let manifest = sample_manifest("test-plugin");
        create_plugin_dir(tmp.path(), "test-plugin", &manifest);

        let plugins = load_plugins(tmp.path()).unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].manifest.name, "test-plugin");
        assert_eq!(plugins[0].manifest.version, "1.0.0");
        assert!(plugins[0].enabled);
    }

    #[test]
    fn load_multiple_plugins_sorted() {
        let tmp = TempDir::new().unwrap();
        create_plugin_dir(tmp.path(), "zeta-plugin", &sample_manifest("zeta-plugin"));
        create_plugin_dir(tmp.path(), "alpha-plugin", &sample_manifest("alpha-plugin"));

        let plugins = load_plugins(tmp.path()).unwrap();
        assert_eq!(plugins.len(), 2);
        assert_eq!(plugins[0].manifest.name, "alpha-plugin");
        assert_eq!(plugins[1].manifest.name, "zeta-plugin");
    }

    #[test]
    fn find_plugin_by_name() {
        let tmp = TempDir::new().unwrap();
        create_plugin_dir(tmp.path(), "my-plugin", &sample_manifest("my-plugin"));
        create_plugin_dir(tmp.path(), "other-plugin", &sample_manifest("other-plugin"));

        let plugins = load_plugins(tmp.path()).unwrap();
        let found = find_plugin(&plugins, "my-plugin");
        assert!(found.is_some());
        assert_eq!(found.unwrap().manifest.name, "my-plugin");

        let not_found = find_plugin(&plugins, "nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn invalid_manifest_returns_error() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("bad-plugin");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("plugin.json"), "not valid json").unwrap();

        let result = load_plugins(tmp.path());
        assert!(matches!(result, Err(PluginError::InvalidManifest(_))));
    }
}
