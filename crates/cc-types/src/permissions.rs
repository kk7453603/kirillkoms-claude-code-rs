use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    Default,
    Auto,
    Plan,
    AcceptEdits,
    BypassPermissions,
    DontAsk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionBehavior {
    Allow,
    Deny,
    Ask,
}

impl Serialize for PermissionBehavior {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            PermissionBehavior::Allow => serializer.serialize_str("allow"),
            PermissionBehavior::Deny => serializer.serialize_str("deny"),
            PermissionBehavior::Ask => serializer.serialize_str("ask"),
        }
    }
}

impl<'de> Deserialize<'de> for PermissionBehavior {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "allow" => Ok(PermissionBehavior::Allow),
            "deny" => Ok(PermissionBehavior::Deny),
            "ask" => Ok(PermissionBehavior::Ask),
            _ => Err(serde::de::Error::unknown_variant(
                &s,
                &["allow", "deny", "ask"],
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    pub source: PermissionRuleSource,
    pub tool_name: Option<String>,
    pub input_pattern: Option<String>,
    pub behavior: PermissionBehavior,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionRuleSource {
    UserSettings,
    ProjectSettings,
    LocalSettings,
    FlagSettings,
    PolicySettings,
    CliArg,
    Command,
    Session,
}

#[derive(Debug, Clone)]
pub enum PermissionDecision {
    Allow {
        message: Option<String>,
    },
    Ask {
        message: String,
        allow_rules: Vec<PermissionRule>,
    },
    Deny {
        message: String,
    },
}

#[derive(Debug, Clone)]
pub struct ToolPermissionContext {
    pub mode: PermissionMode,
    pub always_allow_rules: Vec<PermissionRule>,
    pub always_deny_rules: Vec<PermissionRule>,
    pub always_ask_rules: Vec<PermissionRule>,
    pub is_bypass_mode_available: bool,
}

impl Default for ToolPermissionContext {
    fn default() -> Self {
        Self {
            mode: PermissionMode::Default,
            always_allow_rules: Vec::new(),
            always_deny_rules: Vec::new(),
            always_ask_rules: Vec::new(),
            is_bypass_mode_available: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_mode_serde_roundtrip() {
        let modes = [
            (PermissionMode::Default, "\"default\""),
            (PermissionMode::Auto, "\"auto\""),
            (PermissionMode::Plan, "\"plan\""),
            (PermissionMode::AcceptEdits, "\"acceptEdits\""),
            (PermissionMode::BypassPermissions, "\"bypassPermissions\""),
            (PermissionMode::DontAsk, "\"dontAsk\""),
        ];
        for (mode, expected_json) in modes {
            let json = serde_json::to_string(&mode).unwrap();
            assert_eq!(json, expected_json);
            let back: PermissionMode = serde_json::from_str(&json).unwrap();
            assert_eq!(mode, back);
        }
    }

    #[test]
    fn permission_behavior_serde() {
        let json = serde_json::to_string(&PermissionBehavior::Allow).unwrap();
        assert_eq!(json, "\"allow\"");
        let back: PermissionBehavior = serde_json::from_str(&json).unwrap();
        assert_eq!(back, PermissionBehavior::Allow);

        let json = serde_json::to_string(&PermissionBehavior::Deny).unwrap();
        assert_eq!(json, "\"deny\"");

        let json = serde_json::to_string(&PermissionBehavior::Ask).unwrap();
        assert_eq!(json, "\"ask\"");
    }

    #[test]
    fn permission_rule_roundtrip() {
        let rule = PermissionRule {
            source: PermissionRuleSource::UserSettings,
            tool_name: Some("bash".to_string()),
            input_pattern: Some("git.*".to_string()),
            behavior: PermissionBehavior::Allow,
        };
        let json = serde_json::to_string(&rule).unwrap();
        let back: PermissionRule = serde_json::from_str(&json).unwrap();
        assert_eq!(back.tool_name, Some("bash".to_string()));
        assert_eq!(back.behavior, PermissionBehavior::Allow);
        assert_eq!(back.source, PermissionRuleSource::UserSettings);
    }

    #[test]
    fn permission_rule_source_variants() {
        let sources = [
            (PermissionRuleSource::UserSettings, "\"userSettings\""),
            (PermissionRuleSource::ProjectSettings, "\"projectSettings\""),
            (PermissionRuleSource::LocalSettings, "\"localSettings\""),
            (PermissionRuleSource::FlagSettings, "\"flagSettings\""),
            (PermissionRuleSource::PolicySettings, "\"policySettings\""),
            (PermissionRuleSource::CliArg, "\"cliArg\""),
            (PermissionRuleSource::Command, "\"command\""),
            (PermissionRuleSource::Session, "\"session\""),
        ];
        for (src, expected) in sources {
            let json = serde_json::to_string(&src).unwrap();
            assert_eq!(json, expected);
        }
    }

    #[test]
    fn permission_decision_construction() {
        let allow = PermissionDecision::Allow {
            message: Some("Auto-allowed".to_string()),
        };
        match &allow {
            PermissionDecision::Allow { message } => {
                assert_eq!(message.as_deref(), Some("Auto-allowed"));
            }
            _ => panic!("expected Allow"),
        }

        let deny = PermissionDecision::Deny {
            message: "Not permitted".to_string(),
        };
        match &deny {
            PermissionDecision::Deny { message } => assert_eq!(message, "Not permitted"),
            _ => panic!("expected Deny"),
        }

        let ask = PermissionDecision::Ask {
            message: "Confirm?".to_string(),
            allow_rules: vec![],
        };
        match &ask {
            PermissionDecision::Ask {
                message,
                allow_rules,
            } => {
                assert_eq!(message, "Confirm?");
                assert!(allow_rules.is_empty());
            }
            _ => panic!("expected Ask"),
        }
    }

    #[test]
    fn tool_permission_context_default() {
        let ctx = ToolPermissionContext::default();
        assert_eq!(ctx.mode, PermissionMode::Default);
        assert!(ctx.always_allow_rules.is_empty());
        assert!(ctx.always_deny_rules.is_empty());
        assert!(ctx.always_ask_rules.is_empty());
        assert!(!ctx.is_bypass_mode_available);
    }
}
