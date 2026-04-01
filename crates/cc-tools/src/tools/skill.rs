use async_trait::async_trait;
use serde_json::{Value, json};

use crate::trait_def::{Tool, ToolError, ToolResult, ValidationResult};

pub struct SkillTool;

impl SkillTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SkillTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &str {
        "Skill"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "skill": {
                    "type": "string",
                    "description": "The skill name to invoke. E.g., 'commit', 'review-pr', or 'pdf'"
                },
                "args": {
                    "type": "string",
                    "description": "Optional arguments for the skill"
                }
            },
            "required": ["skill"]
        })
    }

    fn description(&self) -> String {
        "Execute a skill within the conversation. Skills provide specialized capabilities and domain knowledge.".to_string()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    fn is_concurrency_safe(&self, _input: &Value) -> bool {
        false
    }

    fn should_defer(&self) -> bool {
        true
    }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        match input.get("skill").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => ValidationResult::Ok,
            _ => ValidationResult::Error {
                message: "Missing or empty 'skill' parameter".to_string(),
            },
        }
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let skill_name =
            input
                .get("skill")
                .and_then(|v| v.as_str())
                .ok_or(ToolError::ValidationFailed {
                    message: "Missing 'skill' parameter".into(),
                })?;
        let args = input.get("args").and_then(|v| v.as_str()).unwrap_or("");

        // Look up in bundled skills
        let skills = cc_skills::bundled::bundled_skills();
        let skill = skills
            .iter()
            .find(|s| s.name == skill_name)
            .ok_or_else(|| ToolError::ExecutionFailed {
                message: format!(
                    "Skill '{}' not found. Available: {}",
                    skill_name,
                    skills
                        .iter()
                        .map(|s| s.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            })?;

        // Build prompt from template
        let prompt = if args.is_empty() {
            skill.prompt_template.clone()
        } else {
            format!("{}\n\nArguments: {}", skill.prompt_template, args)
        };

        Ok(ToolResult::text(&format!(
            "Executing skill '{}': {}",
            skill_name, prompt
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_and_schema() {
        let tool = SkillTool::new();
        assert_eq!(tool.name(), "Skill");
        let schema = tool.input_schema();
        assert!(schema["properties"]["skill"].is_object());
        assert!(schema["properties"]["args"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("skill")));
    }

    #[test]
    fn test_validate_input() {
        let tool = SkillTool::new();
        assert!(matches!(
            tool.validate_input(&json!({"skill": "commit"})),
            ValidationResult::Ok
        ));
        assert!(matches!(
            tool.validate_input(&json!({})),
            ValidationResult::Error { .. }
        ));
    }

    #[tokio::test]
    async fn test_call_known_skill() {
        let tool = SkillTool::new();
        let result = tool
            .call(json!({"skill": "commit", "args": "-m 'fix'"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.content.as_str().unwrap();
        assert!(text.contains("commit"));
        assert!(text.contains("Arguments: -m 'fix'"));
    }

    #[tokio::test]
    async fn test_call_unknown_skill() {
        let tool = SkillTool::new();
        let result = tool.call(json!({"skill": "nonexistent"})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_call_skill_no_args() {
        let tool = SkillTool::new();
        let result = tool.call(json!({"skill": "review-pr"})).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.as_str().unwrap().contains("review-pr"));
    }

    #[test]
    fn test_should_defer() {
        let tool = SkillTool::new();
        assert!(tool.should_defer());
    }
}
