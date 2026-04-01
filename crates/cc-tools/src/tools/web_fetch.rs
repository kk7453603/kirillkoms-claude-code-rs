use async_trait::async_trait;
use serde_json::{Value, json};

use crate::trait_def::{
    RenderedContent, SearchReadInfo, Tool, ToolError, ToolResult, ValidationResult,
};

pub struct WebFetchTool;

impl WebFetchTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "WebFetch"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch content from"
                },
                "prompt": {
                    "type": "string",
                    "description": "A prompt to describe what information to extract from the page"
                }
            },
            "required": ["url"]
        })
    }

    fn description(&self) -> String {
        "Fetches content from a URL and returns the raw text. Useful for reading web pages, API responses, and documentation.".to_string()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    fn is_concurrency_safe(&self, _input: &Value) -> bool {
        true
    }

    fn should_defer(&self) -> bool {
        true
    }

    fn search_read_info(&self, _input: &Value) -> SearchReadInfo {
        SearchReadInfo {
            is_search: false,
            is_read: true,
            is_list: false,
        }
    }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        match input.get("url").and_then(|v| v.as_str()) {
            Some(u) if !u.is_empty() => {
                if url::Url::parse(u).is_ok() {
                    ValidationResult::Ok
                } else {
                    ValidationResult::Error {
                        message: format!("Invalid URL: '{}'", u),
                    }
                }
            }
            _ => ValidationResult::Error {
                message: "Missing or empty 'url' parameter".to_string(),
            },
        }
    }

    fn render_tool_use(&self, input: &Value) -> RenderedContent {
        let url = input
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("<unknown>");
        RenderedContent::Styled {
            text: format!("Fetch: {}", url),
            bold: true,
            dim: false,
            color: Some("blue".to_string()),
        }
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let url = input
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or(ToolError::ValidationFailed {
                message: "Missing 'url' parameter".into(),
            })?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("claude-code/0.1")
            .build()
            .map_err(|e| ToolError::ExecutionFailed {
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                message: format!("Failed to fetch URL '{}': {}", url, e),
            })?;

        let status = response.status();
        if !status.is_success() {
            return Ok(ToolResult::error(&format!(
                "HTTP {} when fetching '{}'",
                status, url
            )));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        let body = response
            .text()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                message: format!("Failed to read response body: {}", e),
            })?;

        // Truncate very large responses
        let max_chars = 100_000;
        let truncated = if body.len() > max_chars {
            format!(
                "{}\n\n... (truncated, {} total characters)",
                &body[..max_chars],
                body.len()
            )
        } else {
            body
        };

        let result = format!(
            "URL: {}\nContent-Type: {}\nLength: {} chars\n\n{}",
            url,
            content_type,
            truncated.len(),
            truncated
        );

        Ok(ToolResult::text(&result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_and_schema() {
        let tool = WebFetchTool::new();
        assert_eq!(tool.name(), "WebFetch");
        let schema = tool.input_schema();
        assert!(schema["properties"]["url"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("url")));
    }

    #[test]
    fn test_is_read_only() {
        let tool = WebFetchTool::new();
        assert!(tool.is_read_only(&json!({})));
    }

    #[test]
    fn test_validate_input() {
        let tool = WebFetchTool::new();
        assert!(matches!(
            tool.validate_input(&json!({"url": "https://example.com"})),
            ValidationResult::Ok
        ));
        assert!(matches!(
            tool.validate_input(&json!({})),
            ValidationResult::Error { .. }
        ));
        assert!(matches!(
            tool.validate_input(&json!({"url": "not a url"})),
            ValidationResult::Error { .. }
        ));
    }

    #[test]
    fn test_should_defer() {
        let tool = WebFetchTool::new();
        assert!(tool.should_defer());
    }

    #[test]
    fn test_description() {
        let tool = WebFetchTool::new();
        assert!(tool.description().contains("Fetches content"));
    }
}
