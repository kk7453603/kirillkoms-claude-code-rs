use std::sync::Mutex;

use tokio::sync::mpsc;

/// A request from the engine asking the user for tool-use permission.
#[derive(Debug)]
pub struct PermissionRequest {
    pub tool_name: String,
    pub message: String,
    pub input: serde_json::Value,
    pub response_tx: tokio::sync::oneshot::Sender<bool>,
}

/// Channel pair for passing permission requests from the engine to the TUI.
pub struct PermissionChannel {
    pub request_rx: Mutex<mpsc::Receiver<PermissionRequest>>,
    request_tx: mpsc::Sender<PermissionRequest>,
}

impl Default for PermissionChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionChannel {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(4);
        Self {
            request_rx: Mutex::new(rx),
            request_tx: tx,
        }
    }

    pub fn sender(&self) -> TuiPermissionCallback {
        TuiPermissionCallback {
            request_tx: self.request_tx.clone(),
        }
    }
}

/// PermissionCallback implementation that sends requests through a channel
/// to the TUI event loop, where the user can approve/deny them.
#[derive(Clone)]
pub struct TuiPermissionCallback {
    request_tx: mpsc::Sender<PermissionRequest>,
}

#[async_trait::async_trait]
impl cc_engine::tool_execution::PermissionCallback for TuiPermissionCallback {
    async fn ask_permission(
        &self,
        tool_name: &str,
        message: &str,
        input: &serde_json::Value,
    ) -> bool {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        let request = PermissionRequest {
            tool_name: tool_name.to_string(),
            message: message.to_string(),
            input: input.clone(),
            response_tx,
        };

        // Send request to TUI event loop
        if self.request_tx.send(request).await.is_err() {
            // Channel closed — deny by default
            return false;
        }

        // Wait for user response
        response_rx.await.unwrap_or(false)
    }
}

/// Extract a short summary of a tool input for display in the permission dialog.
pub fn summarize_input(tool_name: &str, input: &serde_json::Value) -> String {
    match tool_name {
        "Bash" => input
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "Read" => input
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "Write" => input
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "Edit" => input
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        _ => {
            let s = input.to_string();
            if s.len() > 80 {
                format!("{}...", &s[..80])
            } else {
                s
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summarize_input_bash() {
        let input = serde_json::json!({"command": "ls -la"});
        assert_eq!(summarize_input("Bash", &input), "ls -la");
    }

    #[test]
    fn test_summarize_input_read() {
        let input = serde_json::json!({"file_path": "/tmp/test.rs"});
        assert_eq!(summarize_input("Read", &input), "/tmp/test.rs");
    }

    #[test]
    fn test_summarize_input_unknown() {
        let input = serde_json::json!({"foo": "bar"});
        let summary = summarize_input("Unknown", &input);
        assert!(summary.contains("foo"));
    }

    #[test]
    fn test_permission_channel_creation() {
        let channel = PermissionChannel::new();
        let _sender = channel.sender();
    }
}
