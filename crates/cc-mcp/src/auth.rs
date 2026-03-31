#[derive(Debug, Clone)]
pub enum McpAuth {
    None,
    ApiKey(String),
    Bearer(String),
}

impl McpAuth {
    /// Attempt to load auth from environment variables for a given server.
    ///
    /// Checks for `MCP_<SERVER_NAME>_API_KEY` and `MCP_<SERVER_NAME>_BEARER_TOKEN`.
    /// Returns `McpAuth::None` if no relevant env vars are set.
    pub fn from_env(server_name: &str) -> Self {
        let upper = server_name.to_uppercase().replace('-', "_");
        if let Ok(key) = std::env::var(format!("MCP_{}_API_KEY", upper)) {
            return McpAuth::ApiKey(key);
        }
        if let Ok(token) = std::env::var(format!("MCP_{}_BEARER_TOKEN", upper)) {
            return McpAuth::Bearer(token);
        }
        McpAuth::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_env_returns_none_by_default() {
        // With no env vars set for a random name, should return None
        let auth = McpAuth::from_env("nonexistent_server_xyz_12345");
        assert!(matches!(auth, McpAuth::None));
    }

    #[test]
    fn test_auth_variants() {
        let none = McpAuth::None;
        let api = McpAuth::ApiKey("key123".to_string());
        let bearer = McpAuth::Bearer("token456".to_string());

        assert!(matches!(none, McpAuth::None));
        assert!(matches!(api, McpAuth::ApiKey(ref k) if k == "key123"));
        assert!(matches!(bearer, McpAuth::Bearer(ref t) if t == "token456"));
    }

    #[test]
    fn test_auth_clone() {
        let auth = McpAuth::ApiKey("secret".to_string());
        let cloned = auth.clone();
        assert!(matches!(cloned, McpAuth::ApiKey(ref k) if k == "secret"));
    }
}
