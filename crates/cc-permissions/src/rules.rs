use serde::{Deserialize, Serialize};

/// The behavior a permission rule dictates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionBehavior {
    Allow,
    Deny,
    Ask,
}

/// Where a permission rule originates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuleSource {
    UserSettings,
    ProjectSettings,
    LocalSettings,
    FlagSettings,
    PolicySettings,
    CliArg,
    Command,
    Session,
}

/// A single permission rule that can match tool invocations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    pub tool_name: String,
    #[serde(default)]
    pub input_pattern: Option<String>,
    pub behavior: PermissionBehavior,
    pub source: RuleSource,
}

impl PermissionRule {
    /// Check if this rule matches a given tool name and input.
    ///
    /// Tool name matching supports:
    /// - Exact match (e.g., "Bash" matches "Bash")
    /// - Glob-style wildcard at end (e.g., "mcp__*" matches "mcp__foo")
    ///
    /// If `input_pattern` is set, at least one string value in the input JSON
    /// must contain the pattern as a substring.
    pub fn matches(&self, tool_name: &str, input: &serde_json::Value) -> bool {
        if !self.matches_tool_name(tool_name) {
            return false;
        }

        if let Some(ref pattern) = self.input_pattern {
            return self.matches_input(pattern, input);
        }

        true
    }

    fn matches_tool_name(&self, tool_name: &str) -> bool {
        if self.tool_name == tool_name {
            return true;
        }

        // Glob-style matching: support wildcards (* and ?)
        if (self.tool_name.contains('*') || self.tool_name.contains('?'))
            && let Ok(glob) = globset::Glob::new(&self.tool_name) {
                let matcher = glob.compile_matcher();
                return matcher.is_match(tool_name);
            }

        false
    }

    fn matches_input(&self, pattern: &str, input: &serde_json::Value) -> bool {
        match input {
            serde_json::Value::String(s) => s.contains(pattern),
            serde_json::Value::Object(map) => {
                for value in map.values() {
                    if self.matches_input(pattern, value) {
                        return true;
                    }
                }
                false
            }
            serde_json::Value::Array(arr) => {
                for value in arr {
                    if self.matches_input(pattern, value) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }
}

/// Match a tool name against rules. First match wins.
pub fn find_matching_rule<'a>(
    rules: &'a [PermissionRule],
    tool_name: &str,
    input: &serde_json::Value,
) -> Option<&'a PermissionRule> {
    rules.iter().find(|rule| rule.matches(tool_name, input))
}

/// Parse rules from a settings JSON value.
///
/// Expects a JSON array of objects with fields: `tool`, `input_pattern` (optional),
/// `behavior`.
pub fn parse_rules_from_settings(
    settings: &serde_json::Value,
    source: RuleSource,
) -> Vec<PermissionRule> {
    let arr = match settings.as_array() {
        Some(a) => a,
        None => return Vec::new(),
    };

    let mut rules = Vec::new();
    for item in arr {
        let tool_name = match item.get("tool").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };

        let behavior = match item.get("behavior").and_then(|v| v.as_str()) {
            Some("allow") => PermissionBehavior::Allow,
            Some("deny") => PermissionBehavior::Deny,
            Some("ask") => PermissionBehavior::Ask,
            _ => continue,
        };

        let input_pattern = item
            .get("input_pattern")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        rules.push(PermissionRule {
            tool_name,
            input_pattern,
            behavior,
            source,
        });
    }

    rules
}

/// A collection of permission rules organized by behavior for efficient evaluation.
#[derive(Debug, Clone)]
pub struct PermissionRuleSet {
    pub allow_rules: Vec<PermissionRule>,
    pub deny_rules: Vec<PermissionRule>,
    pub ask_rules: Vec<PermissionRule>,
}

impl PermissionRuleSet {
    pub fn new() -> Self {
        Self {
            allow_rules: Vec::new(),
            deny_rules: Vec::new(),
            ask_rules: Vec::new(),
        }
    }

    pub fn add_rule(&mut self, rule: PermissionRule) {
        match rule.behavior {
            PermissionBehavior::Allow => self.allow_rules.push(rule),
            PermissionBehavior::Deny => self.deny_rules.push(rule),
            PermissionBehavior::Ask => self.ask_rules.push(rule),
        }
    }

    pub fn merge(&mut self, other: &PermissionRuleSet) {
        self.allow_rules.extend(other.allow_rules.iter().cloned());
        self.deny_rules.extend(other.deny_rules.iter().cloned());
        self.ask_rules.extend(other.ask_rules.iter().cloned());
    }

    /// Evaluate rules for a tool use. Returns first matching behavior.
    /// Deny rules are checked first, then allow, then ask.
    pub fn evaluate(
        &self,
        tool_name: &str,
        input: &serde_json::Value,
    ) -> Option<PermissionBehavior> {
        if let Some(_rule) = find_matching_rule(&self.deny_rules, tool_name, input) {
            return Some(PermissionBehavior::Deny);
        }
        if let Some(_rule) = find_matching_rule(&self.allow_rules, tool_name, input) {
            return Some(PermissionBehavior::Allow);
        }
        if let Some(_rule) = find_matching_rule(&self.ask_rules, tool_name, input) {
            return Some(PermissionBehavior::Ask);
        }
        None
    }
}

impl Default for PermissionRuleSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_rule(
        tool_name: &str,
        input_pattern: Option<&str>,
        behavior: PermissionBehavior,
    ) -> PermissionRule {
        PermissionRule {
            tool_name: tool_name.to_string(),
            input_pattern: input_pattern.map(|s| s.to_string()),
            behavior,
            source: RuleSource::UserSettings,
        }
    }

    #[test]
    fn test_exact_match() {
        let rule = make_rule("Bash", None, PermissionBehavior::Allow);
        assert!(rule.matches("Bash", &json!({})));
        assert!(!rule.matches("Edit", &json!({})));
    }

    #[test]
    fn test_glob_match() {
        let rule = make_rule("mcp__*", None, PermissionBehavior::Allow);
        assert!(rule.matches("mcp__server_tool", &json!({})));
        assert!(rule.matches("mcp__another", &json!({})));
        assert!(!rule.matches("Bash", &json!({})));
    }

    #[test]
    fn test_input_pattern_match() {
        let rule = make_rule("Bash", Some("git push"), PermissionBehavior::Deny);
        assert!(rule.matches("Bash", &json!({"command": "git push origin main"})));
        assert!(!rule.matches("Bash", &json!({"command": "git status"})));
    }

    #[test]
    fn test_input_pattern_nested() {
        let rule = make_rule("Bash", Some("dangerous"), PermissionBehavior::Deny);
        let input = json!({"args": {"nested": "this is dangerous stuff"}});
        assert!(rule.matches("Bash", &input));
    }

    #[test]
    fn test_input_pattern_array() {
        let rule = make_rule("Bash", Some("secret"), PermissionBehavior::Deny);
        let input = json!({"items": ["normal", "secret value"]});
        assert!(rule.matches("Bash", &input));
    }

    #[test]
    fn test_input_pattern_no_match() {
        let rule = make_rule("Bash", Some("rm -rf"), PermissionBehavior::Deny);
        assert!(!rule.matches("Bash", &json!({"command": "ls -la"})));
    }

    #[test]
    fn test_find_matching_rule_first_wins() {
        let rules = vec![
            make_rule("Bash", Some("git push"), PermissionBehavior::Deny),
            make_rule("Bash", None, PermissionBehavior::Allow),
        ];
        let input = json!({"command": "git push"});
        let matched = find_matching_rule(&rules, "Bash", &input).unwrap();
        assert_eq!(matched.behavior, PermissionBehavior::Deny);
    }

    #[test]
    fn test_find_matching_rule_none() {
        let rules = vec![make_rule("Edit", None, PermissionBehavior::Allow)];
        assert!(find_matching_rule(&rules, "Bash", &json!({})).is_none());
    }

    #[test]
    fn test_parse_rules_from_settings() {
        let settings = json!([
            {"tool": "Bash", "behavior": "allow"},
            {"tool": "Edit", "behavior": "deny", "input_pattern": ".env"},
            {"tool": "Read", "behavior": "ask"},
            {"invalid": true},
            {"tool": "Bad", "behavior": "invalid_behavior"},
        ]);
        let rules = parse_rules_from_settings(&settings, RuleSource::ProjectSettings);
        assert_eq!(rules.len(), 3);
        assert_eq!(rules[0].tool_name, "Bash");
        assert_eq!(rules[0].behavior, PermissionBehavior::Allow);
        assert_eq!(rules[1].tool_name, "Edit");
        assert_eq!(rules[1].input_pattern.as_deref(), Some(".env"));
        assert_eq!(rules[1].behavior, PermissionBehavior::Deny);
        assert_eq!(rules[2].behavior, PermissionBehavior::Ask);
    }

    #[test]
    fn test_parse_rules_not_array() {
        let settings = json!({"not": "an array"});
        let rules = parse_rules_from_settings(&settings, RuleSource::UserSettings);
        assert!(rules.is_empty());
    }

    #[test]
    fn test_rule_set_evaluate_deny_first() {
        let mut ruleset = PermissionRuleSet::new();
        ruleset.add_rule(make_rule("Bash", None, PermissionBehavior::Allow));
        ruleset.add_rule(make_rule("Bash", None, PermissionBehavior::Deny));

        let result = ruleset.evaluate("Bash", &json!({}));
        assert_eq!(result, Some(PermissionBehavior::Deny));
    }

    #[test]
    fn test_rule_set_evaluate_allow_before_ask() {
        let mut ruleset = PermissionRuleSet::new();
        ruleset.add_rule(make_rule("Bash", None, PermissionBehavior::Allow));
        ruleset.add_rule(make_rule("Bash", None, PermissionBehavior::Ask));

        let result = ruleset.evaluate("Bash", &json!({}));
        assert_eq!(result, Some(PermissionBehavior::Allow));
    }

    #[test]
    fn test_rule_set_evaluate_none() {
        let ruleset = PermissionRuleSet::new();
        assert_eq!(ruleset.evaluate("Bash", &json!({})), None);
    }

    #[test]
    fn test_rule_set_merge() {
        let mut set1 = PermissionRuleSet::new();
        set1.add_rule(make_rule("Bash", None, PermissionBehavior::Allow));

        let mut set2 = PermissionRuleSet::new();
        set2.add_rule(make_rule("Edit", None, PermissionBehavior::Deny));

        set1.merge(&set2);
        assert_eq!(set1.allow_rules.len(), 1);
        assert_eq!(set1.deny_rules.len(), 1);
    }

    #[test]
    fn test_serde_behavior() {
        let json = serde_json::to_string(&PermissionBehavior::Allow).unwrap();
        assert_eq!(json, "\"allow\"");
        let back: PermissionBehavior = serde_json::from_str(&json).unwrap();
        assert_eq!(back, PermissionBehavior::Allow);
    }

    #[test]
    fn test_serde_rule_source() {
        let json = serde_json::to_string(&RuleSource::ProjectSettings).unwrap();
        assert_eq!(json, "\"projectSettings\"");
    }

    #[test]
    fn test_glob_question_mark() {
        let rule = make_rule("mcp_?_tool", None, PermissionBehavior::Allow);
        assert!(rule.matches("mcp_x_tool", &json!({})));
        assert!(!rule.matches("mcp_xx_tool", &json!({})));
    }
}
