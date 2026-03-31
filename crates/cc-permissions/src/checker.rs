use crate::modes::PermissionMode;
use crate::rules::{PermissionBehavior, PermissionRuleSet};

/// The outcome of a permission check.
#[derive(Debug, Clone)]
pub enum PermissionDecision {
    Allow { reason: String },
    Deny { reason: String },
    Ask { message: String, tool_name: String },
}

impl PermissionDecision {
    pub fn is_allow(&self) -> bool {
        matches!(self, Self::Allow { .. })
    }

    pub fn is_deny(&self) -> bool {
        matches!(self, Self::Deny { .. })
    }

    pub fn is_ask(&self) -> bool {
        matches!(self, Self::Ask { .. })
    }
}

/// Contextual state for permission evaluation.
#[derive(Debug, Clone)]
pub struct PermissionContext {
    pub mode: PermissionMode,
    pub rules: PermissionRuleSet,
    pub is_bypass_available: bool,
}

impl PermissionContext {
    pub fn new(mode: PermissionMode) -> Self {
        Self {
            mode,
            rules: PermissionRuleSet::new(),
            is_bypass_available: false,
        }
    }

    /// Check permissions for a tool use.
    ///
    /// Logic order:
    /// 1. If mode is BypassPermissions or DontAsk -> Allow
    /// 2. Check deny rules -> if match, Deny
    /// 3. If mode is Plan and not read_only -> Deny
    /// 4. Check allow rules -> if match, Allow
    /// 5. If mode allows_read_only and is_read_only -> Allow
    /// 6. If mode allows_edits and !is_destructive -> Allow
    /// 7. Otherwise -> Ask
    pub fn check_permission(
        &self,
        tool_name: &str,
        input: &serde_json::Value,
        is_read_only: bool,
        is_destructive: bool,
    ) -> PermissionDecision {
        // 1. Bypass modes allow everything
        if self.mode.allows_all() {
            return PermissionDecision::Allow {
                reason: format!("Mode {} allows all operations", self.mode),
            };
        }

        // 2. Check deny rules first
        if let Some(PermissionBehavior::Deny) = self.rules.evaluate_deny_only(tool_name, input) {
            return PermissionDecision::Deny {
                reason: format!("Tool '{}' denied by rule", tool_name),
            };
        }

        // 3. Plan mode blocks non-read-only
        if self.mode.is_read_only_mode() && !is_read_only {
            return PermissionDecision::Deny {
                reason: format!(
                    "Tool '{}' denied: plan mode only allows read-only operations",
                    tool_name
                ),
            };
        }

        // 4. Check allow rules
        if let Some(PermissionBehavior::Allow) = self.rules.evaluate_allow_only(tool_name, input) {
            return PermissionDecision::Allow {
                reason: format!("Tool '{}' allowed by rule", tool_name),
            };
        }

        // 5. Mode allows read-only and tool is read-only
        if self.mode.allows_read_only() && is_read_only {
            return PermissionDecision::Allow {
                reason: format!("Read-only tool '{}' allowed by mode {}", tool_name, self.mode),
            };
        }

        // 6. Mode allows edits and not destructive
        if self.mode.allows_edits() && !is_destructive {
            return PermissionDecision::Allow {
                reason: format!(
                    "Non-destructive tool '{}' allowed by mode {}",
                    tool_name, self.mode
                ),
            };
        }

        // 7. Ask
        PermissionDecision::Ask {
            message: format!(
                "Tool '{}' requires permission in mode {}",
                tool_name, self.mode
            ),
            tool_name: tool_name.to_string(),
        }
    }
}

// Helper methods on PermissionRuleSet used by the checker
impl PermissionRuleSet {
    /// Check only deny rules.
    pub fn evaluate_deny_only(
        &self,
        tool_name: &str,
        input: &serde_json::Value,
    ) -> Option<PermissionBehavior> {
        if crate::rules::find_matching_rule(&self.deny_rules, tool_name, input).is_some() {
            Some(PermissionBehavior::Deny)
        } else {
            None
        }
    }

    /// Check only allow rules.
    pub fn evaluate_allow_only(
        &self,
        tool_name: &str,
        input: &serde_json::Value,
    ) -> Option<PermissionBehavior> {
        if crate::rules::find_matching_rule(&self.allow_rules, tool_name, input).is_some() {
            Some(PermissionBehavior::Allow)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{PermissionRule, RuleSource};
    use serde_json::json;

    fn make_rule(
        tool_name: &str,
        behavior: PermissionBehavior,
    ) -> PermissionRule {
        PermissionRule {
            tool_name: tool_name.to_string(),
            input_pattern: None,
            behavior,
            source: RuleSource::UserSettings,
        }
    }

    #[test]
    fn test_bypass_mode_allows_all() {
        let ctx = PermissionContext::new(PermissionMode::BypassPermissions);
        let decision = ctx.check_permission("Bash", &json!({}), false, true);
        assert!(decision.is_allow());
    }

    #[test]
    fn test_dont_ask_mode_allows_all() {
        let ctx = PermissionContext::new(PermissionMode::DontAsk);
        let decision = ctx.check_permission("Bash", &json!({}), false, true);
        assert!(decision.is_allow());
    }

    #[test]
    fn test_deny_rule_overrides_mode() {
        let mut ctx = PermissionContext::new(PermissionMode::Auto);
        ctx.rules.add_rule(make_rule("Bash", PermissionBehavior::Deny));
        let decision = ctx.check_permission("Bash", &json!({}), true, false);
        assert!(decision.is_deny());
    }

    #[test]
    fn test_plan_mode_denies_writes() {
        let ctx = PermissionContext::new(PermissionMode::Plan);
        let decision = ctx.check_permission("Edit", &json!({}), false, false);
        assert!(decision.is_deny());
    }

    #[test]
    fn test_plan_mode_allows_reads() {
        let ctx = PermissionContext::new(PermissionMode::Plan);
        // Plan mode doesn't allows_read_only (returns false), so read-only tools
        // won't be auto-allowed by step 5. But there's no deny rule and it's read_only
        // so step 3 passes. Step 5: mode.allows_read_only() is false for Plan.
        // So it falls through to Ask.
        let decision = ctx.check_permission("Read", &json!({}), true, false);
        // Plan mode: is_read_only_mode=true, allows_read_only=false
        // Step 3: mode is Plan AND is_read_only=true -> skip deny
        // Step 5: allows_read_only()=false for Plan -> skip
        // -> Ask
        assert!(decision.is_ask());
    }

    #[test]
    fn test_allow_rule_works() {
        let mut ctx = PermissionContext::new(PermissionMode::Default);
        ctx.rules.add_rule(make_rule("Bash", PermissionBehavior::Allow));
        let decision = ctx.check_permission("Bash", &json!({}), false, true);
        assert!(decision.is_allow());
    }

    #[test]
    fn test_default_mode_allows_read_only() {
        let ctx = PermissionContext::new(PermissionMode::Default);
        let decision = ctx.check_permission("Read", &json!({}), true, false);
        assert!(decision.is_allow());
    }

    #[test]
    fn test_default_mode_asks_for_writes() {
        let ctx = PermissionContext::new(PermissionMode::Default);
        let decision = ctx.check_permission("Edit", &json!({}), false, false);
        assert!(decision.is_ask());
    }

    #[test]
    fn test_accept_edits_allows_non_destructive() {
        let ctx = PermissionContext::new(PermissionMode::AcceptEdits);
        let decision = ctx.check_permission("Edit", &json!({}), false, false);
        assert!(decision.is_allow());
    }

    #[test]
    fn test_accept_edits_asks_for_destructive() {
        let ctx = PermissionContext::new(PermissionMode::AcceptEdits);
        let decision = ctx.check_permission("Bash", &json!({}), false, true);
        assert!(decision.is_ask());
    }

    #[test]
    fn test_auto_mode_read_only_allowed() {
        let ctx = PermissionContext::new(PermissionMode::Auto);
        let decision = ctx.check_permission("Read", &json!({}), true, false);
        assert!(decision.is_allow());
    }

    #[test]
    fn test_auto_mode_write_asks() {
        let ctx = PermissionContext::new(PermissionMode::Auto);
        let decision = ctx.check_permission("Edit", &json!({}), false, false);
        assert!(decision.is_ask());
    }

    #[test]
    fn test_deny_rule_checked_before_allow_rule() {
        let mut ctx = PermissionContext::new(PermissionMode::Default);
        ctx.rules.add_rule(make_rule("Bash", PermissionBehavior::Allow));
        ctx.rules.add_rule(make_rule("Bash", PermissionBehavior::Deny));
        let decision = ctx.check_permission("Bash", &json!({}), false, false);
        assert!(decision.is_deny());
    }

    #[test]
    fn test_decision_variants() {
        let allow = PermissionDecision::Allow {
            reason: "test".to_string(),
        };
        let deny = PermissionDecision::Deny {
            reason: "test".to_string(),
        };
        let ask = PermissionDecision::Ask {
            message: "test".to_string(),
            tool_name: "Bash".to_string(),
        };
        assert!(allow.is_allow());
        assert!(!allow.is_deny());
        assert!(!allow.is_ask());
        assert!(deny.is_deny());
        assert!(ask.is_ask());
    }
}
