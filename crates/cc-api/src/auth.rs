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

    // Note: from_env() tests that manipulate env vars are inherently racy
    // in parallel test execution. We test the logic through with_api_key()
    // and direct struct construction instead.

    #[test]
    fn from_env_returns_result() {
        // Just verify from_env doesn't panic - result depends on actual env
        let _ = ApiConfig::from_env();
    }

    #[test]
    fn default_base_url() {
        let config = ApiConfig::with_api_key("sk-test".into());
        assert_eq!(config.base_url, "https://api.anthropic.com");
    }

    #[test]
    fn default_retries_and_timeout() {
        let config = ApiConfig::with_api_key("sk-test".into());
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.timeout_secs, 600);
    }

    #[test]
    fn api_key_auth_method() {
        let config = ApiConfig {
            auth: AuthMethod::ApiKey("sk-key".into()),
            base_url: "https://api.anthropic.com".into(),
            max_retries: 3,
            timeout_secs: 600,
        };
        let (name, value) = config.auth_header();
        assert_eq!(name, "x-api-key");
        assert_eq!(value, "sk-key");
    }

    #[test]
    fn oauth_auth_method() {
        let config = ApiConfig {
            auth: AuthMethod::OAuthToken("token-123".into()),
            base_url: "https://api.anthropic.com".into(),
            max_retries: 3,
            timeout_secs: 600,
        };
        let (name, value) = config.auth_header();
        assert_eq!(name, "authorization");
        assert_eq!(value, "Bearer token-123");
    }
}
