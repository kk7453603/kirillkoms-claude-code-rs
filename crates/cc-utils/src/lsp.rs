use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

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

// ---------------------------------------------------------------------------
// Language detection & default server mapping
// ---------------------------------------------------------------------------

/// Detect programming language from a file extension.
pub fn detect_language(file_path: &str) -> &str {
    match file_path.rsplit('.').next() {
        Some("rs") => "rust",
        Some("ts" | "tsx") => "typescript",
        Some("js" | "jsx") => "javascript",
        Some("py") => "python",
        Some("go") => "go",
        _ => "plaintext",
    }
}

/// Return the default LSP server command and arguments for a given language.
pub fn default_server_for_language(lang: &str) -> Option<(&str, &[&str])> {
    match lang {
        "rust" => Some(("rust-analyzer", &[])),
        "typescript" | "javascript" => Some(("typescript-language-server", &["--stdio"])),
        "python" => Some(("pylsp", &[])),
        "go" => Some(("gopls", &["serve"])),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// LSP symbol kind mapping
// ---------------------------------------------------------------------------

fn symbol_kind_name(kind: u64) -> &'static str {
    match kind {
        1 => "File",
        2 => "Module",
        3 => "Namespace",
        4 => "Package",
        5 => "Class",
        6 => "Method",
        7 => "Property",
        8 => "Field",
        9 => "Constructor",
        10 => "Enum",
        11 => "Interface",
        12 => "Function",
        13 => "Variable",
        14 => "Constant",
        15 => "String",
        16 => "Number",
        17 => "Boolean",
        18 => "Array",
        19 => "Object",
        20 => "Key",
        21 => "Null",
        22 => "EnumMember",
        23 => "Struct",
        24 => "Event",
        25 => "Operator",
        26 => "TypeParameter",
        _ => "Unknown",
    }
}

// ---------------------------------------------------------------------------
// JSON-RPC types (LSP)
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
struct LspJsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

#[derive(serde::Serialize)]
struct LspJsonRpcNotification {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
}

#[derive(serde::Deserialize)]
struct LspJsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<u64>,
    result: Option<serde_json::Value>,
    error: Option<LspJsonRpcError>,
}

#[derive(serde::Deserialize)]
struct LspJsonRpcError {
    code: i64,
    message: String,
}

// ---------------------------------------------------------------------------
// StdioLspClient
// ---------------------------------------------------------------------------

struct LspClientInner {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<BufReader<ChildStdout>>,
    next_id: u64,
    initialized: bool,
    opened_files: HashSet<String>,
}

/// An LSP client that communicates with a language server over stdio using
/// the Content-Length framing protocol.
pub struct StdioLspClient {
    server_command: String,
    server_args: Vec<String>,
    root_uri: String,
    inner: Mutex<LspClientInner>,
}

impl StdioLspClient {
    /// Create a new `StdioLspClient`. Call [`connect`](Self::connect) before
    /// issuing any requests.
    pub fn new(server_command: String, server_args: Vec<String>, root_uri: String) -> Self {
        Self {
            server_command,
            server_args,
            root_uri,
            inner: Mutex::new(LspClientInner {
                child: None,
                stdin: None,
                stdout: None,
                next_id: 1,
                initialized: false,
                opened_files: HashSet::new(),
            }),
        }
    }

    /// Spawn the language server process and perform the LSP initialize
    /// handshake (initialize request + initialized notification).
    pub async fn connect(&self) -> Result<(), LspError> {
        let mut inner = self.inner.lock().await;

        let mut child = Command::new(&self.server_command)
            .args(&self.server_args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| LspError::Protocol(format!("Failed to spawn server: {}", e)))?;

        inner.stdin = child.stdin.take();
        inner.stdout = child.stdout.take().map(BufReader::new);
        inner.child = Some(child);

        // Send initialize request
        let init_params = serde_json::json!({
            "processId": std::process::id(),
            "capabilities": {
                "textDocument": {
                    "definition": { "dynamicRegistration": false },
                    "references": { "dynamicRegistration": false },
                    "hover": { "dynamicRegistration": false },
                    "documentSymbol": { "dynamicRegistration": false }
                },
                "workspace": {
                    "symbol": { "dynamicRegistration": false }
                }
            },
            "rootUri": self.root_uri,
            "rootPath": self.root_uri.strip_prefix("file://").unwrap_or(&self.root_uri),
        });

        Self::send_request_inner(&mut inner, "initialize", init_params).await?;

        // Send initialized notification
        Self::send_notification_inner(&mut inner, "initialized", serde_json::json!({})).await?;

        inner.initialized = true;
        Ok(())
    }

    // -- low-level framing ------------------------------------------------

    /// Write a Content-Length framed message to the given writer.
    async fn write_message(
        stdin: &mut ChildStdin,
        body: &[u8],
    ) -> Result<(), LspError> {
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        stdin
            .write_all(header.as_bytes())
            .await
            .map_err(|e| LspError::Protocol(format!("write header: {}", e)))?;
        stdin
            .write_all(body)
            .await
            .map_err(|e| LspError::Protocol(format!("write body: {}", e)))?;
        stdin
            .flush()
            .await
            .map_err(|e| LspError::Protocol(format!("flush: {}", e)))?;
        Ok(())
    }

    /// Read one Content-Length framed message from the given reader.
    async fn read_message(
        stdout: &mut BufReader<ChildStdout>,
    ) -> Result<String, LspError> {
        // Read headers until we find the blank line (\r\n\r\n).
        let mut content_length: Option<usize> = None;
        loop {
            let mut line = String::new();
            let n = stdout
                .read_line(&mut line)
                .await
                .map_err(|e| LspError::Protocol(format!("read header line: {}", e)))?;
            if n == 0 {
                return Err(LspError::Protocol("Server closed connection".into()));
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                // End of headers
                break;
            }
            if let Some(val) = trimmed.strip_prefix("Content-Length:") {
                content_length = Some(
                    val.trim()
                        .parse::<usize>()
                        .map_err(|e| LspError::Protocol(format!("bad Content-Length: {}", e)))?,
                );
            }
            // Ignore other headers (e.g. Content-Type)
        }

        let len = content_length
            .ok_or_else(|| LspError::Protocol("Missing Content-Length header".into()))?;

        let mut buf = vec![0u8; len];
        stdout
            .read_exact(&mut buf)
            .await
            .map_err(|e| LspError::Protocol(format!("read body: {}", e)))?;

        String::from_utf8(buf)
            .map_err(|e| LspError::Protocol(format!("invalid UTF-8 in body: {}", e)))
    }

    // -- JSON-RPC helpers -------------------------------------------------

    async fn send_request_inner(
        inner: &mut LspClientInner,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, LspError> {
        let id = inner.next_id;
        inner.next_id += 1;

        let request = LspJsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        let body = serde_json::to_vec(&request)
            .map_err(|e| LspError::Protocol(format!("serialize request: {}", e)))?;

        let stdin = inner
            .stdin
            .as_mut()
            .ok_or_else(|| LspError::Protocol("Not connected".into()))?;
        Self::write_message(stdin, &body).await?;

        let stdout = inner
            .stdout
            .as_mut()
            .ok_or_else(|| LspError::Protocol("Not connected".into()))?;

        // Read responses, skipping server-initiated notifications (no id).
        loop {
            let msg = Self::read_message(stdout).await?;
            let resp: LspJsonRpcResponse = serde_json::from_str(&msg)
                .map_err(|e| LspError::Protocol(format!("parse response: {}", e)))?;

            // Skip notifications (messages without an id)
            if resp.id.is_none() {
                continue;
            }

            if let Some(err) = resp.error {
                return Err(LspError::Protocol(format!(
                    "{}: {}",
                    err.code, err.message
                )));
            }

            return Ok(resp.result.unwrap_or(serde_json::Value::Null));
        }
    }

    async fn send_notification_inner(
        inner: &mut LspClientInner,
        method: &str,
        params: serde_json::Value,
    ) -> Result<(), LspError> {
        let notification = LspJsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
        };

        let body = serde_json::to_vec(&notification)
            .map_err(|e| LspError::Protocol(format!("serialize notification: {}", e)))?;

        let stdin = inner
            .stdin
            .as_mut()
            .ok_or_else(|| LspError::Protocol("Not connected".into()))?;
        Self::write_message(stdin, &body).await?;
        Ok(())
    }

    async fn send_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, LspError> {
        let mut inner = self.inner.lock().await;
        if !inner.initialized {
            return Err(LspError::Protocol("Client not initialized".into()));
        }
        Self::send_request_inner(&mut inner, method, params).await
    }

    #[allow(dead_code)]
    async fn send_notification(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<(), LspError> {
        let mut inner = self.inner.lock().await;
        if !inner.initialized {
            return Err(LspError::Protocol("Client not initialized".into()));
        }
        Self::send_notification_inner(&mut inner, method, params).await
    }

    /// Ensure we have sent `textDocument/didOpen` for the given file path.
    async fn ensure_file_open(&self, file_path: &str) -> Result<(), LspError> {
        let mut inner = self.inner.lock().await;
        if !inner.initialized {
            return Err(LspError::Protocol("Client not initialized".into()));
        }
        let uri = path_to_uri(file_path);
        if inner.opened_files.contains(&uri) {
            return Ok(());
        }

        let text = std::fs::read_to_string(file_path).unwrap_or_default();
        let lang = detect_language(file_path);

        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
                "languageId": lang,
                "version": 1,
                "text": text,
            }
        });

        Self::send_notification_inner(&mut inner, "textDocument/didOpen", params).await?;
        inner.opened_files.insert(uri);
        Ok(())
    }

    /// Helper: build a `TextDocumentPositionParams` JSON value.
    fn text_document_position(file_path: &str, pos: &Position) -> serde_json::Value {
        serde_json::json!({
            "textDocument": { "uri": path_to_uri(file_path) },
            "position": { "line": pos.line, "character": pos.character }
        })
    }
}

// ---------------------------------------------------------------------------
// LspClient trait implementation
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
impl LspClient for StdioLspClient {
    async fn go_to_definition(
        &self,
        file: &str,
        pos: Position,
    ) -> Result<Vec<Location>, LspError> {
        self.ensure_file_open(file).await?;
        let params = Self::text_document_position(file, &pos);
        let result = self.send_request("textDocument/definition", params).await?;
        Ok(parse_locations(&result))
    }

    async fn find_references(
        &self,
        file: &str,
        pos: Position,
    ) -> Result<Vec<Location>, LspError> {
        self.ensure_file_open(file).await?;
        let mut params = Self::text_document_position(file, &pos);
        // References requires a context.includeDeclaration field.
        params.as_object_mut().unwrap().insert(
            "context".to_string(),
            serde_json::json!({ "includeDeclaration": true }),
        );
        let result = self.send_request("textDocument/references", params).await?;
        Ok(parse_locations(&result))
    }

    async fn hover(
        &self,
        file: &str,
        pos: Position,
    ) -> Result<Option<HoverResult>, LspError> {
        self.ensure_file_open(file).await?;
        let params = Self::text_document_position(file, &pos);
        let result = self.send_request("textDocument/hover", params).await?;

        if result.is_null() {
            return Ok(None);
        }

        let contents = extract_hover_contents(&result);
        let range = result.get("range").and_then(|r| parse_range(r));

        Ok(Some(HoverResult { contents, range }))
    }

    async fn document_symbols(&self, file: &str) -> Result<Vec<SymbolInfo>, LspError> {
        self.ensure_file_open(file).await?;
        let params = serde_json::json!({
            "textDocument": { "uri": path_to_uri(file) }
        });
        let result = self
            .send_request("textDocument/documentSymbol", params)
            .await?;
        Ok(parse_symbols(&result, file))
    }

    async fn workspace_symbols(&self, query: &str) -> Result<Vec<SymbolInfo>, LspError> {
        let params = serde_json::json!({ "query": query });
        let result = self.send_request("workspace/symbol", params).await?;
        Ok(parse_symbols(&result, ""))
    }
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

fn path_to_uri(path: &str) -> String {
    if path.starts_with("file://") {
        path.to_string()
    } else {
        format!("file://{}", path)
    }
}

fn parse_range(v: &serde_json::Value) -> Option<Range> {
    let start = v.get("start")?;
    let end = v.get("end")?;
    Some(Range {
        start: Position {
            line: start.get("line")?.as_u64()? as u32,
            character: start.get("character")?.as_u64()? as u32,
        },
        end: Position {
            line: end.get("line")?.as_u64()? as u32,
            character: end.get("character")?.as_u64()? as u32,
        },
    })
}

fn parse_location(v: &serde_json::Value) -> Option<Location> {
    let uri = v.get("uri")?.as_str()?.to_string();
    let range = parse_range(v.get("range")?)?;
    Some(Location { uri, range })
}

/// Parse an LSP definition / references result which may be a single Location,
/// an array of Locations, or null.
fn parse_locations(v: &serde_json::Value) -> Vec<Location> {
    if v.is_null() {
        return Vec::new();
    }
    if let Some(arr) = v.as_array() {
        arr.iter().filter_map(parse_location).collect()
    } else if let Some(loc) = parse_location(v) {
        vec![loc]
    } else {
        Vec::new()
    }
}

fn extract_hover_contents(v: &serde_json::Value) -> String {
    let contents = match v.get("contents") {
        Some(c) => c,
        None => return String::new(),
    };

    // MarkedString (plain string)
    if let Some(s) = contents.as_str() {
        return s.to_string();
    }

    // MarkupContent { kind, value }
    if let Some(val) = contents.get("value").and_then(|v| v.as_str()) {
        return val.to_string();
    }

    // Array of MarkedString
    if let Some(arr) = contents.as_array() {
        let parts: Vec<String> = arr
            .iter()
            .filter_map(|item| {
                if let Some(s) = item.as_str() {
                    Some(s.to_string())
                } else {
                    item.get("value").and_then(|v| v.as_str()).map(String::from)
                }
            })
            .collect();
        return parts.join("\n");
    }

    String::new()
}

/// Parse document symbols or workspace symbols. Handles both `SymbolInformation[]`
/// (with a `location` field) and `DocumentSymbol[]` (with a `range` field, no
/// `location`).
fn parse_symbols(v: &serde_json::Value, default_file: &str) -> Vec<SymbolInfo> {
    let arr = match v.as_array() {
        Some(a) => a,
        None => return Vec::new(),
    };

    let mut out = Vec::new();
    for item in arr {
        let name = match item.get("name").and_then(|n| n.as_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        let kind_num = item.get("kind").and_then(|k| k.as_u64()).unwrap_or(0);
        let kind = symbol_kind_name(kind_num).to_string();

        // SymbolInformation style (workspace/symbol)
        if let Some(loc) = item.get("location").and_then(parse_location) {
            out.push(SymbolInfo {
                name,
                kind,
                location: loc,
            });
            continue;
        }

        // DocumentSymbol style (textDocument/documentSymbol) – has `range` but
        // no `location`.
        if let Some(range) = item.get("range").and_then(parse_range) {
            let uri = path_to_uri(default_file);
            out.push(SymbolInfo {
                name: name.clone(),
                kind: kind.clone(),
                location: Location { uri, range },
            });
        }

        // Recurse into children (DocumentSymbol can be nested)
        if let Some(children) = item.get("children").and_then(|c| c.as_array()) {
            let child_val = serde_json::Value::Array(children.clone());
            out.extend(parse_symbols(&child_val, default_file));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
        assert!(json.contains("my_function"));
        assert!(json.contains("Function"));
    }

    #[test]
    fn hover_result_with_and_without_range() {
        let hover = HoverResult {
            contents: "fn foo() -> i32".to_string(),
            range: Some(Range {
                start: Position {
                    line: 1,
                    character: 4,
                },
                end: Position {
                    line: 1,
                    character: 7,
                },
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
        let pos = Position {
            line: 0,
            character: 0,
        };

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

    // -- Content-Length framing tests -------------------------------------

    #[tokio::test]
    async fn test_write_message_framing() {
        // We test write_message by writing to a child process's stdin and
        // verifying the bytes via a pipe. We use a simple approach: spawn
        // cat, write a framed message, read back from stdout.
        let mut child = Command::new("cat")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();

        let mut stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout);

        let body = b"{\"jsonrpc\":\"2.0\",\"id\":1}";
        StdioLspClient::write_message(&mut stdin, body)
            .await
            .unwrap();
        // Close stdin so cat writes everything out
        drop(stdin);

        let mut output = String::new();
        reader
            .read_to_string(&mut output)
            .await
            .unwrap();

        let expected = format!(
            "Content-Length: {}\r\n\r\n{}",
            body.len(),
            std::str::from_utf8(body).unwrap()
        );
        assert_eq!(output, expected);
    }

    #[tokio::test]
    async fn test_read_message_framing() {
        // Spawn a process that writes a valid Content-Length framed message to
        // its stdout, then read it back with read_message.
        let body = r#"{"jsonrpc":"2.0","id":1,"result":null}"#;
        let frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);

        let mut child = Command::new("sh")
            .arg("-c")
            .arg(format!("printf '{}'", frame))
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();

        let stdout = child.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout);

        let msg = StdioLspClient::read_message(&mut reader).await.unwrap();
        assert_eq!(msg, body);
    }

    #[tokio::test]
    async fn test_read_message_missing_content_length() {
        let frame = "SomeHeader: value\r\n\r\n{}";

        let mut child = Command::new("sh")
            .arg("-c")
            .arg(format!("printf '{}'", frame))
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();

        let stdout = child.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout);

        let result = StdioLspClient::read_message(&mut reader).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Content-Length") || err_msg.contains("read body"));
    }

    // -- Language detection tests -----------------------------------------

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language("main.rs"), "rust");
        assert_eq!(detect_language("/home/user/project/lib.rs"), "rust");
        assert_eq!(detect_language("app.ts"), "typescript");
        assert_eq!(detect_language("component.tsx"), "typescript");
        assert_eq!(detect_language("index.js"), "javascript");
        assert_eq!(detect_language("widget.jsx"), "javascript");
        assert_eq!(detect_language("script.py"), "python");
        assert_eq!(detect_language("main.go"), "go");
        assert_eq!(detect_language("readme.md"), "plaintext");
        assert_eq!(detect_language("noext"), "plaintext");
    }

    #[test]
    fn test_default_server_for_language() {
        let (cmd, args) = default_server_for_language("rust").unwrap();
        assert_eq!(cmd, "rust-analyzer");
        assert!(args.is_empty());

        let (cmd, args) = default_server_for_language("typescript").unwrap();
        assert_eq!(cmd, "typescript-language-server");
        assert_eq!(args, &["--stdio"]);

        let (cmd, args) = default_server_for_language("javascript").unwrap();
        assert_eq!(cmd, "typescript-language-server");
        assert_eq!(args, &["--stdio"]);

        let (cmd, _) = default_server_for_language("python").unwrap();
        assert_eq!(cmd, "pylsp");

        let (cmd, args) = default_server_for_language("go").unwrap();
        assert_eq!(cmd, "gopls");
        assert_eq!(args, &["serve"]);

        assert!(default_server_for_language("plaintext").is_none());
        assert!(default_server_for_language("unknown").is_none());
    }

    // -- Parsing helpers tests --------------------------------------------

    #[test]
    fn test_path_to_uri() {
        assert_eq!(path_to_uri("/home/user/file.rs"), "file:///home/user/file.rs");
        assert_eq!(
            path_to_uri("file:///already/uri"),
            "file:///already/uri"
        );
    }

    #[test]
    fn test_parse_locations_null() {
        let v = serde_json::Value::Null;
        assert!(parse_locations(&v).is_empty());
    }

    #[test]
    fn test_parse_locations_single() {
        let v = serde_json::json!({
            "uri": "file:///foo.rs",
            "range": {
                "start": { "line": 1, "character": 2 },
                "end": { "line": 3, "character": 4 }
            }
        });
        let locs = parse_locations(&v);
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].uri, "file:///foo.rs");
        assert_eq!(locs[0].range.start.line, 1);
    }

    #[test]
    fn test_parse_locations_array() {
        let v = serde_json::json!([
            {
                "uri": "file:///a.rs",
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 0, "character": 5 }
                }
            },
            {
                "uri": "file:///b.rs",
                "range": {
                    "start": { "line": 10, "character": 0 },
                    "end": { "line": 10, "character": 5 }
                }
            }
        ]);
        let locs = parse_locations(&v);
        assert_eq!(locs.len(), 2);
        assert_eq!(locs[1].uri, "file:///b.rs");
    }

    #[test]
    fn test_extract_hover_contents_string() {
        let v = serde_json::json!({ "contents": "hello world" });
        assert_eq!(extract_hover_contents(&v), "hello world");
    }

    #[test]
    fn test_extract_hover_contents_markup() {
        let v = serde_json::json!({
            "contents": { "kind": "markdown", "value": "## Title" }
        });
        assert_eq!(extract_hover_contents(&v), "## Title");
    }

    #[test]
    fn test_extract_hover_contents_array() {
        let v = serde_json::json!({
            "contents": [
                "first",
                { "language": "rust", "value": "fn foo()" }
            ]
        });
        assert_eq!(extract_hover_contents(&v), "first\nfn foo()");
    }

    #[test]
    fn test_extract_hover_contents_missing() {
        let v = serde_json::json!({});
        assert_eq!(extract_hover_contents(&v), "");
    }

    #[test]
    fn test_parse_symbols_symbol_information() {
        let v = serde_json::json!([
            {
                "name": "MyStruct",
                "kind": 23,
                "location": {
                    "uri": "file:///lib.rs",
                    "range": {
                        "start": { "line": 0, "character": 0 },
                        "end": { "line": 5, "character": 1 }
                    }
                }
            }
        ]);
        let syms = parse_symbols(&v, "");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].name, "MyStruct");
        assert_eq!(syms[0].kind, "Struct");
    }

    #[test]
    fn test_parse_symbols_document_symbol() {
        let v = serde_json::json!([
            {
                "name": "main",
                "kind": 12,
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 10, "character": 1 }
                },
                "children": [
                    {
                        "name": "x",
                        "kind": 13,
                        "range": {
                            "start": { "line": 1, "character": 4 },
                            "end": { "line": 1, "character": 10 }
                        }
                    }
                ]
            }
        ]);
        let syms = parse_symbols(&v, "/src/main.rs");
        assert_eq!(syms.len(), 2);
        assert_eq!(syms[0].name, "main");
        assert_eq!(syms[0].kind, "Function");
        assert_eq!(syms[1].name, "x");
        assert_eq!(syms[1].kind, "Variable");
    }

    #[test]
    fn test_symbol_kind_name() {
        assert_eq!(symbol_kind_name(1), "File");
        assert_eq!(symbol_kind_name(5), "Class");
        assert_eq!(symbol_kind_name(12), "Function");
        assert_eq!(symbol_kind_name(23), "Struct");
        assert_eq!(symbol_kind_name(999), "Unknown");
    }

    #[tokio::test]
    async fn test_stdio_lsp_client_connect_invalid_command() {
        let client = StdioLspClient::new(
            "/nonexistent/lsp-server".to_string(),
            vec![],
            "file:///tmp".to_string(),
        );
        let result = client.connect().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stdio_lsp_client_send_request_not_initialized() {
        let client = StdioLspClient::new(
            "echo".to_string(),
            vec![],
            "file:///tmp".to_string(),
        );
        let result = client
            .send_request("textDocument/definition", serde_json::json!({}))
            .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not initialized"));
    }
}
