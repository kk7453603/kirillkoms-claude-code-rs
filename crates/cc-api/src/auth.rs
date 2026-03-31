use crate::errors::ApiError;

#[derive(Debug, Clone)]
pub enum AuthMethod {
    ApiKey(String),
    OAuthToken(String),
}

#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub auth: AuthMethod,
    pub base_url: String,
    pub max_retries: u32,
    pub timeout_secs: u64,
}

impl ApiConfig {
    /// Create config from environment variables
    pub fn from_env() -> Result<Self, ApiError> {
        let api_key = std::env::var("ANTHROPIC_API_KEY").ok();
        let auth_token = std::env::var("ANTHROPIC_AUTH_TOKEN")
            .ok()
            .or_else(|| std::env::var("CLAUDE_CODE_OAUTH_TOKEN").ok());
        let base_url = std::env::var("ANTHROPIC_BASE_URL")
            .unwrap_or_else(|_| "https://api.anthropic.com".to_string());

        let auth = if let Some(key) = api_key {
            AuthMethod::ApiKey(key)
        } else if let Some(token) = auth_token {
            AuthMethod::OAuthToken(token)
        } else {
            return Err(ApiError::AuthError {
                message: "No API key or OAuth token found. Set ANTHROPIC_API_KEY or ANTHROPIC_AUTH_TOKEN.".into(),
            });
        };

        Ok(Self {
            auth,
            base_url,
            max_retries: 3,
            timeout_secs: 600,
        })
    }

    pub fn with_api_key(key: String) -> Self {
        Self {
            auth: AuthMethod::ApiKey(key),
            base_url: "https://api.anthropic.com".to_string(),
            max_retries: 3,
            timeout_secs: 600,
        }
    }

    /// Returns the header name and value for authentication.
    pub fn auth_header(&self) -> (&str, String) {
        match &self.auth {
            AuthMethod::ApiKey(key) => ("x-api-key", key.clone()),
            AuthMethod::OAuthToken(token) => ("authorization", format!("Bearer {}", token)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_api_key_creates_config() {
        let config = ApiConfig::with_api_key("sk-test-123".to_string());
        assert_eq!(config.base_url, "https://api.anthropic.com");
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.timeout_secs, 600);
        match &config.auth {
            AuthMethod::ApiKey(k) => assert_eq!(k, "sk-test-123"),
            _ => panic!("expected ApiKey"),
        }
    }

    #[test]
    fn auth_header_api_key() {
        let config = ApiConfig::with_api_key("sk-test".to_string());
        let (name, value) = config.auth_header();
        assert_eq!(name, "x-api-key");
        assert_eq!(value, "sk-test");
    }

    #[test]
    fn auth_header_oauth_token() {
        let config = ApiConfig {
            auth: AuthMethod::OAuthToken("oauth-token-123".to_string()),
            base_url: "https://api.anthropic.com".to_string(),
            max_retries: 3,
            timeout_secs: 600,
        };
        let (name, value) = config.auth_header();
        assert_eq!(name, "authorization");
        assert_eq!(value, "Bearer oauth-token-123");
    }

    // Helper to safely set/remove env vars in tests.
    // SAFETY: These tests must be run with --test-threads=1 to avoid races.
    unsafe fn set_env(key: &str, val: &str) {
        unsafe { std::env::set_var(key, val) };
    }
    unsafe fn remove_env(key: &str) {
        unsafe { std::env::remove_var(key) };
    }
    unsafe fn restore_env(key: &str, saved: Option<String>) {
        match saved {
            Some(v) => unsafe { std::env::set_var(key, v) },
            None => unsafe { std::env::remove_var(key) },
        }
    }

    #[test]
    fn from_env_with_api_key() {
        unsafe {
            let saved_key = std::env::var("ANTHROPIC_API_KEY").ok();
            let saved_token = std::env::var("ANTHROPIC_AUTH_TOKEN").ok();
            let saved_oauth = std::env::var("CLAUDE_CODE_OAUTH_TOKEN").ok();
            let saved_url = std::env::var("ANTHROPIC_BASE_URL").ok();

            set_env("ANTHROPIC_API_KEY", "sk-env-test");
            remove_env("ANTHROPIC_AUTH_TOKEN");
            remove_env("CLAUDE_CODE_OAUTH_TOKEN");
            remove_env("ANTHROPIC_BASE_URL");

            let config = ApiConfig::from_env().unwrap();
            match &config.auth {
                AuthMethod::ApiKey(k) => assert_eq!(k, "sk-env-test"),
                _ => panic!("expected ApiKey"),
            }
            assert_eq!(config.base_url, "https://api.anthropic.com");

            restore_env("ANTHROPIC_API_KEY", saved_key);
            restore_env("ANTHROPIC_AUTH_TOKEN", saved_token);
            restore_env("CLAUDE_CODE_OAUTH_TOKEN", saved_oauth);
            restore_env("ANTHROPIC_BASE_URL", saved_url);
        }
    }

    #[test]
    fn from_env_with_oauth_token() {
        unsafe {
            let saved_key = std::env::var("ANTHROPIC_API_KEY").ok();
            let saved_token = std::env::var("ANTHROPIC_AUTH_TOKEN").ok();
            let saved_oauth = std::env::var("CLAUDE_CODE_OAUTH_TOKEN").ok();

            remove_env("ANTHROPIC_API_KEY");
            set_env("ANTHROPIC_AUTH_TOKEN", "oauth-env-test");
            remove_env("CLAUDE_CODE_OAUTH_TOKEN");

            let config = ApiConfig::from_env().unwrap();
            match &config.auth {
                AuthMethod::OAuthToken(t) => assert_eq!(t, "oauth-env-test"),
                _ => panic!("expected OAuthToken"),
            }

            restore_env("ANTHROPIC_API_KEY", saved_key);
            restore_env("ANTHROPIC_AUTH_TOKEN", saved_token);
            restore_env("CLAUDE_CODE_OAUTH_TOKEN", saved_oauth);
        }
    }

    #[test]
    fn from_env_with_claude_code_oauth_token() {
        unsafe {
            let saved_key = std::env::var("ANTHROPIC_API_KEY").ok();
            let saved_token = std::env::var("ANTHROPIC_AUTH_TOKEN").ok();
            let saved_oauth = std::env::var("CLAUDE_CODE_OAUTH_TOKEN").ok();

            remove_env("ANTHROPIC_API_KEY");
            remove_env("ANTHROPIC_AUTH_TOKEN");
            set_env("CLAUDE_CODE_OAUTH_TOKEN", "cc-oauth-test");

            let config = ApiConfig::from_env().unwrap();
            match &config.auth {
                AuthMethod::OAuthToken(t) => assert_eq!(t, "cc-oauth-test"),
                _ => panic!("expected OAuthToken"),
            }

            restore_env("ANTHROPIC_API_KEY", saved_key);
            restore_env("ANTHROPIC_AUTH_TOKEN", saved_token);
            restore_env("CLAUDE_CODE_OAUTH_TOKEN", saved_oauth);
        }
    }

    #[test]
    fn from_env_no_credentials() {
        unsafe {
            let saved_key = std::env::var("ANTHROPIC_API_KEY").ok();
            let saved_token = std::env::var("ANTHROPIC_AUTH_TOKEN").ok();
            let saved_oauth = std::env::var("CLAUDE_CODE_OAUTH_TOKEN").ok();

            remove_env("ANTHROPIC_API_KEY");
            remove_env("ANTHROPIC_AUTH_TOKEN");
            remove_env("CLAUDE_CODE_OAUTH_TOKEN");

            let result = ApiConfig::from_env();
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, ApiError::AuthError { .. }));

            restore_env("ANTHROPIC_API_KEY", saved_key);
            restore_env("ANTHROPIC_AUTH_TOKEN", saved_token);
            restore_env("CLAUDE_CODE_OAUTH_TOKEN", saved_oauth);
        }
    }

    #[test]
    fn from_env_custom_base_url() {
        unsafe {
            let saved_key = std::env::var("ANTHROPIC_API_KEY").ok();
            let saved_url = std::env::var("ANTHROPIC_BASE_URL").ok();

            set_env("ANTHROPIC_API_KEY", "sk-test");
            set_env("ANTHROPIC_BASE_URL", "https://custom.api.com");

            let config = ApiConfig::from_env().unwrap();
            assert_eq!(config.base_url, "https://custom.api.com");

            restore_env("ANTHROPIC_API_KEY", saved_key);
            restore_env("ANTHROPIC_BASE_URL", saved_url);
        }
    }

    #[test]
    fn api_key_takes_precedence_over_oauth() {
        unsafe {
            let saved_key = std::env::var("ANTHROPIC_API_KEY").ok();
            let saved_token = std::env::var("ANTHROPIC_AUTH_TOKEN").ok();

            set_env("ANTHROPIC_API_KEY", "sk-key");
            set_env("ANTHROPIC_AUTH_TOKEN", "oauth-token");

            let config = ApiConfig::from_env().unwrap();
            match &config.auth {
                AuthMethod::ApiKey(k) => assert_eq!(k, "sk-key"),
                _ => panic!("expected ApiKey to take precedence"),
            }

            restore_env("ANTHROPIC_API_KEY", saved_key);
            restore_env("ANTHROPIC_AUTH_TOKEN", saved_token);
        }
    }
}
