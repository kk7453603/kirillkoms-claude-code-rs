use async_trait::async_trait;
use serde_json::{json, Value};

use crate::trait_def::{SearchReadInfo, Tool, ToolError, ToolResult, ValidationResult};

pub struct ToolSearchTool {
    /// Stored tool info for searching: (name, description)
    tool_info: Vec<(String, String)>,
}

impl ToolSearchTool {
    pub fn new() -> Self {
        Self {
            tool_info: Vec::new(),
        }
    }

    /// Create with tool registry info for searching
    pub fn with_tools(tools: Vec<(String, String)>) -> Self {
        Self { tool_info: tools }
    }

    /// Update the available tools list
    pub fn set_tools(&mut self, tools: Vec<(String, String)>) {
        self.tool_info = tools;
    }

    fn score_match(query: &str, name: &str, description: &str) -> f64 {
        let query_lower = query.to_lowercase();
        let name_lower = name.to_lowercase();
        let desc_lower = description.to_lowercase();

        let mut score = 0.0;

        // Exact name match
        if name_lower == query_lower {
            score += 100.0;
        } else if name_lower.contains(&query_lower) {
            score += 50.0;
        }

        // Keyword matching
        for word in query_lower.split_whitespace() {
            if name_lower.contains(word) {
                score += 20.0;
            }
            if desc_lower.contains(word) {
                score += 10.0;
            }
        }

        // Prefix matching
        if query_lower.starts_with("select:") {
            let names: Vec<&str> = query_lower["select:".len()..].split(',').collect();
            for n in names {
                if name_lower == n.trim() {
                    score += 200.0;
                }
            }
        }

        // Require-in-name matching (+ prefix)
        if query_lower.starts_with('+') {
            let parts: Vec<&str> = query_lower[1..].split_whitespace().collect();
            if let Some(required) = parts.first() {
                if !name_lower.contains(required) {
                    return 0.0;
                }
                for part in &parts[1..] {
                    if name_lower.contains(part) {
                        score += 15.0;
                    }
                    if desc_lower.contains(part) {
                        score += 8.0;
                    }
                }
            }
        }

        score
    }
}

impl Default for ToolSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ToolSearchTool {
    fn name(&self) -> &str {
        "ToolSearch"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Query to find deferred tools. Use 'select:<tool_name>' for direct selection, or keywords to search."
                },
                "max_results": {
                    "type": "number",
                    "description": "Maximum number of results to return (default: 5)"
                }
            },
            "required": ["query"]
        })
    }

    fn description(&self) -> String {
        "Search for available tools by name or description keywords. Use 'select:ToolName' for exact matches.".to_string()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    fn is_concurrency_safe(&self, _input: &Value) -> bool {
        true
    }

    fn search_read_info(&self, _input: &Value) -> SearchReadInfo {
        SearchReadInfo {
            is_search: true,
            is_read: false,
            is_list: true,
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

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let query = input
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or(ToolError::ValidationFailed {
                message: "Missing 'query' parameter".into(),
            })?;

        let max_results = input
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(5) as usize;

        if self.tool_info.is_empty() {
            return Ok(ToolResult::text(
                "No tools registered in the search index. Tools will be available once the registry is initialized.",
            ));
        }

        let mut scored: Vec<(f64, &str, &str)> = self
            .tool_info
            .iter()
            .map(|(name, desc)| (Self::score_match(query, name, desc), name.as_str(), desc.as_str()))
            .filter(|(score, _, _)| *score > 0.0)
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(max_results);

        if scored.is_empty() {
            return Ok(ToolResult::text(&format!(
                "No tools found matching query: '{}'",
                query
            )));
        }

        let mut result = format!("Found {} tool(s) matching '{}':\n\n", scored.len(), query);
        for (score, name, desc) in &scored {
            let _ = score;
            result.push_str(&format!("  {} - {}\n", name, desc));
        }

        Ok(ToolResult::text(&result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tools() -> Vec<(String, String)> {
        vec![
            ("Bash".into(), "Execute bash commands".into()),
            ("Read".into(), "Read files from disk".into()),
            ("Edit".into(), "Edit files with string replacement".into()),
            ("Grep".into(), "Search file contents with regex".into()),
            ("Glob".into(), "Find files by pattern".into()),
        ]
    }

    #[test]
    fn test_name_and_schema() {
        let tool = ToolSearchTool::new();
        assert_eq!(tool.name(), "ToolSearch");
        let schema = tool.input_schema();
        assert!(schema["properties"]["query"].is_object());
    }

    #[test]
    fn test_score_exact_match() {
        let score = ToolSearchTool::score_match("Bash", "Bash", "Execute bash commands");
        assert!(score > 50.0);
    }

    #[test]
    fn test_score_keyword() {
        let score = ToolSearchTool::score_match("search", "Grep", "Search file contents with regex");
        assert!(score > 0.0);
    }

    #[tokio::test]
    async fn test_search_finds_tools() {
        let tool = ToolSearchTool::with_tools(sample_tools());
        let result = tool
            .call(json!({"query": "file"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.content.as_str().unwrap();
        // Should find Read, Edit, Grep, Glob (all mention "file")
        assert!(text.contains("Read") || text.contains("Edit") || text.contains("Grep") || text.contains("Glob"));
    }

    #[tokio::test]
    async fn test_search_select_syntax() {
        let tool = ToolSearchTool::with_tools(sample_tools());
        let result = tool
            .call(json!({"query": "select:Bash,Grep"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.content.as_str().unwrap();
        assert!(text.contains("Bash"));
        assert!(text.contains("Grep"));
    }

    #[tokio::test]
    async fn test_search_no_results() {
        let tool = ToolSearchTool::with_tools(sample_tools());
        let result = tool
            .call(json!({"query": "zzz_nonexistent"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        assert!(result.content.as_str().unwrap().contains("No tools found"));
    }
}
