use serde::{Deserialize, Serialize};

use crate::content::ContentBlock;

/// A content block list used within messages.
pub type MessageContent = Vec<ContentBlock>;

/// Top-level message type, tagged by `"type"` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    #[serde(rename = "user")]
    User(UserMessage),
    #[serde(rename = "assistant")]
    Assistant(AssistantMessage),
    #[serde(rename = "system")]
    System(SystemMessage),
    #[serde(rename = "result")]
    Result(ToolResultMessage),
    #[serde(rename = "progress")]
    Progress(ProgressMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub uuid: String,
    pub message: MessageContent,
    pub tool_use_result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub uuid: String,
    pub message: MessageContent,
    pub model: String,
    pub cost_usd: f64,
    pub duration_ms: u64,
    pub stop_reason: Option<StopReason>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMessage {
    pub uuid: String,
    pub message: String,
    pub system_message_type: SystemMessageType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultMessage {
    pub uuid: String,
    pub tool_use_id: String,
    pub content: serde_json::Value,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressMessage {
    pub uuid: String,
    pub tool_use_id: String,
    pub content: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    StopSequence,
    ToolUse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemMessageType {
    Error,
    Warning,
    Info,
    CompactBoundary,
    Tombstone,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::ContentBlock;

    #[test]
    fn user_message_roundtrip() {
        let msg = Message::User(UserMessage {
            uuid: "u1".to_string(),
            message: vec![ContentBlock::Text {
                text: "Hello".to_string(),
            }],
            tool_use_result: None,
        });
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"user\""));
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        match deserialized {
            Message::User(u) => {
                assert_eq!(u.uuid, "u1");
                assert_eq!(u.message.len(), 1);
            }
            _ => panic!("expected User"),
        }
    }

    #[test]
    fn assistant_message_roundtrip() {
        let msg = Message::Assistant(AssistantMessage {
            uuid: "a1".to_string(),
            message: vec![ContentBlock::Text {
                text: "Hi there".to_string(),
            }],
            model: "claude-opus-4-20250514".to_string(),
            cost_usd: 0.005,
            duration_ms: 1200,
            stop_reason: Some(StopReason::EndTurn),
        });
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        match deserialized {
            Message::Assistant(a) => {
                assert_eq!(a.model, "claude-opus-4-20250514");
                assert_eq!(a.stop_reason, Some(StopReason::EndTurn));
            }
            _ => panic!("expected Assistant"),
        }
    }

    #[test]
    fn system_message_roundtrip() {
        let msg = Message::System(SystemMessage {
            uuid: "s1".to_string(),
            message: "Something went wrong".to_string(),
            system_message_type: SystemMessageType::Error,
        });
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        match deserialized {
            Message::System(s) => {
                assert_eq!(s.system_message_type, SystemMessageType::Error);
            }
            _ => panic!("expected System"),
        }
    }

    #[test]
    fn tool_result_message_roundtrip() {
        let msg = Message::Result(ToolResultMessage {
            uuid: "r1".to_string(),
            tool_use_id: "tu_1".to_string(),
            content: serde_json::json!({"output": "done"}),
            is_error: false,
        });
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        match deserialized {
            Message::Result(r) => {
                assert!(!r.is_error);
                assert_eq!(r.tool_use_id, "tu_1");
            }
            _ => panic!("expected Result"),
        }
    }

    #[test]
    fn progress_message_roundtrip() {
        let msg = Message::Progress(ProgressMessage {
            uuid: "p1".to_string(),
            tool_use_id: "tu_2".to_string(),
            content: serde_json::json!({"progress": 50}),
        });
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        match deserialized {
            Message::Progress(p) => {
                assert_eq!(p.tool_use_id, "tu_2");
            }
            _ => panic!("expected Progress"),
        }
    }

    #[test]
    fn stop_reason_variants() {
        for reason in [
            StopReason::EndTurn,
            StopReason::MaxTokens,
            StopReason::StopSequence,
            StopReason::ToolUse,
        ] {
            let json = serde_json::to_string(&reason).unwrap();
            let back: StopReason = serde_json::from_str(&json).unwrap();
            assert_eq!(reason, back);
        }
    }

    #[test]
    fn system_message_type_variants() {
        for smt in [
            SystemMessageType::Error,
            SystemMessageType::Warning,
            SystemMessageType::Info,
            SystemMessageType::CompactBoundary,
            SystemMessageType::Tombstone,
        ] {
            let json = serde_json::to_string(&smt).unwrap();
            let back: SystemMessageType = serde_json::from_str(&json).unwrap();
            assert_eq!(smt, back);
        }
    }
}
