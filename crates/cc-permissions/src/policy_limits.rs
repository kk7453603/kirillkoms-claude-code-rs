use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyLimits {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_turns_per_session: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens_per_turn: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_cost_per_session_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub denied_tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_models: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_file_size_bytes: Option<u64>,
    #[serde(default)]
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
    /// Check if a tool is allowed by the policy.
    /// If `denied_tools` is set, the tool must NOT be in the list.
    /// If `allowed_tools` is set, the tool must be in the list.
    /// If neither is set, the tool is allowed.
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        if let Some(denied) = &self.denied_tools {
            if denied.iter().any(|t| t == tool_name) {
                return false;
            }
        }
        if let Some(allowed) = &self.allowed_tools {
            return allowed.iter().any(|t| t == tool_name);
        }
        true
    }

    /// Check if a model is allowed by the policy.
    /// If `allowed_models` is not set, all models are allowed.
    pub fn is_model_allowed(&self, model: &str) -> bool {
        if let Some(allowed) = &self.allowed_models {
            return allowed.iter().any(|m| m == model);
        }
        true
    }

    /// Check if cost is within limits. Returns true if OK, false if limit exceeded.
    pub fn check_cost(&self, current_cost: f64) -> bool {
        if let Some(max_cost) = self.max_cost_per_session_usd {
            return current_cost <= max_cost;
        }
        true
    }

    /// Check if turns are within limits. Returns true if OK, false if limit exceeded.
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
    fn default_policy_allows_everything() {
        let policy = PolicyLimits::default();
        assert!(policy.is_tool_allowed("bash"));
        assert!(policy.is_model_allowed("claude-opus-4-20250514"));
        assert!(policy.check_cost(999.99));
        assert!(policy.check_turns(9999));
        assert!(!policy.read_only_mode);
    }

    #[test]
    fn tool_allowlist_restricts() {
        let policy = PolicyLimits {
            allowed_tools: Some(vec!["read_file".to_string(), "grep".to_string()]),
            ..Default::default()
        };
        assert!(policy.is_tool_allowed("read_file"));
        assert!(policy.is_tool_allowed("grep"));
        assert!(!policy.is_tool_allowed("bash"));
    }

    #[test]
    fn tool_denylist_blocks() {
        let policy = PolicyLimits {
            denied_tools: Some(vec!["bash".to_string()]),
            ..Default::default()
        };
        assert!(!policy.is_tool_allowed("bash"));
        assert!(policy.is_tool_allowed("read_file"));
    }

    #[test]
    fn denied_takes_precedence_over_allowed() {
        let policy = PolicyLimits {
            allowed_tools: Some(vec!["bash".to_string(), "read_file".to_string()]),
            denied_tools: Some(vec!["bash".to_string()]),
            ..Default::default()
        };
        assert!(!policy.is_tool_allowed("bash"));
        assert!(policy.is_tool_allowed("read_file"));
    }

    #[test]
    fn cost_and_turns_limits() {
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
    fn serialization_roundtrip() {
        let policy = PolicyLimits {
            max_turns_per_session: Some(50),
            max_tokens_per_turn: Some(4096),
            max_cost_per_session_usd: Some(10.0),
            allowed_tools: Some(vec!["bash".to_string()]),
            denied_tools: None,
            allowed_models: Some(vec!["claude-opus-4-20250514".to_string()]),
            max_file_size_bytes: Some(1_000_000),
            read_only_mode: true,
        };
        let json = serde_json::to_string(&policy).unwrap();
        let back: PolicyLimits = serde_json::from_str(&json).unwrap();
        assert_eq!(back.max_turns_per_session, Some(50));
        assert!(back.read_only_mode);
        assert!(back.is_model_allowed("claude-opus-4-20250514"));
        assert!(!back.is_model_allowed("other-model"));
    }
}
