use async_trait::async_trait;
use globset::{Glob, GlobMatcher};
use regex::{Regex, RegexBuilder};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::trait_def::{
    RenderedContent, SearchReadInfo, Tool, ToolError, ToolResult, ValidationResult,
};

/// Directories to always skip when walking.
const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    ".hg",
    ".svn",
    "__pycache__",
    ".tox",
    ".mypy_cache",
    ".pytest_cache",
    ".venv",
    "venv",
    "dist",
    "build",
    ".next",
    ".nuxt",
    "coverage",
    ".cache",
    ".parcel-cache",
];

/// Map short type names to glob patterns (mirrors ripgrep --type).
fn type_to_globs(ty: &str) -> Option<Vec<&'static str>> {
    match ty {
        "rust" | "rs" => Some(vec!["*.rs"]),
        "py" | "python" => Some(vec!["*.py", "*.pyi"]),
        "js" | "javascript" => Some(vec!["*.js", "*.mjs", "*.cjs", "*.jsx"]),
        "ts" | "typescript" => Some(vec!["*.ts", "*.mts", "*.cts", "*.tsx"]),
        "go" => Some(vec!["*.go"]),
        "java" => Some(vec!["*.java"]),
        "c" => Some(vec!["*.c", "*.h"]),
        "cpp" => Some(vec!["*.cpp", "*.cc", "*.cxx", "*.hpp", "*.hh", "*.hxx", "*.h"]),
        "ruby" | "rb" => Some(vec!["*.rb"]),
        "php" => Some(vec!["*.php"]),
        "swift" => Some(vec!["*.swift"]),
        "kotlin" | "kt" => Some(vec!["*.kt", "*.kts"]),
        "scala" => Some(vec!["*.scala"]),
        "html" => Some(vec!["*.html", "*.htm"]),
        "css" => Some(vec!["*.css"]),
        "json" => Some(vec!["*.json"]),
        "yaml" | "yml" => Some(vec!["*.yaml", "*.yml"]),
        "toml" => Some(vec!["*.toml"]),
        "xml" => Some(vec!["*.xml"]),
        "md" | "markdown" => Some(vec!["*.md", "*.markdown"]),
        "sh" | "shell" | "bash" => Some(vec!["*.sh", "*.bash"]),
        "sql" => Some(vec!["*.sql"]),
        "r" => Some(vec!["*.r", "*.R"]),
        "lua" => Some(vec!["*.lua"]),
        "dart" => Some(vec!["*.dart"]),
        "zig" => Some(vec!["*.zig"]),
        _ => None,
    }
}

fn is_binary_content(buf: &[u8]) -> bool {
    let check_len = buf.len().min(8192);
    buf[..check_len].contains(&0)
}

pub struct GrepTool;

impl GrepTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GrepTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "Grep"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The regular expression pattern to search for in file contents"
                },
                "path": {
                    "type": "string",
                    "description": "File or directory to search in. Defaults to current working directory."
                },
                "glob": {
                    "type": "string",
                    "description": "Glob pattern to filter files (e.g. \"*.js\", \"*.{ts,tsx}\")"
                },
                "output_mode": {
                    "type": "string",
                    "description": "Output mode: \"content\" shows matching lines, \"files_with_matches\" shows file paths (default), \"count\" shows match counts.",
                    "enum": ["content", "files_with_matches", "count"]
                },
                "-A": {
                    "type": "number",
                    "description": "Number of lines to show after each match. Requires output_mode: \"content\"."
                },
                "-B": {
                    "type": "number",
                    "description": "Number of lines to show before each match. Requires output_mode: \"content\"."
                },
                "-C": {
                    "type": "number",
                    "description": "Number of lines of context to show around each match. Alias for context."
                },
                "context": {
                    "type": "number",
                    "description": "Number of lines to show before and after each match."
                },
                "-n": {
                    "type": "boolean",
                    "description": "Show line numbers in output. Defaults to true."
                },
                "-i": {
                    "type": "boolean",
                    "description": "Case insensitive search."
                },
                "head_limit": {
                    "type": "number",
                    "description": "Limit output to first N lines/entries. Defaults to 250."
                },
                "offset": {
                    "type": "number",
                    "description": "Skip first N entries before applying head_limit. Defaults to 0."
                },
                "multiline": {
                    "type": "boolean",
                    "description": "Enable multiline mode where . matches newlines. Default: false."
                },
                "type": {
                    "type": "string",
                    "description": "File type to search (e.g. js, py, rust, go, java)."
                }
            },
            "required": ["pattern"]
        })
    }

    fn description(&self) -> String {
        "A powerful search tool built on regex. Supports full regex syntax, file filtering by glob or type, multiple output modes, and context lines.".to_string()
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
            is_list: false,
        }
    }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        match input.get("pattern").and_then(|v| v.as_str()) {
            Some(p) if !p.is_empty() => ValidationResult::Ok,
            _ => ValidationResult::Error {
                message: "Missing or empty 'pattern' parameter".to_string(),
            },
        }
    }

    fn render_tool_use(&self, input: &Value) -> RenderedContent {
        let pattern = input
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("<unknown>");
        let path = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        RenderedContent::Styled {
            text: format!("Grep: {} in {}", pattern, path),
            bold: true,
            dim: false,
            color: Some("cyan".to_string()),
        }
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let pattern_str = input
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or(ToolError::ValidationFailed {
                message: "Missing 'pattern' parameter".into(),
            })?;

        let search_path = input
            .get("path")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let output_mode = input
            .get("output_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("files_with_matches");

        let case_insensitive = input.get("-i").and_then(|v| v.as_bool()).unwrap_or(false);
        let multiline = input.get("multiline").and_then(|v| v.as_bool()).unwrap_or(false);
        let show_line_numbers = input.get("-n").and_then(|v| v.as_bool()).unwrap_or(true);
        let head_limit = input
            .get("head_limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(250) as usize;
        let offset = input
            .get("offset")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        // Context lines
        let context_c = input
            .get("-C")
            .and_then(|v| v.as_u64())
            .or_else(|| input.get("context").and_then(|v| v.as_u64()));
        let after_context = input
            .get("-A")
            .and_then(|v| v.as_u64())
            .or(context_c)
            .unwrap_or(0) as usize;
        let before_context = input
            .get("-B")
            .and_then(|v| v.as_u64())
            .or(context_c)
            .unwrap_or(0) as usize;

        // Build glob filter
        let glob_pattern = input.get("glob").and_then(|v| v.as_str());
        let type_filter = input.get("type").and_then(|v| v.as_str());

        let glob_matchers: Vec<GlobMatcher> = build_glob_matchers(glob_pattern, type_filter)?;

        // Build regex
        let re = RegexBuilder::new(pattern_str)
            .case_insensitive(case_insensitive)
            .multi_line(true)
            .dot_matches_new_line(multiline)
            .build()
            .map_err(|e| ToolError::ExecutionFailed {
                message: format!("Invalid regex pattern '{}': {}", pattern_str, e),
            })?;

        // If path is a file, search just that file
        if search_path.is_file() {
            let results = search_file(&search_path, &re, output_mode, show_line_numbers, before_context, after_context)?;
            let output = format_results(&results, output_mode, head_limit, offset);
            return Ok(ToolResult::text(&output));
        }

        if !search_path.is_dir() {
            return Ok(ToolResult::error(&format!(
                "Path not found: {}",
                search_path.display()
            )));
        }

        // Walk directory
        let mut all_results: Vec<FileSearchResult> = Vec::new();
        let walker = WalkDir::new(&search_path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                if e.file_type().is_dir() {
                    let name = e.file_name().to_string_lossy();
                    !SKIP_DIRS.contains(&name.as_ref())
                } else {
                    true
                }
            });

        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();

            // Apply glob/type filter
            if !glob_matchers.is_empty() && !glob_matchers.iter().any(|m| m.is_match(path) || m.is_match(path.file_name().unwrap_or_default())) {
                continue;
            }

            match search_file(path, &re, output_mode, show_line_numbers, before_context, after_context) {
                Ok(result) if !result.is_empty() => all_results.push(result),
                _ => continue,
            }

            // Early exit if we have way more results than needed
            if output_mode == "files_with_matches" && all_results.len() > offset + head_limit + 100 {
                break;
            }
        }

        let output = format_all_results(&all_results, output_mode, head_limit, offset);

        if output.is_empty() {
            return Ok(ToolResult::text("No matches found."));
        }

        Ok(ToolResult::text(&output))
    }
}

fn build_glob_matchers(
    glob_pattern: Option<&str>,
    type_filter: Option<&str>,
) -> Result<Vec<GlobMatcher>, ToolError> {
    let mut matchers = Vec::new();

    if let Some(pattern) = glob_pattern {
        let g = Glob::new(pattern).map_err(|e| ToolError::ExecutionFailed {
            message: format!("Invalid glob pattern '{}': {}", pattern, e),
        })?;
        matchers.push(g.compile_matcher());
    }

    if let Some(ty) = type_filter {
        if let Some(globs) = type_to_globs(ty) {
            for g_str in globs {
                let g = Glob::new(g_str).map_err(|e| ToolError::ExecutionFailed {
                    message: format!("Invalid type glob '{}': {}", g_str, e),
                })?;
                matchers.push(g.compile_matcher());
            }
        } else {
            return Err(ToolError::ExecutionFailed {
                message: format!("Unknown file type: '{}'. Common types: js, py, rust, go, java, ts, cpp, rb, php.", ty),
            });
        }
    }

    Ok(matchers)
}

#[derive(Debug)]
struct FileSearchResult {
    path: PathBuf,
    /// For content mode: formatted lines to display
    lines: Vec<String>,
    /// For count mode
    match_count: usize,
}

impl FileSearchResult {
    fn is_empty(&self) -> bool {
        self.match_count == 0
    }
}

fn search_file(
    path: &Path,
    re: &Regex,
    output_mode: &str,
    show_line_numbers: bool,
    before_context: usize,
    after_context: usize,
) -> Result<FileSearchResult, ()> {
    // Read file, skip binary
    let bytes = fs::read(path).map_err(|_| ())?;
    if is_binary_content(&bytes) {
        return Err(());
    }
    let content = String::from_utf8_lossy(&bytes);

    let file_lines: Vec<&str> = content.lines().collect();
    let total_lines = file_lines.len();

    match output_mode {
        "content" => {
            let mut output_lines: Vec<String> = Vec::new();
            let mut last_printed_line: Option<usize> = None;

            for (line_idx, line) in file_lines.iter().enumerate() {
                if re.is_match(line) {
                    let start = line_idx.saturating_sub(before_context);
                    let end = (line_idx + after_context + 1).min(total_lines);

                    // Add separator if there's a gap
                    if let Some(last) = last_printed_line {
                        if start > last + 1 {
                            output_lines.push("--".to_string());
                        }
                    }

                    for ctx_idx in start..end {
                        if let Some(last) = last_printed_line {
                            if ctx_idx <= last {
                                continue;
                            }
                        }
                        let prefix = if show_line_numbers {
                            if ctx_idx == line_idx {
                                format!("{}:", ctx_idx + 1)
                            } else {
                                format!("{}-", ctx_idx + 1)
                            }
                        } else if ctx_idx == line_idx {
                            String::new()
                        } else {
                            String::new()
                        };
                        output_lines.push(format!("{}{}", prefix, file_lines[ctx_idx]));
                        last_printed_line = Some(ctx_idx);
                    }
                }
            }

            let match_count = file_lines.iter().filter(|l| re.is_match(l)).count();

            Ok(FileSearchResult {
                path: path.to_path_buf(),
                lines: output_lines,
                match_count,
            })
        }
        "count" => {
            let match_count = file_lines.iter().filter(|l| re.is_match(l)).count();
            Ok(FileSearchResult {
                path: path.to_path_buf(),
                lines: vec![],
                match_count,
            })
        }
        _ => {
            // files_with_matches
            let has_match = file_lines.iter().any(|l| re.is_match(l));
            Ok(FileSearchResult {
                path: path.to_path_buf(),
                lines: vec![],
                match_count: if has_match { 1 } else { 0 },
            })
        }
    }
}

fn format_results(result: &FileSearchResult, output_mode: &str, head_limit: usize, offset: usize) -> String {
    match output_mode {
        "content" => {
            result.lines.iter().skip(offset).take(head_limit).cloned().collect::<Vec<_>>().join("\n")
        }
        "count" => {
            format!("{}:{}", result.path.display(), result.match_count)
        }
        _ => {
            if result.match_count > 0 {
                result.path.display().to_string()
            } else {
                String::new()
            }
        }
    }
}

fn format_all_results(
    results: &[FileSearchResult],
    output_mode: &str,
    head_limit: usize,
    offset: usize,
) -> String {
    match output_mode {
        "content" => {
            let mut output = Vec::new();
            let mut entry_count = 0usize;
            let mut skipped = 0usize;

            for result in results {
                if result.lines.is_empty() {
                    continue;
                }
                if skipped < offset {
                    skipped += 1;
                    continue;
                }
                if entry_count >= head_limit {
                    break;
                }
                if !output.is_empty() {
                    output.push(String::new());
                }
                output.push(format!("{}:", result.path.display()));
                for line in &result.lines {
                    output.push(line.clone());
                }
                entry_count += 1;
            }
            output.join("\n")
        }
        "count" => {
            results
                .iter()
                .filter(|r| r.match_count > 0)
                .skip(offset)
                .take(head_limit)
                .map(|r| format!("{}:{}", r.path.display(), r.match_count))
                .collect::<Vec<_>>()
                .join("\n")
        }
        _ => {
            // files_with_matches
            results
                .iter()
                .filter(|r| r.match_count > 0)
                .skip(offset)
                .take(head_limit)
                .map(|r| r.path.display().to_string())
                .collect::<Vec<_>>()
                .join("\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_name_and_schema() {
        let tool = GrepTool::new();
        assert_eq!(tool.name(), "Grep");
        let schema = tool.input_schema();
        assert!(schema["properties"]["pattern"].is_object());
        assert!(schema["properties"]["output_mode"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("pattern")));
    }

    #[test]
    fn test_is_read_only() {
        let tool = GrepTool::new();
        assert!(tool.is_read_only(&json!({})));
    }

    #[test]
    fn test_validate_input() {
        let tool = GrepTool::new();
        assert!(matches!(
            tool.validate_input(&json!({"pattern": "foo"})),
            ValidationResult::Ok
        ));
        assert!(matches!(
            tool.validate_input(&json!({})),
            ValidationResult::Error { .. }
        ));
    }

    #[tokio::test]
    async fn test_grep_finds_matches() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.txt"), "hello world\ngoodbye world\n").unwrap();
        fs::write(dir.path().join("b.txt"), "no match here\n").unwrap();

        let tool = GrepTool::new();
        let result = tool
            .call(json!({
                "pattern": "hello",
                "path": dir.path().to_str().unwrap(),
                "output_mode": "files_with_matches"
            }))
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.content.as_str().unwrap();
        assert!(text.contains("a.txt"));
        assert!(!text.contains("b.txt"));
    }

    #[tokio::test]
    async fn test_grep_content_mode() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("test.txt"), "line1\nhello world\nline3\n").unwrap();

        let tool = GrepTool::new();
        let result = tool
            .call(json!({
                "pattern": "hello",
                "path": dir.path().to_str().unwrap(),
                "output_mode": "content",
                "-n": true
            }))
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.content.as_str().unwrap();
        assert!(text.contains("2:hello world"));
    }

    #[tokio::test]
    async fn test_grep_count_mode() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("test.txt"),
            "foo\nbar\nfoo again\nbaz\nfoo third\n",
        )
        .unwrap();

        let tool = GrepTool::new();
        let result = tool
            .call(json!({
                "pattern": "foo",
                "path": dir.path().to_str().unwrap(),
                "output_mode": "count"
            }))
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.content.as_str().unwrap();
        assert!(text.contains(":3"));
    }

    #[tokio::test]
    async fn test_grep_case_insensitive() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("test.txt"), "Hello World\n").unwrap();

        let tool = GrepTool::new();
        let result = tool
            .call(json!({
                "pattern": "hello",
                "path": dir.path().to_str().unwrap(),
                "-i": true,
                "output_mode": "files_with_matches"
            }))
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.content.as_str().unwrap();
        assert!(text.contains("test.txt"));
    }

    #[tokio::test]
    async fn test_grep_no_matches() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("test.txt"), "nothing here\n").unwrap();

        let tool = GrepTool::new();
        let result = tool
            .call(json!({
                "pattern": "zzz_no_match",
                "path": dir.path().to_str().unwrap()
            }))
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.content.as_str().unwrap();
        assert!(text.contains("No matches"));
    }

    #[tokio::test]
    async fn test_grep_with_context() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("test.txt"),
            "line1\nline2\nmatch_here\nline4\nline5\n",
        )
        .unwrap();

        let tool = GrepTool::new();
        let result = tool
            .call(json!({
                "pattern": "match_here",
                "path": dir.path().to_str().unwrap(),
                "output_mode": "content",
                "-C": 1
            }))
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.content.as_str().unwrap();
        assert!(text.contains("line2"));
        assert!(text.contains("match_here"));
        assert!(text.contains("line4"));
    }

    #[tokio::test]
    async fn test_grep_glob_filter() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.rs"), "fn main() {}\n").unwrap();
        fs::write(dir.path().join("b.txt"), "fn main() {}\n").unwrap();

        let tool = GrepTool::new();
        let result = tool
            .call(json!({
                "pattern": "fn main",
                "path": dir.path().to_str().unwrap(),
                "glob": "*.rs",
                "output_mode": "files_with_matches"
            }))
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.content.as_str().unwrap();
        assert!(text.contains("a.rs"));
        assert!(!text.contains("b.txt"));
    }
}
