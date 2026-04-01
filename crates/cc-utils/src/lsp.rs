use serde::{Deserialize, Serialize};

/// LSP position in a file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// LSP range within a file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// LSP location (file + range)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

/// LSP symbol information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: String,
    pub location: Location,
}

/// Hover result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    async fn hover(&self, _file: &str, _pos: Position) -> Result<Option<HoverResult>, LspError> {
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
    fn test_position_serialization() {
        let pos = Position {
            line: 10,
            character: 5,
        };
        let json = serde_json::to_string(&pos).unwrap();
        let deserialized: Position = serde_json::from_str(&json).unwrap();
        assert_eq!(pos, deserialized);
    }

    #[test]
    fn test_location_serialization() {
        let loc = Location {
            uri: "file:///src/main.rs".to_string(),
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 10,
                },
            },
        };
        let json = serde_json::to_string(&loc).unwrap();
        assert!(json.contains("file:///src/main.rs"));
        let deserialized: Location = serde_json::from_str(&json).unwrap();
        assert_eq!(loc, deserialized);
    }

    #[test]
    fn test_symbol_info_serialization() {
        let sym = SymbolInfo {
            name: "my_function".to_string(),
            kind: "Function".to_string(),
            location: Location {
                uri: "file:///src/lib.rs".to_string(),
                range: Range {
                    start: Position {
                        line: 5,
                        character: 0,
                    },
                    end: Position {
                        line: 5,
                        character: 20,
                    },
                },
            },
        };
        let json = serde_json::to_string(&sym).unwrap();
        let deserialized: SymbolInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(sym, deserialized);
    }

    #[test]
    fn test_hover_result_with_range() {
        let hover = HoverResult {
            contents: "fn foo() -> i32".to_string(),
            range: Some(Range {
                start: Position {
                    line: 1,
                    character: 0,
                },
                end: Position {
                    line: 1,
                    character: 3,
                },
            }),
        };
        let json = serde_json::to_string(&hover).unwrap();
        let deserialized: HoverResult = serde_json::from_str(&json).unwrap();
        assert_eq!(hover, deserialized);
    }

    #[test]
    fn test_hover_result_without_range() {
        let hover = HoverResult {
            contents: "documentation text".to_string(),
            range: None,
        };
        let json = serde_json::to_string(&hover).unwrap();
        let deserialized: HoverResult = serde_json::from_str(&json).unwrap();
        assert_eq!(hover, deserialized);
        assert!(deserialized.range.is_none());
    }

    #[tokio::test]
    async fn test_stub_lsp_client_returns_not_available() {
        let client = StubLspClient;
        let pos = Position {
            line: 0,
            character: 0,
        };
        let result = client.go_to_definition("test.rs", pos.clone()).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LspError::NotAvailable));

        let result = client.find_references("test.rs", pos.clone()).await;
        assert!(result.is_err());

        let result = client.hover("test.rs", pos).await;
        assert!(result.is_err());

        let result = client.document_symbols("test.rs").await;
        assert!(result.is_err());

        let result = client.workspace_symbols("query").await;
        assert!(result.is_err());
    }
}
