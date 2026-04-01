use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::trait_def::{
    RenderedContent, SearchReadInfo, Tool, ToolError, ToolResult, ValidationResult,
};

const DEFAULT_SEARXNG_URL: &str = "https://searx.be";
const MAX_RESULTS: usize = 10;

#[derive(Debug, Deserialize)]
struct SearxngResponse {
    results: Vec<SearxngResult>,
}

#[derive(Debug, Deserialize)]
struct SearxngResult {
    title: String,
    url: String,
    #[serde(default)]
    content: String,
}

pub struct WebSearchTool;

impl WebSearchTool {
    pub fn new() -> Self {
        Self
    }

    fn base_url() -> String {
        std::env::var("SEARXNG_URL").unwrap_or_else(|_| DEFAULT_SEARXNG_URL.to_string())
    }

    fn build_query(query: &str, input: &Value) -> String {
        let mut parts = vec![query.to_string()];

        if let Some(domains) = input.get("allowed_domains").and_then(|v| v.as_array()) {
            for domain in domains {
                if let Some(d) = domain.as_str() {
                    parts.push(format!("site:{d}"));
                }
            }
        }

        if let Some(domains) = input.get("blocked_domains").and_then(|v| v.as_array()) {
            for domain in domains {
                if let Some(d) = domain.as_str() {
                    parts.push(format!("-site:{d}"));
                }
            }
        }

        parts.join(" ")
    }

    fn format_results(results: &[SearxngResult]) -> String {
        if results.is_empty() {
            return "No results found.".to_string();
        }

        let count = results.len().min(MAX_RESULTS);
        let mut output = format!("Found {} result(s):\n", count);

        for (i, result) in results.iter().take(MAX_RESULTS).enumerate() {
            output.push_str(&format!(
                "\n{}. {}\n   URL: {}\n",
                i + 1,
                result.title,
                result.url
            ));
            if !result.content.is_empty() {
                output.push_str(&format!("   {}\n", result.content));
            }
        }

        output
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
        "Search the web for information. Returns search results with titles, URLs, and snippets."
            .to_string()
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

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let query = input.get("query").and_then(|v| v.as_str()).ok_or_else(|| {
            ToolError::ValidationFailed {
                message: "Missing 'query' parameter".to_string(),
            }
        })?;

        let full_query = Self::build_query(query, &input);
        let base_url = Self::base_url();
        let search_url = format!("{base_url}/search");

        let client = reqwest::Client::new();
        let response = client
            .get(&search_url)
            .query(&[("q", &full_query), ("format", &"json".to_string())])
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                message: format!("HTTP request failed: {e}"),
            })?;

        if !response.status().is_success() {
            return Ok(ToolResult::error(&format!(
                "SearXNG returned HTTP {}",
                response.status()
            )));
        }

        let searxng_response: SearxngResponse =
            response
                .json()
                .await
                .map_err(|e| ToolError::ExecutionFailed {
                    message: format!("Failed to parse response: {e}"),
                })?;

        let text = Self::format_results(&searxng_response.results);
        Ok(ToolResult::text(&text))
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
        assert!(matches!(
            tool.validate_input(&json!({"query": ""})),
            ValidationResult::Error { .. }
        ));
    }

    #[test]
    fn test_should_defer() {
        let tool = WebSearchTool::new();
        assert!(tool.should_defer());
    }

    #[test]
    fn test_build_query_plain() {
        let input = json!({"query": "rust programming"});
        let q = WebSearchTool::build_query("rust programming", &input);
        assert_eq!(q, "rust programming");
    }

    #[test]
    fn test_build_query_with_allowed_domains() {
        let input = json!({
            "query": "rust async",
            "allowed_domains": ["docs.rs", "crates.io"]
        });
        let q = WebSearchTool::build_query("rust async", &input);
        assert_eq!(q, "rust async site:docs.rs site:crates.io");
    }

    #[test]
    fn test_build_query_with_blocked_domains() {
        let input = json!({
            "query": "rust async",
            "blocked_domains": ["pinterest.com"]
        });
        let q = WebSearchTool::build_query("rust async", &input);
        assert_eq!(q, "rust async -site:pinterest.com");
    }

    #[test]
    fn test_build_query_with_both_domain_filters() {
        let input = json!({
            "query": "test",
            "allowed_domains": ["example.com"],
            "blocked_domains": ["spam.com"]
        });
        let q = WebSearchTool::build_query("test", &input);
        assert_eq!(q, "test site:example.com -site:spam.com");
    }

    #[test]
    fn test_format_results_empty() {
        let results: Vec<SearxngResult> = vec![];
        let text = WebSearchTool::format_results(&results);
        assert_eq!(text, "No results found.");
    }

    #[test]
    fn test_format_results_single() {
        let results = vec![SearxngResult {
            title: "Rust Lang".to_string(),
            url: "https://www.rust-lang.org".to_string(),
            content: "A systems programming language.".to_string(),
        }];
        let text = WebSearchTool::format_results(&results);
        assert!(text.contains("Found 1 result(s):"));
        assert!(text.contains("1. Rust Lang"));
        assert!(text.contains("URL: https://www.rust-lang.org"));
        assert!(text.contains("A systems programming language."));
    }

    #[test]
    fn test_format_results_no_content() {
        let results = vec![SearxngResult {
            title: "Example".to_string(),
            url: "https://example.com".to_string(),
            content: String::new(),
        }];
        let text = WebSearchTool::format_results(&results);
        assert!(text.contains("1. Example"));
        assert!(text.contains("URL: https://example.com"));
        // Empty content line should not appear
        let lines: Vec<&str> = text.lines().collect();
        assert!(
            !lines
                .iter()
                .any(|l| l.trim().is_empty() && l.starts_with("   "))
        );
    }

    #[test]
    fn test_format_results_max_10() {
        let results: Vec<SearxngResult> = (0..15)
            .map(|i| SearxngResult {
                title: format!("Result {i}"),
                url: format!("https://example.com/{i}"),
                content: format!("Content {i}"),
            })
            .collect();
        let text = WebSearchTool::format_results(&results);
        assert!(text.contains("Found 10 result(s):"));
        assert!(text.contains("10. Result 9"));
        assert!(!text.contains("11."));
    }

    #[test]
    fn test_default_base_url() {
        // When SEARXNG_URL is not set, should use default
        // (This test might pick up env var if set, so we just check it returns a string)
        let url = WebSearchTool::base_url();
        assert!(url.starts_with("http"));
    }

    #[test]
    fn test_render_tool_use() {
        let tool = WebSearchTool::new();
        let rendered = tool.render_tool_use(&json!({"query": "hello world"}));
        match rendered {
            RenderedContent::Styled { text, bold, .. } => {
                assert_eq!(text, "Search: hello world");
                assert!(bold);
            }
            _ => panic!("Expected Styled variant"),
        }
    }

    #[test]
    fn test_deserialize_searxng_response() {
        let json_str = r#"{
            "results": [
                {
                    "title": "Test Title",
                    "url": "https://example.com",
                    "content": "Test content"
                },
                {
                    "title": "No Content",
                    "url": "https://example.com/2"
                }
            ]
        }"#;
        let response: SearxngResponse = serde_json::from_str(json_str).unwrap();
        assert_eq!(response.results.len(), 2);
        assert_eq!(response.results[0].title, "Test Title");
        assert_eq!(response.results[0].content, "Test content");
        assert_eq!(response.results[1].content, ""); // default
    }
}
