use serde::{Deserialize, Serialize};
use std::fmt;

/// A unique session identifier, wrapping a UUID-based string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    /// Generate a new UUID-based session ID.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Wrap an existing string as a session ID.
    pub fn from_string(s: String) -> Self {
        Self(s)
    }

    /// Borrow the inner string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An agent identifier following the pattern `a<optional-label->16hex`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(String);

impl AgentId {
    /// Generate a new agent ID with an optional label.
    /// Pattern: `a<label->16hex` or `a16hex` if no label.
    pub fn new(label: Option<&str>) -> Self {
        use rand::Rng;
        let mut rng = rand::rng();
        let hex: u64 = rng.random();
        let hex_str = format!("{:016x}", hex);
        let id = match label {
            Some(l) => format!("a{}-{}", l, hex_str),
            None => format!("a{}", hex_str),
        };
        Self(id)
    }

    /// Parse and validate a string as an AgentId.
    /// Must match: `^a(?:.+-)?[0-9a-f]{16}$`
    pub fn parse(s: &str) -> Option<Self> {
        let re = regex::Regex::new(r"^a(?:.+-)?[0-9a-f]{16}$").unwrap();
        if re.is_match(s) {
            Some(Self(s.to_string()))
        } else {
            None
        }
    }

    /// Borrow the inner string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_id_new_generates_uuid() {
        let id = SessionId::new();
        // UUID v4 format: 8-4-4-4-12 hex chars
        assert_eq!(id.as_str().len(), 36);
        assert!(id.as_str().contains('-'));
    }

    #[test]
    fn session_id_from_string() {
        let id = SessionId::from_string("my-session-123".to_string());
        assert_eq!(id.as_str(), "my-session-123");
    }

    #[test]
    fn session_id_display() {
        let id = SessionId::from_string("test-id".to_string());
        assert_eq!(format!("{}", id), "test-id");
    }

    #[test]
    fn session_id_serde_roundtrip() {
        let id = SessionId::from_string("sess-abc".to_string());
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: SessionId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn session_id_equality_and_hash() {
        use std::collections::HashSet;
        let a = SessionId::from_string("x".to_string());
        let b = SessionId::from_string("x".to_string());
        let c = SessionId::from_string("y".to_string());
        assert_eq!(a, b);
        assert_ne!(a, c);
        let mut set = HashSet::new();
        set.insert(a.clone());
        assert!(set.contains(&b));
    }

    #[test]
    fn agent_id_new_no_label() {
        let id = AgentId::new(None);
        let s = id.as_str();
        assert!(s.starts_with('a'));
        // 'a' + 16 hex chars = 17
        assert_eq!(s.len(), 17);
        assert!(AgentId::parse(s).is_some());
    }

    #[test]
    fn agent_id_new_with_label() {
        let id = AgentId::new(Some("test"));
        let s = id.as_str();
        assert!(s.starts_with("atest-"));
        assert!(AgentId::parse(s).is_some());
    }

    #[test]
    fn agent_id_parse_valid() {
        assert!(AgentId::parse("a0123456789abcdef").is_some());
        assert!(AgentId::parse("amylabel-0123456789abcdef").is_some());
        assert!(AgentId::parse("aa-b-c-0123456789abcdef").is_some());
    }

    #[test]
    fn agent_id_parse_invalid() {
        assert!(AgentId::parse("").is_none());
        assert!(AgentId::parse("b0123456789abcdef").is_none()); // wrong prefix
        assert!(AgentId::parse("a012345").is_none()); // too short hex
        assert!(AgentId::parse("a0123456789ABCDEF").is_none()); // uppercase hex
        assert!(AgentId::parse("a0123456789abcdefg").is_none()); // too long
    }

    #[test]
    fn agent_id_display() {
        let id = AgentId::parse("a0123456789abcdef").unwrap();
        assert_eq!(format!("{}", id), "a0123456789abcdef");
    }

    #[test]
    fn agent_id_serde_roundtrip() {
        let id = AgentId::new(Some("serde"));
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: AgentId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }
}
