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

    #[test]
    fn from_env_with_api_key() {
        // Save and restore env vars
        let saved_key = std::env::var("ANTHROPIC_API_KEY").ok();
        let saved_token = std::env::var("ANTHROPIC_AUTH_TOKEN").ok();
        let saved_oauth = std::env::var("CLAUDE_CODE_OAUTH_TOKEN").ok();
        let saved_url = std::env::var("ANTHROPIC_BASE_URL").ok();

        std::env::set_var("ANTHROPIC_API_KEY", "sk-env-test");
        std::env::remove_var("ANTHROPIC_AUTH_TOKEN");
        std::env::remove_var("CLAUDE_CODE_OAUTH_TOKEN");
        std::env::remove_var("ANTHROPIC_BASE_URL");

        let config = ApiConfig::from_env().unwrap();
        match &config.auth {
            AuthMethod::ApiKey(k) => assert_eq!(k, "sk-env-test"),
            _ => panic!("expected ApiKey"),
        }
        assert_eq!(config.base_url, "https://api.anthropic.com");

        // Restore
        match saved_key {
            Some(v) => std::env::set_var("ANTHROPIC_API_KEY", v),
            None => std::env::remove_var("ANTHROPIC_API_KEY"),
        }
        match saved_token {
            Some(v) => std::env::set_var("ANTHROPIC_AUTH_TOKEN", v),
            None => std::env::remove_var("ANTHROPIC_AUTH_TOKEN"),
        }
        match saved_oauth {
            Some(v) => std::env::set_var("CLAUDE_CODE_OAUTH_TOKEN", v),
            None => std::env::remove_var("CLAUDE_CODE_OAUTH_TOKEN"),
        }
        match saved_url {
            Some(v) => std::env::set_var("ANTHROPIC_BASE_URL", v),
            None => std::env::remove_var("ANTHROPIC_BASE_URL"),
        }
    }

    #[test]
    fn from_env_with_oauth_token() {
        let saved_key = std::env::var("ANTHROPIC_API_KEY").ok();
        let saved_token = std::env::var("ANTHROPIC_AUTH_TOKEN").ok();
        let saved_oauth = std::env::var("CLAUDE_CODE_OAUTH_TOKEN").ok();

        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::set_var("ANTHROPIC_AUTH_TOKEN", "oauth-env-test");
        std::env::remove_var("CLAUDE_CODE_OAUTH_TOKEN");

        let config = ApiConfig::from_env().unwrap();
        match &config.auth {
            AuthMethod::OAuthToken(t) => assert_eq!(t, "oauth-env-test"),
            _ => panic!("expected OAuthToken"),
        }

        // Restore
        match saved_key {
            Some(v) => std::env::set_var("ANTHROPIC_API_KEY", v),
            None => std::env::remove_var("ANTHROPIC_API_KEY"),
        }
        match saved_token {
            Some(v) => std::env::set_var("ANTHROPIC_AUTH_TOKEN", v),
            None => std::env::remove_var("ANTHROPIC_AUTH_TOKEN"),
        }
        match saved_oauth {
            Some(v) => std::env::set_var("CLAUDE_CODE_OAUTH_TOKEN", v),
            None => std::env::remove_var("CLAUDE_CODE_OAUTH_TOKEN"),
        }
    }

    #[test]
    fn from_env_with_claude_code_oauth_token() {
        let saved_key = std::env::var("ANTHROPIC_API_KEY").ok();
        let saved_token = std::env::var("ANTHROPIC_AUTH_TOKEN").ok();
        let saved_oauth = std::env::var("CLAUDE_CODE_OAUTH_TOKEN").ok();

        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("ANTHROPIC_AUTH_TOKEN");
        std::env::set_var("CLAUDE_CODE_OAUTH_TOKEN", "cc-oauth-test");

        let config = ApiConfig::from_env().unwrap();
        match &config.auth {
            AuthMethod::OAuthToken(t) => assert_eq!(t, "cc-oauth-test"),
            _ => panic!("expected OAuthToken"),
        }

        // Restore
        match saved_key {
            Some(v) => std::env::set_var("ANTHROPIC_API_KEY", v),
            None => std::env::remove_var("ANTHROPIC_API_KEY"),
        }
        match saved_token {
            Some(v) => std::env::set_var("ANTHROPIC_AUTH_TOKEN", v),
            None => std::env::remove_var("ANTHROPIC_AUTH_TOKEN"),
        }
        match saved_oauth {
            Some(v) => std::env::set_var("CLAUDE_CODE_OAUTH_TOKEN", v),
            None => std::env::remove_var("CLAUDE_CODE_OAUTH_TOKEN"),
        }
    }

    #[test]
    fn from_env_no_credentials() {
        let saved_key = std::env::var("ANTHROPIC_API_KEY").ok();
        let saved_token = std::env::var("ANTHROPIC_AUTH_TOKEN").ok();
        let saved_oauth = std::env::var("CLAUDE_CODE_OAUTH_TOKEN").ok();

        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("ANTHROPIC_AUTH_TOKEN");
        std::env::remove_var("CLAUDE_CODE_OAUTH_TOKEN");

        let result = ApiConfig::from_env();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ApiError::AuthError { .. }));

        // Restore
        match saved_key {
            Some(v) => std::env::set_var("ANTHROPIC_API_KEY", v),
            None => std::env::remove_var("ANTHROPIC_API_KEY"),
        }
        match saved_token {
            Some(v) => std::env::set_var("ANTHROPIC_AUTH_TOKEN", v),
            None => std::env::remove_var("ANTHROPIC_AUTH_TOKEN"),
        }
        match saved_oauth {
            Some(v) => std::env::set_var("CLAUDE_CODE_OAUTH_TOKEN", v),
            None => std::env::remove_var("CLAUDE_CODE_OAUTH_TOKEN"),
        }
    }

    #[test]
    fn from_env_custom_base_url() {
        let saved_key = std::env::var("ANTHROPIC_API_KEY").ok();
        let saved_url = std::env::var("ANTHROPIC_BASE_URL").ok();

        std::env::set_var("ANTHROPIC_API_KEY", "sk-test");
        std::env::set_var("ANTHROPIC_BASE_URL", "https://custom.api.com");

        let config = ApiConfig::from_env().unwrap();
        assert_eq!(config.base_url, "https://custom.api.com");

        // Restore
        match saved_key {
            Some(v) => std::env::set_var("ANTHROPIC_API_KEY", v),
            None => std::env::remove_var("ANTHROPIC_API_KEY"),
        }
        match saved_url {
            Some(v) => std::env::set_var("ANTHROPIC_BASE_URL", v),
            None => std::env::remove_var("ANTHROPIC_BASE_URL"),
        }
    }

    #[test]
    fn api_key_takes_precedence_over_oauth() {
        let saved_key = std::env::var("ANTHROPIC_API_KEY").ok();
        let saved_token = std::env::var("ANTHROPIC_AUTH_TOKEN").ok();

        std::env::set_var("ANTHROPIC_API_KEY", "sk-key");
        std::env::set_var("ANTHROPIC_AUTH_TOKEN", "oauth-token");

        let config = ApiConfig::from_env().unwrap();
        match &config.auth {
            AuthMethod::ApiKey(k) => assert_eq!(k, "sk-key"),
            _ => panic!("expected ApiKey to take precedence"),
        }

        // Restore
        match saved_key {
            Some(v) => std::env::set_var("ANTHROPIC_API_KEY", v),
            None => std::env::remove_var("ANTHROPIC_API_KEY"),
        }
        match saved_token {
            Some(v) => std::env::set_var("ANTHROPIC_AUTH_TOKEN", v),
            None => std::env::remove_var("ANTHROPIC_AUTH_TOKEN"),
        }
    }
}
