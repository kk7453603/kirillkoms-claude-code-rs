use chrono::Utc;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AnalyticsEvent {
    pub event_name: String,
    pub timestamp: String,
    pub session_id: Option<String>,
    pub properties: serde_json::Value,
}

impl AnalyticsEvent {
    pub fn new(name: &str, properties: serde_json::Value) -> Self {
        Self {
            event_name: name.to_string(),
            timestamp: Utc::now().to_rfc3339(),
            session_id: None,
            properties,
        }
    }

    pub fn with_session(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self
    }
}

/// Analytics collector (no-op by default, can be enabled).
pub struct AnalyticsCollector {
    enabled: bool,
    events: Vec<AnalyticsEvent>,
}

impl AnalyticsCollector {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            events: vec![],
        }
    }

    pub fn track(&mut self, event: AnalyticsEvent) {
        if self.enabled {
            self.events.push(event);
        }
    }

    pub fn events(&self) -> &[AnalyticsEvent] {
        &self.events
    }

    pub fn flush(&mut self) -> Vec<AnalyticsEvent> {
        std::mem::take(&mut self.events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = AnalyticsEvent::new("test_event", serde_json::json!({"key": "value"}));
        assert_eq!(event.event_name, "test_event");
        assert!(event.session_id.is_none());
        assert!(!event.timestamp.is_empty());
        assert_eq!(event.properties["key"], "value");
    }

    #[test]
    fn test_event_with_session() {
        let event = AnalyticsEvent::new("evt", serde_json::json!({})).with_session("session-123");
        assert_eq!(event.session_id, Some("session-123".to_string()));
    }

    #[test]
    fn test_collector_disabled() {
        let mut collector = AnalyticsCollector::new(false);
        collector.track(AnalyticsEvent::new("evt", serde_json::json!({})));
        assert!(collector.events().is_empty());
    }

    #[test]
    fn test_collector_enabled() {
        let mut collector = AnalyticsCollector::new(true);
        collector.track(AnalyticsEvent::new("evt1", serde_json::json!({})));
        collector.track(AnalyticsEvent::new("evt2", serde_json::json!({})));
        assert_eq!(collector.events().len(), 2);
        assert_eq!(collector.events()[0].event_name, "evt1");
    }

    #[test]
    fn test_collector_flush() {
        let mut collector = AnalyticsCollector::new(true);
        collector.track(AnalyticsEvent::new("evt", serde_json::json!({})));
        let flushed = collector.flush();
        assert_eq!(flushed.len(), 1);
        assert!(collector.events().is_empty());
    }

    #[test]
    fn test_event_serialization() {
        let event =
            AnalyticsEvent::new("test", serde_json::json!({"count": 42})).with_session("s1");
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"event_name\":\"test\""));
        assert!(json.contains("\"session_id\":\"s1\""));
        assert!(json.contains("\"count\":42"));
    }

    #[test]
    fn test_collector_multiple_flush() {
        let mut collector = AnalyticsCollector::new(true);
        collector.track(AnalyticsEvent::new("a", serde_json::json!({})));
        let first = collector.flush();
        assert_eq!(first.len(), 1);

        collector.track(AnalyticsEvent::new("b", serde_json::json!({})));
        let second = collector.flush();
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].event_name, "b");
    }
}
