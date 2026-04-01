use async_trait::async_trait;
use serde_json::{json, Value};

use crate::trait_def::{Tool, ToolError, ToolResult, ValidationResult};

pub struct AskUserQuestionTool;

impl AskUserQuestionTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AskUserQuestionTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for AskUserQuestionTool {
    fn name(&self) -> &str {
        "AskUserQuestion"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "description": "Array of questions to ask the user",
                    "items": {
                        "type": "object",
                        "properties": {
                            "question": {
                                "type": "string",
                                "description": "The question text"
                            },
                            "header": {
                                "type": "string",
                                "description": "Optional header/title for the question"
                            },
                            "options": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Optional list of answer options"
                            },
                            "multiSelect": {
                                "type": "boolean",
                                "description": "If true, allow selecting multiple options"
                            }
                        },
                        "required": ["question"]
                    }
                }
            },
            "required": ["questions"]
        })
    }

    fn description(&self) -> String {
        "Ask the user one or more questions, optionally with predefined answer options.".to_string()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    fn is_concurrency_safe(&self, _input: &Value) -> bool {
        false
    }

    fn should_defer(&self) -> bool {
        true
    }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        match input.get("questions").and_then(|v| v.as_array()) {
            Some(arr) if !arr.is_empty() => {
                for (i, q) in arr.iter().enumerate() {
                    if q.get("question").and_then(|v| v.as_str()).is_none() {
                        return ValidationResult::Error {
                            message: format!("Question {} is missing 'question' field", i),
                        };
                    }
                }
                ValidationResult::Ok
            }
            Some(_) => ValidationResult::Error {
                message: "Questions array must not be empty".to_string(),
            },
            None => ValidationResult::Error {
                message: "Missing or invalid 'questions' array parameter".to_string(),
            },
        }
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let questions = input
            .get("questions")
            .and_then(|v| v.as_array())
            .ok_or(ToolError::ValidationFailed {
                message: "Missing 'questions' parameter".into(),
            })?;

        // In a real implementation, this would pause and wait for user input.
        // For now, return the questions that would be asked.
        let mut output = String::from("Questions to present to user:\n\n");

        for (i, q) in questions.iter().enumerate() {
            let question = q
                .get("question")
                .and_then(|v| v.as_str())
                .unwrap_or("(no question)");

            if let Some(header) = q.get("header").and_then(|v| v.as_str()) {
                output.push_str(&format!("{}. [{}] {}\n", i + 1, header, question));
            } else {
                output.push_str(&format!("{}. {}\n", i + 1, question));
            }

            if let Some(options) = q.get("options").and_then(|v| v.as_array()) {
                let multi = q
                    .get("multiSelect")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let mode = if multi { "multi-select" } else { "single-select" };
                output.push_str(&format!("   Options ({}): ", mode));
                let opts: Vec<&str> = options.iter().filter_map(|o| o.as_str()).collect();
                output.push_str(&opts.join(", "));
                output.push('\n');
            }
        }

        output.push_str("\n(User input collection requires TUI integration)");

        Ok(ToolResult::text(&output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_and_schema() {
        let tool = AskUserQuestionTool::new();
        assert_eq!(tool.name(), "AskUserQuestion");
        let schema = tool.input_schema();
        assert!(schema["properties"]["questions"].is_object());
    }

    #[test]
    fn test_validate_input() {
        let tool = AskUserQuestionTool::new();
        assert!(matches!(
            tool.validate_input(&json!({
                "questions": [{"question": "What color?"}]
            })),
            ValidationResult::Ok
        ));
        assert!(matches!(
            tool.validate_input(&json!({"questions": []})),
            ValidationResult::Error { .. }
        ));
        assert!(matches!(
            tool.validate_input(&json!({})),
            ValidationResult::Error { .. }
        ));
    }

    #[tokio::test]
    async fn test_call() {
        let tool = AskUserQuestionTool::new();
        let result = tool
            .call(json!({
                "questions": [
                    {"question": "What language?", "options": ["Rust", "Go", "Python"]},
                    {"question": "Why?", "header": "Follow-up"}
                ]
            }))
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.content.as_str().unwrap();
        assert!(text.contains("What language?"));
        assert!(text.contains("Rust"));
        assert!(text.contains("Follow-up"));
    }

    #[test]
    fn test_should_defer() {
        let tool = AskUserQuestionTool::new();
        assert!(tool.should_defer());
    }
}
