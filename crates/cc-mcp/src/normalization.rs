/// Normalize an MCP tool name to the format: mcp__<server>__<tool>
pub fn normalize_tool_name(server_name: &str, tool_name: &str) -> String {
    format!("mcp__{}__{}", server_name, tool_name)
}

/// Parse a normalized MCP tool name back to (server, tool)
pub fn parse_tool_name(normalized: &str) -> Option<(String, String)> {
    let stripped = normalized.strip_prefix("mcp__")?;
    let idx = stripped.find("__")?;
    let server = &stripped[..idx];
    let tool = &stripped[idx + 2..];
    if server.is_empty() || tool.is_empty() {
        return None;
    }
    Some((server.to_string(), tool.to_string()))
}

/// Check if a tool name is an MCP tool
pub fn is_mcp_tool(name: &str) -> bool {
    name.starts_with("mcp__") && parse_tool_name(name).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_tool_name() {
        assert_eq!(
            normalize_tool_name("github", "create_pr"),
            "mcp__github__create_pr"
        );
    }

    #[test]
    fn test_parse_tool_name() {
        let (server, tool) = parse_tool_name("mcp__github__create_pr").unwrap();
        assert_eq!(server, "github");
        assert_eq!(tool, "create_pr");
    }

    #[test]
    fn test_roundtrip() {
        let server = "my_server";
        let tool = "my_tool";
        let normalized = normalize_tool_name(server, tool);
        let (s, t) = parse_tool_name(&normalized).unwrap();
        assert_eq!(s, server);
        assert_eq!(t, tool);
    }

    #[test]
    fn test_parse_invalid() {
        assert!(parse_tool_name("not_mcp").is_none());
        assert!(parse_tool_name("mcp__").is_none());
        assert!(parse_tool_name("mcp____tool").is_none());
        assert!(parse_tool_name("mcp__server__").is_none());
    }

    #[test]
    fn test_is_mcp_tool() {
        assert!(is_mcp_tool("mcp__github__create_pr"));
        assert!(!is_mcp_tool("read_file"));
        assert!(!is_mcp_tool("mcp__"));
        assert!(!is_mcp_tool("mcp__server__"));
    }

    #[test]
    fn test_tool_with_double_underscores_in_name() {
        // Tool name itself may contain underscores
        let normalized = normalize_tool_name("srv", "do__thing");
        let (s, t) = parse_tool_name(&normalized).unwrap();
        assert_eq!(s, "srv");
        assert_eq!(t, "do__thing");
    }
}
