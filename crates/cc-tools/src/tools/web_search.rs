use async_trait::async_trait;
use serde_json::{json, Value};

use crate::trait_def::{
    RenderedContent, SearchReadInfo, Tool, ToolError, ToolResult, ValidationResult,
};

pub struct WebSearchTool;

impl WebSearchTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "WebSearch"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query to submit to the web search engine"
                },
                "allowed_domains": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional list of domains to restrict search results to"
                },
                "blocked_domains": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional list of domains to exclude from search results"
                }
            },
            "required": ["query"]
        })
    }

    fn description(&self) -> String {
        "Search the web for information. Returns search results with titles, URLs, and snippets.".to_string()
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
            is_search: true,
            is_read: false,
            is_list: false,
        }
    }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        match input.get("query").and_then(|v| v.as_str()) {
            Some(q) if !q.is_empty() => ValidationResult::Ok,
            _ => ValidationResult::Error {
                message: "Missing or empty 'query' parameter".to_string(),
            },
        }
    }

    fn render_tool_use(&self, input: &Value) -> RenderedContent {
        let query = input
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("<unknown>");
        RenderedContent::Styled {
            text: format!("Search: {}", query),
            bold: true,
            dim: false,
            color: Some("magenta".to_string()),
        }
    }

    async fn call(&self, _input: Value) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::error(
            "Web search is not yet available. This feature requires integration with a search API (e.g., Brave Search, Google Custom Search). Please use the WebFetch tool with a known URL instead, or use Bash with curl to fetch specific web pages.",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_and_schema() {
        let tool = WebSearchTool::new();
        assert_eq!(tool.name(), "WebSearch");
        let schema = tool.input_schema();
        assert!(schema["properties"]["query"].is_object());
        assert!(schema["properties"]["allowed_domains"].is_object());
        assert!(schema["properties"]["blocked_domains"].is_object());
    }

    #[test]
    fn test_validate_input() {
        let tool = WebSearchTool::new();
        assert!(matches!(
            tool.validate_input(&json!({"query": "rust async"})),
            ValidationResult::Ok
        ));
        assert!(matches!(
            tool.validate_input(&json!({})),
            ValidationResult::Error { .. }
        ));
    }

    #[tokio::test]
    async fn test_call_returns_error() {
        let tool = WebSearchTool::new();
        let result = tool
            .call(json!({"query": "test"}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.as_str().unwrap().contains("not yet available"));
    }

    #[test]
    fn test_should_defer() {
        let tool = WebSearchTool::new();
        assert!(tool.should_defer());
    }
}
