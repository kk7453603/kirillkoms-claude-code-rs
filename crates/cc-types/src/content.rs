use serde::{Deserialize, Serialize};

/// A block of content within a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { source: ImageSource },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: ToolResultContent,
        is_error: Option<bool>,
    },
    #[serde(rename = "thinking")]
    Thinking {
        thinking: String,
        signature: Option<String>,
    },
}

/// Source data for an image content block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    pub media_type: String,
    pub data: String,
    #[serde(rename = "type")]
    pub source_type: String,
}

/// Content within a tool result: either plain text or nested content blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolResultContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_block_roundtrip() {
        let block = ContentBlock::Text {
            text: "Hello, world!".to_string(),
        };
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        let deserialized: ContentBlock = serde_json::from_str(&json).unwrap();
        match deserialized {
            ContentBlock::Text { text } => assert_eq!(text, "Hello, world!"),
            _ => panic!("expected Text variant"),
        }
    }

    #[test]
    fn image_block_roundtrip() {
        let block = ContentBlock::Image {
            source: ImageSource {
                media_type: "image/png".to_string(),
                data: "iVBORw0KGgo=".to_string(),
                source_type: "base64".to_string(),
            },
        };
        let json = serde_json::to_string(&block).unwrap();
        let deserialized: ContentBlock = serde_json::from_str(&json).unwrap();
        match deserialized {
            ContentBlock::Image { source } => {
                assert_eq!(source.media_type, "image/png");
                assert_eq!(source.source_type, "base64");
            }
            _ => panic!("expected Image variant"),
        }
    }

    #[test]
    fn tool_use_block_roundtrip() {
        let block = ContentBlock::ToolUse {
            id: "tu_123".to_string(),
            name: "read_file".to_string(),
            input: serde_json::json!({"path": "/tmp/file.txt"}),
        };
        let json = serde_json::to_string(&block).unwrap();
        let deserialized: ContentBlock = serde_json::from_str(&json).unwrap();
        match deserialized {
            ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "tu_123");
                assert_eq!(name, "read_file");
                assert_eq!(input["path"], "/tmp/file.txt");
            }
            _ => panic!("expected ToolUse variant"),
        }
    }

    #[test]
    fn tool_result_block_with_text_content() {
        let block = ContentBlock::ToolResult {
            tool_use_id: "tu_123".to_string(),
            content: ToolResultContent::Text("file contents here".to_string()),
            is_error: Some(false),
        };
        let json = serde_json::to_string(&block).unwrap();
        let deserialized: ContentBlock = serde_json::from_str(&json).unwrap();
        match deserialized {
            ContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
            } => {
                assert_eq!(tool_use_id, "tu_123");
                assert_eq!(is_error, Some(false));
                match content {
                    ToolResultContent::Text(t) => assert_eq!(t, "file contents here"),
                    _ => panic!("expected Text content"),
                }
            }
            _ => panic!("expected ToolResult variant"),
        }
    }

    #[test]
    fn tool_result_block_with_blocks_content() {
        let block = ContentBlock::ToolResult {
            tool_use_id: "tu_456".to_string(),
            content: ToolResultContent::Blocks(vec![ContentBlock::Text {
                text: "nested".to_string(),
            }]),
            is_error: None,
        };
        let json = serde_json::to_string(&block).unwrap();
        let deserialized: ContentBlock = serde_json::from_str(&json).unwrap();
        match deserialized {
            ContentBlock::ToolResult { content, .. } => match content {
                ToolResultContent::Blocks(blocks) => {
                    assert_eq!(blocks.len(), 1);
                }
                _ => panic!("expected Blocks content"),
            },
            _ => panic!("expected ToolResult variant"),
        }
    }

    #[test]
    fn thinking_block_roundtrip() {
        let block = ContentBlock::Thinking {
            thinking: "Let me consider...".to_string(),
            signature: Some("sig123".to_string()),
        };
        let json = serde_json::to_string(&block).unwrap();
        let deserialized: ContentBlock = serde_json::from_str(&json).unwrap();
        match deserialized {
            ContentBlock::Thinking {
                thinking,
                signature,
            } => {
                assert_eq!(thinking, "Let me consider...");
                assert_eq!(signature, Some("sig123".to_string()));
            }
            _ => panic!("expected Thinking variant"),
        }
    }
}
