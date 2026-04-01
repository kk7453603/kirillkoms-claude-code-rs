use serde::{Deserialize, Serialize};

/// LSP position in a file
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// LSP range (start + end positions)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// LSP location (file + range)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

/// LSP symbol information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: String,
    pub location: Location,
}

/// Hover result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoverResult {
    pub contents: String,
    pub range: Option<Range>,
}

/// LSP client interface
#[async_trait::async_trait]
pub trait LspClient: Send + Sync {
    async fn go_to_definition(&self, file: &str, pos: Position) -> Result<Vec<Location>, LspError>;
    async fn find_references(&self, file: &str, pos: Position) -> Result<Vec<Location>, LspError>;
    async fn hover(&self, file: &str, pos: Position) -> Result<Option<HoverResult>, LspError>;
    async fn document_symbols(&self, file: &str) -> Result<Vec<SymbolInfo>, LspError>;
    async fn workspace_symbols(&self, query: &str) -> Result<Vec<SymbolInfo>, LspError>;
}

#[derive(Debug, thiserror::Error)]
pub enum LspError {
    #[error("LSP not available")]
    NotAvailable,
    #[error("LSP error: {0}")]
    Protocol(String),
    #[error("Timeout")]
    Timeout,
}

/// Stub LSP client (until real LSP integration)
pub struct StubLspClient;

#[async_trait::async_trait]
impl LspClient for StubLspClient {
    async fn go_to_definition(
        &self,
        _file: &str,
        _pos: Position,
    ) -> Result<Vec<Location>, LspError> {
        Err(LspError::NotAvailable)
    }
    async fn find_references(
        &self,
        _file: &str,
        _pos: Position,
    ) -> Result<Vec<Location>, LspError> {
        Err(LspError::NotAvailable)
    }
    async fn hover(
        &self,
        _file: &str,
        _pos: Position,
    ) -> Result<Option<HoverResult>, LspError> {
        Err(LspError::NotAvailable)
    }
    async fn document_symbols(&self, _file: &str) -> Result<Vec<SymbolInfo>, LspError> {
        Err(LspError::NotAvailable)
    }
    async fn workspace_symbols(&self, _query: &str) -> Result<Vec<SymbolInfo>, LspError> {
        Err(LspError::NotAvailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_serialization_roundtrip() {
        let pos = Position {
            line: 10,
            character: 5,
        };
        let json = serde_json::to_string(&pos).unwrap();
        let back: Position = serde_json::from_str(&json).unwrap();
        assert_eq!(back.line, 10);
        assert_eq!(back.character, 5);
    }

    #[test]
    fn location_serialization_roundtrip() {
        let loc = Location {
            uri: "file:///src/main.rs".to_string(),
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 10 },
            },
        };
        let json = serde_json::to_string(&loc).unwrap();
        let back: Location = serde_json::from_str(&json).unwrap();
        assert_eq!(back.uri, "file:///src/main.rs");
        assert_eq!(back.range.start.line, 0);
        assert_eq!(back.range.end.character, 10);
    }

    #[test]
    fn symbol_info_serialization() {
        let sym = SymbolInfo {
            name: "my_function".to_string(),
            kind: "Function".to_string(),
            location: Location {
                uri: "file:///lib.rs".to_string(),
                range: Range {
                    start: Position { line: 5, character: 0 },
                    end: Position { line: 5, character: 20 },
                },
            },
        };
        let json = serde_json::to_string(&sym).unwrap();
        assert!(json.contains("my_function"));
        assert!(json.contains("Function"));
    }

    #[test]
    fn hover_result_with_and_without_range() {
        let hover = HoverResult {
            contents: "fn foo() -> i32".to_string(),
            range: Some(Range {
                start: Position { line: 1, character: 4 },
                end: Position { line: 1, character: 7 },
            }),
        };
        let json = serde_json::to_string(&hover).unwrap();
        let back: HoverResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.contents, "fn foo() -> i32");
        assert!(back.range.is_some());

        let hover_no_range = HoverResult {
            contents: "docs".to_string(),
            range: None,
        };
        let json2 = serde_json::to_string(&hover_no_range).unwrap();
        let back2: HoverResult = serde_json::from_str(&json2).unwrap();
        assert!(back2.range.is_none());
    }

    #[tokio::test]
    async fn stub_client_returns_not_available() {
        let client = StubLspClient;
        let pos = Position { line: 0, character: 0 };

        let result = client.go_to_definition("file.rs", pos.clone()).await;
        assert!(matches!(result, Err(LspError::NotAvailable)));

        let result = client.find_references("file.rs", pos.clone()).await;
        assert!(matches!(result, Err(LspError::NotAvailable)));

        let result = client.hover("file.rs", pos).await;
        assert!(matches!(result, Err(LspError::NotAvailable)));

        let result = client.document_symbols("file.rs").await;
        assert!(matches!(result, Err(LspError::NotAvailable)));

        let result = client.workspace_symbols("query").await;
        assert!(matches!(result, Err(LspError::NotAvailable)));
    }
}
