use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyLimits {
    pub max_turns_per_session: Option<u32>,
    pub max_tokens_per_turn: Option<u64>,
    pub max_cost_per_session_usd: Option<f64>,
    pub allowed_tools: Option<Vec<String>>,
    pub denied_tools: Option<Vec<String>>,
    pub allowed_models: Option<Vec<String>>,
    pub max_file_size_bytes: Option<u64>,
    pub read_only_mode: bool,
}

impl Default for PolicyLimits {
    fn default() -> Self {
        Self {
            max_turns_per_session: None,
            max_tokens_per_turn: None,
            max_cost_per_session_usd: None,
            allowed_tools: None,
            denied_tools: None,
            allowed_models: None,
            max_file_size_bytes: None,
            read_only_mode: false,
        }
    }
}

impl PolicyLimits {
    /// Check if a tool is allowed by this policy.
    /// If `allowed_tools` is set, only those tools are allowed.
    /// If `denied_tools` is set, those tools are blocked.
    /// If neither is set, all tools are allowed.
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        if let Some(ref denied) = self.denied_tools {
            if denied.iter().any(|t| t == tool_name) {
                return false;
            }
        }
        if let Some(ref allowed) = self.allowed_tools {
            return allowed.iter().any(|t| t == tool_name);
        }
        true
    }

    /// Check if a model is allowed by this policy.
    /// If `allowed_models` is not set, all models are allowed.
    pub fn is_model_allowed(&self, model: &str) -> bool {
        if let Some(ref allowed) = self.allowed_models {
            return allowed.iter().any(|m| m == model);
        }
        true
    }

    /// Check if the current cost is within the session limit.
    /// Returns `true` if within budget (or no limit set).
    pub fn check_cost(&self, current_cost: f64) -> bool {
        if let Some(max_cost) = self.max_cost_per_session_usd {
            return current_cost <= max_cost;
        }
        true
    }

    /// Check if the current turn count is within the session limit.
    /// Returns `true` if within limit (or no limit set).
    pub fn check_turns(&self, current_turns: u32) -> bool {
        if let Some(max_turns) = self.max_turns_per_session {
            return current_turns <= max_turns;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_allows_everything() {
        let policy = PolicyLimits::default();
        assert!(policy.is_tool_allowed("bash"));
        assert!(policy.is_model_allowed("claude-3-opus"));
        assert!(policy.check_cost(100.0));
        assert!(policy.check_turns(999));
        assert!(!policy.read_only_mode);
    }

    #[test]
    fn test_allowed_tools_whitelist() {
        let policy = PolicyLimits {
            allowed_tools: Some(vec!["read".to_string(), "grep".to_string()]),
            ..Default::default()
        };
        assert!(policy.is_tool_allowed("read"));
        assert!(policy.is_tool_allowed("grep"));
        assert!(!policy.is_tool_allowed("bash"));
    }

    #[test]
    fn test_denied_tools_blacklist() {
        let policy = PolicyLimits {
            denied_tools: Some(vec!["bash".to_string()]),
            ..Default::default()
        };
        assert!(!policy.is_tool_allowed("bash"));
        assert!(policy.is_tool_allowed("read"));
    }

    #[test]
    fn test_denied_takes_precedence_over_allowed() {
        let policy = PolicyLimits {
            allowed_tools: Some(vec!["bash".to_string(), "read".to_string()]),
            denied_tools: Some(vec!["bash".to_string()]),
            ..Default::default()
        };
        assert!(!policy.is_tool_allowed("bash"));
        assert!(policy.is_tool_allowed("read"));
    }

    #[test]
    fn test_cost_and_turn_limits() {
        let policy = PolicyLimits {
            max_cost_per_session_usd: Some(5.0),
            max_turns_per_session: Some(10),
            ..Default::default()
        };
        assert!(policy.check_cost(4.99));
        assert!(policy.check_cost(5.0));
        assert!(!policy.check_cost(5.01));
        assert!(policy.check_turns(10));
        assert!(!policy.check_turns(11));
    }

    #[test]
    fn test_model_allowlist() {
        let policy = PolicyLimits {
            allowed_models: Some(vec!["claude-3-sonnet".to_string()]),
            ..Default::default()
        };
        assert!(policy.is_model_allowed("claude-3-sonnet"));
        assert!(!policy.is_model_allowed("claude-3-opus"));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let policy = PolicyLimits {
            max_turns_per_session: Some(50),
            max_tokens_per_turn: Some(4096),
            max_cost_per_session_usd: Some(10.0),
            allowed_tools: Some(vec!["read".to_string()]),
            denied_tools: None,
            allowed_models: None,
            max_file_size_bytes: Some(1_000_000),
            read_only_mode: true,
        };
        let json = serde_json::to_string(&policy).unwrap();
        let deserialized: PolicyLimits = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.max_turns_per_session, Some(50));
        assert_eq!(deserialized.max_file_size_bytes, Some(1_000_000));
        assert!(deserialized.read_only_mode);
    }
}
