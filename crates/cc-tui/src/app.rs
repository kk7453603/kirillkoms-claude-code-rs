use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::input::TextInput;
use crate::progress::Spinner;
use crate::themes::Theme;

// ─── Data types ────────────────────────────────────────────────────

/// Roles for chat messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// A single content block inside a ChatMessage.
#[derive(Debug, Clone)]
pub enum ContentBlock {
    Text(String),
    Thinking(String),
    ToolUse {
        name: String,
        input_summary: String,
        result: Option<ToolResultInfo>,
        collapsed: bool,
    },
}

/// Outcome of a tool execution.
#[derive(Debug, Clone)]
pub struct ToolResultInfo {
    pub summary: String,
    pub is_error: bool,
    /// For Edit tool: file path that was modified.
    pub file_path: Option<String>,
}

/// One message in the conversation history.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub blocks: Vec<ContentBlock>,
    pub timestamp: Instant,
}

/// A tool that is currently executing (spinner is shown).
#[derive(Debug, Clone)]
pub struct ActiveTool {
    pub id: String,
    pub name: String,
    pub started_at: Instant,
    /// Short summary of the key argument (command, file_path, pattern, etc.).
    pub input_summary: String,
}

/// Session metadata shown in the banner.
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub model: String,
    pub cwd: String,
    pub git_branch: Option<String>,
    pub session_id: String,
    pub version: String,
}

impl Default for SessionInfo {
    fn default() -> Self {
        Self {
            model: String::new(),
            cwd: String::new(),
            git_branch: None,
            session_id: String::new(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Token / cost counters for the status bar.
#[derive(Debug, Clone, Default)]
pub struct UsageInfo {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
    pub turn_count: u32,
}

/// Smart scroll state with auto-follow and smooth animation.
#[derive(Debug, Clone)]
pub struct ScrollState {
    /// Current rendered offset (animated toward target).
    pub offset: usize,
    /// Target offset (where the user wants to scroll to).
    pub target: usize,
    pub auto_follow: bool,
    pub total_lines: usize,
    pub viewport_height: usize,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self {
            offset: 0,
            target: 0,
            auto_follow: true,
            total_lines: 0,
            viewport_height: 0,
        }
    }
}

impl ScrollState {
    /// Scroll up by N lines. Offset changes instantly — no animation delay.
    pub fn scroll_up(&mut self, lines: usize) {
        let max_offset = self.total_lines.saturating_sub(self.viewport_height);
        self.offset = (self.offset + lines).min(max_offset);
        self.target = self.offset;
        self.auto_follow = false;
    }

    /// Scroll down by N lines. Offset changes instantly.
    pub fn scroll_down(&mut self, lines: usize) {
        self.offset = self.offset.saturating_sub(lines);
        self.target = self.offset;
        if self.offset == 0 {
            self.auto_follow = true;
        }
    }

    pub fn follow_if_needed(&mut self) {
        if self.auto_follow {
            self.target = 0;
            self.offset = 0;
        }
    }

    pub fn set_content_size(&mut self, total_lines: usize, viewport_height: usize) {
        self.total_lines = total_lines;
        self.viewport_height = viewport_height;
        let max_offset = total_lines.saturating_sub(viewport_height);
        if self.target > max_offset {
            self.target = max_offset;
        }
        if self.offset > max_offset {
            self.offset = max_offset;
        }
    }
}

// ─── App modes ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Input,
    Scrolling,
    PermissionPrompt,
}

// ─── App actions ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppAction {
    Continue,
    Submit(String),
    Quit,
    /// User responded to a permission prompt.
    /// bool = allowed, Option<bool> = Some(true) means "always allow this tool"
    PermissionResponse(bool),
    PermissionAlwaysAllow,
}

// ─── Core App state ────────────────────────────────────────────────

pub struct App {
    pub mode: AppMode,
    pub input: TextInput,
    pub theme: Theme,
    pub should_quit: bool,

    // Conversation
    pub messages: Vec<ChatMessage>,
    pub streaming_text: String,
    pub streaming_thinking: String,
    pub active_tool: Option<ActiveTool>,
    pub thinking: bool,

    // UI state
    pub spinner: Spinner,
    pub session_info: SessionInfo,
    pub usage: UsageInfo,
    pub scroll: ScrollState,

    // Permission
    pub pending_permission: Option<PendingPermission>,

    // Slash-command completion
    pub command_completions: Vec<String>,
    pub completion_labels: Vec<String>,
    pub completion_index: Option<usize>,

    // Input history
    pub input_history: Vec<String>,
    pub history_index: Option<usize>,
    /// Stash current input when navigating history.
    history_stash: String,
}

/// A permission request waiting for user input.
#[derive(Debug, Clone)]
pub struct PendingPermission {
    pub tool_name: String,
    pub message: String,
    pub input_summary: String,
}

impl App {
    pub fn new() -> Self {
        Self {
            mode: AppMode::Input,
            input: TextInput::new(),
            theme: Theme::default(),
            should_quit: false,
            messages: Vec::new(),
            streaming_text: String::new(),
            streaming_thinking: String::new(),
            active_tool: None,
            thinking: false,
            spinner: Spinner::new(""),
            session_info: SessionInfo::default(),
            usage: UsageInfo::default(),
            scroll: ScrollState::default(),
            pending_permission: None,
            command_completions: Vec::new(),
            completion_labels: Vec::new(),
            completion_index: None,
            input_history: Vec::new(),
            history_index: None,
            history_stash: String::new(),
        }
    }

    /// Tick animations (spinner).
    pub fn tick(&mut self) {
        self.spinner.tick();
    }

    // ─── QueryEvent handlers ───────────────────────────────────────

    /// Append streaming text from the assistant.
    pub fn on_text_delta(&mut self, text: &str) {
        self.thinking = false;
        self.streaming_text.push_str(text);
        self.scroll.follow_if_needed();
    }

    /// Append streaming thinking content.
    pub fn on_thinking_delta(&mut self, text: &str) {
        self.thinking = true;
        self.streaming_thinking.push_str(text);
    }

    /// A tool execution has started.
    pub fn on_tool_use_start(&mut self, id: &str, name: &str, input: &serde_json::Value) {
        // Flush any pending streaming text into a message block
        self.flush_streaming_text();

        let input_summary = extract_input_summary(name, input);
        self.active_tool = Some(ActiveTool {
            id: id.to_string(),
            name: name.to_string(),
            started_at: Instant::now(),
            input_summary: input_summary.clone(),
        });
        if input_summary.is_empty() {
            self.spinner.set_message(&format!("Running {}...", name));
        } else {
            self.spinner
                .set_message(&format!("Running {} {}...", name, input_summary));
        }
        self.scroll.follow_if_needed();
    }

    /// A tool execution has finished.
    pub fn on_tool_result(&mut self, _id: &str, result: &serde_json::Value, is_error: bool) {
        let summary = summarize_tool_result(result);

        if let Some(tool) = self.active_tool.take() {
            // Use input_summary from the ActiveTool (set during on_tool_use_start)
            let input_summary = tool.input_summary.clone();

            // Derive file_path for file-based tools from the input_summary we already have
            let file_path = if matches!(tool.name.as_str(), "Edit" | "Write" | "Read")
                && !input_summary.is_empty()
            {
                Some(input_summary.clone())
            } else {
                None
            };

            self.ensure_assistant_message();
            if let Some(msg) = self.messages.last_mut() {
                msg.blocks.push(ContentBlock::ToolUse {
                    name: tool.name,
                    input_summary,
                    result: Some(ToolResultInfo {
                        summary,
                        is_error,
                        file_path,
                    }),
                    collapsed: true,
                });
            }
        }
        self.scroll.follow_if_needed();
    }

    /// Turn completed — finalize message.
    pub fn on_turn_complete(&mut self, _stop_reason: &str) {
        self.flush_streaming_text();
        self.flush_streaming_thinking();
        self.thinking = false;
        self.active_tool = None;
        self.usage.turn_count += 1;
    }

    /// Usage update from the API.
    pub fn on_usage_update(&mut self, input_tokens: u64, output_tokens: u64) {
        self.usage.input_tokens += input_tokens;
        self.usage.output_tokens += output_tokens;
        // Recalculate cost
        self.usage.cost_usd = cc_cost::model_costs::calculate_cost(
            &self.session_info.model,
            self.usage.input_tokens,
            self.usage.output_tokens,
            0,
            0,
        );
    }

    /// An error occurred.
    pub fn on_error(&mut self, error: &str) {
        self.flush_streaming_text();
        self.active_tool = None;
        self.thinking = false;

        self.messages.push(ChatMessage {
            role: MessageRole::System,
            blocks: vec![ContentBlock::Text(format!("Error: {}", error))],
            timestamp: Instant::now(),
        });
    }

    /// Add a system info message (displayed as-is, no markdown rendering).
    pub fn add_system_info(&mut self, text: &str) {
        self.messages.push(ChatMessage {
            role: MessageRole::System,
            blocks: vec![ContentBlock::Text(text.to_string())],
            timestamp: Instant::now(),
        });
        self.scroll.auto_follow = true;
        self.scroll.offset = 0;
    }

    /// Add a user message to the conversation.
    pub fn add_user_message(&mut self, text: &str) {
        self.messages.push(ChatMessage {
            role: MessageRole::User,
            blocks: vec![ContentBlock::Text(text.to_string())],
            timestamp: Instant::now(),
        });
        self.scroll.auto_follow = true;
        self.scroll.offset = 0;
    }

    // ─── Helpers ───────────────────────────────────────────────────

    fn flush_streaming_text(&mut self) {
        if !self.streaming_text.is_empty() {
            let text = std::mem::take(&mut self.streaming_text);
            self.ensure_assistant_message();
            if let Some(msg) = self.messages.last_mut() {
                msg.blocks.push(ContentBlock::Text(text));
            }
        }
    }

    fn flush_streaming_thinking(&mut self) {
        if !self.streaming_thinking.is_empty() {
            let text = std::mem::take(&mut self.streaming_thinking);
            self.ensure_assistant_message();
            if let Some(msg) = self.messages.last_mut() {
                msg.blocks.push(ContentBlock::Thinking(text));
            }
        }
    }

    fn ensure_assistant_message(&mut self) {
        let needs_new = self
            .messages
            .last()
            .map(|m| m.role != MessageRole::Assistant)
            .unwrap_or(true);

        if needs_new {
            self.messages.push(ChatMessage {
                role: MessageRole::Assistant,
                blocks: Vec::new(),
                timestamp: Instant::now(),
            });
        }
    }

    // ─── Key handling ──────────────────────────────────────────────

    pub fn handle_key_event(&mut self, key: KeyEvent) -> AppAction {
        // Ctrl+C always quits
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return AppAction::Quit;
        }

        match self.mode {
            AppMode::Input => self.handle_input_key(key),
            AppMode::Scrolling => self.handle_scroll_key(key),
            AppMode::PermissionPrompt => self.handle_permission_key(key),
        }
    }

    fn handle_input_key(&mut self, key: KeyEvent) -> AppAction {
        match key.code {
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                // Shift+Enter: insert newline
                self.input.insert_newline();
                AppAction::Continue
            }
            KeyCode::Enter => {
                // If completion is selected, accept it
                if let Some(idx) = self.completion_index
                    && idx < self.command_completions.len()
                {
                    let selected = self.command_completions[idx].clone();
                    let content = self.input.content();

                    if content.contains(' ') {
                        // Arg completion: /resume <selected_id> → submit directly
                        let cmd = content.split_once(' ').map(|(c, _)| c).unwrap_or(&content);
                        let full = format!("{} {}", cmd, selected);
                        self.input.clear();
                        self.clear_completions();
                        return AppAction::Submit(full);
                    } else {
                        // Command completion: /res → /resume (stay in input)
                        self.input.clear();
                        for c in format!("/{} ", selected).chars() {
                            self.input.insert_char(c);
                        }
                        self.clear_completions();
                        return AppAction::Continue;
                    }
                }
                let content = self.input.content();
                if content.is_empty() {
                    return AppAction::Continue;
                }
                let content = content.to_string();
                // Save to input history
                self.input_history.push(content.clone());
                self.history_index = None;
                self.input.clear();
                self.clear_completions();
                AppAction::Submit(content)
            }
            KeyCode::Char(c) => {
                self.input.insert_char(c);
                AppAction::Continue
            }
            KeyCode::Backspace => {
                self.input.delete_char();
                AppAction::Continue
            }
            KeyCode::Left => {
                self.input.move_left();
                AppAction::Continue
            }
            KeyCode::Right => {
                self.input.move_right();
                AppAction::Continue
            }
            KeyCode::Up => {
                if !self.command_completions.is_empty() {
                    self.completion_prev();
                } else if self.input.cursor_row() > 0 {
                    self.input.move_up();
                } else if !self.input_history.is_empty() {
                    // Navigate input history
                    self.history_up();
                } else {
                    self.scroll.scroll_up(1);
                }
                AppAction::Continue
            }
            KeyCode::Down => {
                if !self.command_completions.is_empty() {
                    self.completion_next();
                } else if self.input.cursor_row() < self.input.line_count() - 1 {
                    self.input.move_down();
                } else if self.history_index.is_some() {
                    // Navigate input history forward
                    self.history_down();
                } else {
                    self.scroll.scroll_down(1);
                }
                AppAction::Continue
            }
            KeyCode::Tab => {
                if !self.command_completions.is_empty() {
                    self.completion_next();
                }
                AppAction::Continue
            }
            KeyCode::Home => {
                self.input.move_home();
                AppAction::Continue
            }
            KeyCode::End => {
                self.input.move_end();
                AppAction::Continue
            }
            KeyCode::Esc => {
                if !self.command_completions.is_empty() {
                    // Close completions dropdown
                    self.clear_completions();
                } else {
                    self.mode = AppMode::Scrolling;
                }
                AppAction::Continue
            }
            _ => AppAction::Continue,
        }
    }

    fn handle_scroll_key(&mut self, key: KeyEvent) -> AppAction {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll.scroll_up(1);
                AppAction::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll.scroll_down(1);
                AppAction::Continue
            }
            KeyCode::PageUp => {
                self.scroll.scroll_up(self.scroll.viewport_height / 2);
                AppAction::Continue
            }
            KeyCode::PageDown => {
                self.scroll
                    .scroll_down(self.scroll.viewport_height / 2);
                AppAction::Continue
            }
            KeyCode::Char('G') => {
                self.scroll.offset = 0;
                self.scroll.auto_follow = true;
                AppAction::Continue
            }
            KeyCode::Char('e') => {
                self.toggle_last_tool_block();
                AppAction::Continue
            }
            KeyCode::Char('i') | KeyCode::Enter => {
                self.mode = AppMode::Input;
                AppAction::Continue
            }
            KeyCode::Esc | KeyCode::Char('q') => AppAction::Quit,
            _ => AppAction::Continue,
        }
    }

    fn handle_permission_key(&mut self, key: KeyEvent) -> AppAction {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.mode = AppMode::Input;
                self.pending_permission = None;
                AppAction::PermissionResponse(true)
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.mode = AppMode::Input;
                self.pending_permission = None;
                AppAction::PermissionAlwaysAllow
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.mode = AppMode::Input;
                self.pending_permission = None;
                AppAction::PermissionResponse(false)
            }
            _ => AppAction::Continue,
        }
    }

    /// Update slash-command completions based on current input.
    /// Supports command name completion (`/res` → `resume`) and
    /// argument completion (`/resume <partial>` → session IDs).
    /// Update completions. `arg_items` contains (value, label) pairs for arg completion.
    pub fn update_completions(
        &mut self,
        available_commands: &[String],
        arg_items: &[(String, String)],
    ) {
        let content = self.input.content();

        if content.starts_with('/') && !content.contains(' ') {
            // Command name completion
            let prefix = content[1..].to_lowercase();
            let mut new_completions: Vec<String> = available_commands
                .iter()
                .filter(|cmd| cmd.to_lowercase().starts_with(&prefix))
                .cloned()
                .collect();

            if new_completions.is_empty() && !prefix.is_empty() {
                new_completions = available_commands
                    .iter()
                    .filter(|cmd| cmd.to_lowercase().contains(&prefix))
                    .cloned()
                    .collect();
            }

            if new_completions != self.command_completions {
                self.completion_index = None;
            }
            self.completion_labels = new_completions.iter().map(|c| format!("/{}", c)).collect();
            self.command_completions = new_completions;
        } else if content.starts_with('/') && content.contains(' ') && !arg_items.is_empty() {
            // Argument completion
            let arg_prefix = content
                .split_once(' ')
                .map(|(_, rest)| rest.to_lowercase())
                .unwrap_or_default();

            let filtered: Vec<&(String, String)> = arg_items
                .iter()
                .filter(|(val, _)| val.to_lowercase().starts_with(&arg_prefix))
                .collect();

            let new_completions: Vec<String> = filtered.iter().map(|(v, _)| v.clone()).collect();
            if new_completions != self.command_completions {
                self.completion_index = None;
            }
            self.completion_labels = filtered.iter().map(|(_, l)| l.clone()).collect();
            self.command_completions = new_completions;
        } else {
            self.clear_completions();
        }
    }

    /// Clear all completion state.
    fn clear_completions(&mut self) {
        self.command_completions.clear();
        self.completion_labels.clear();
        self.completion_index = None;
    }

    /// Navigate input history backwards (older).
    fn history_up(&mut self) {
        if self.input_history.is_empty() {
            return;
        }
        match self.history_index {
            None => {
                // Stash current input, jump to latest history
                self.history_stash = self.input.content();
                self.history_index = Some(self.input_history.len() - 1);
            }
            Some(0) => return, // Already at oldest
            Some(i) => {
                self.history_index = Some(i - 1);
            }
        }
        self.set_input_from_history();
    }

    /// Navigate input history forwards (newer).
    fn history_down(&mut self) {
        match self.history_index {
            None => return,
            Some(i) if i >= self.input_history.len() - 1 => {
                // Restore stashed input
                self.history_index = None;
                self.input.clear();
                for c in self.history_stash.chars() {
                    self.input.insert_char(c);
                }
                self.history_stash.clear();
            }
            Some(i) => {
                self.history_index = Some(i + 1);
                self.set_input_from_history();
            }
        }
    }

    fn set_input_from_history(&mut self) {
        if let Some(idx) = self.history_index {
            if let Some(text) = self.input_history.get(idx) {
                self.input.clear();
                for c in text.chars() {
                    self.input.insert_char(c);
                }
            }
        }
    }

    /// Move completion selection down.
    pub fn completion_next(&mut self) {
        if self.command_completions.is_empty() {
            return;
        }
        self.completion_index = Some(match self.completion_index {
            Some(i) => (i + 1) % self.command_completions.len(),
            None => 0,
        });
    }

    /// Move completion selection up.
    pub fn completion_prev(&mut self) {
        if self.command_completions.is_empty() {
            return;
        }
        self.completion_index = Some(match self.completion_index {
            Some(0) | None => self.command_completions.len() - 1,
            Some(i) => i - 1,
        });
    }

    /// Toggle the collapsed state of the most recent ToolUse block across all messages.
    pub fn toggle_last_tool_block(&mut self) {
        for msg in self.messages.iter_mut().rev() {
            for block in msg.blocks.iter_mut().rev() {
                if let ContentBlock::ToolUse { collapsed, .. } = block {
                    *collapsed = !*collapsed;
                    return;
                }
            }
        }
    }

    /// Show a permission prompt.
    pub fn show_permission_prompt(&mut self, tool_name: &str, message: &str, input_summary: &str) {
        self.pending_permission = Some(PendingPermission {
            tool_name: tool_name.to_string(),
            message: message.to_string(),
            input_summary: input_summary.to_string(),
        });
        self.mode = AppMode::PermissionPrompt;
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Utility ───────────────────────────────────────────────────────

/// Extract a short, human-readable summary of the key argument from a tool input JSON.
///
/// For Bash: the `command` field.
/// For Read/Edit/Write: the `file_path` field.
/// For Grep/Glob: the `pattern` field.
/// Everything else: first 60 chars of the serialised JSON.
pub fn extract_input_summary(tool_name: &str, input: &serde_json::Value) -> String {
    let summary = match tool_name {
        "Bash" => input
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "Read" | "Edit" | "Write" => input
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "Grep" => input
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "Glob" => input
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        _ => {
            let s = input.to_string();
            if s == "null" || s == "{}" {
                String::new()
            } else if s.len() > 60 {
                format!("{}...", &s[..60])
            } else {
                s
            }
        }
    };
    // Truncate very long values (e.g. huge commands)
    if summary.len() > 80 {
        format!("{}...", &summary[..80])
    } else {
        summary
    }
}

/// Summarize a tool result JSON value into a short string.
/// Produce a short, human-readable summary of a tool result.
/// For file content: "N lines". For short text: first line. For errors: first line.
fn summarize_tool_result(result: &serde_json::Value) -> String {
    let text = match result.as_str() {
        Some(s) => s.trim().to_string(),
        None => {
            let s = result.to_string();
            s.trim_matches('"').to_string()
        }
    };

    if text.is_empty() {
        return "(empty)".to_string();
    }

    let line_count = text.lines().count();

    // Multi-line content (likely file content or command output): show line count + first line
    if line_count > 3 {
        let first = text.lines().next().unwrap_or("");
        let first_trunc = if first.len() > 50 {
            format!("{}...", &first[..50])
        } else {
            first.to_string()
        };
        return format!("{} lines — {}", line_count, first_trunc);
    }

    // Short content: show first line, truncated
    let first = text.lines().next().unwrap_or(&text);
    if first.len() > 60 {
        format!("{}...", &first[..60])
    } else {
        first.to_string()
    }
}

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_new() {
        let app = App::new();
        assert_eq!(app.mode, AppMode::Input);
        assert!(!app.should_quit);
        assert!(app.input.is_empty());
        assert!(app.messages.is_empty());
    }

    #[test]
    fn test_default_app() {
        let app = App::default();
        assert_eq!(app.mode, AppMode::Input);
    }

    #[test]
    fn test_add_user_message() {
        let mut app = App::new();
        app.add_user_message("Hello!");
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].role, MessageRole::User);
    }

    #[test]
    fn test_on_text_delta() {
        let mut app = App::new();
        app.on_text_delta("Hello ");
        app.on_text_delta("world");
        assert_eq!(app.streaming_text, "Hello world");
    }

    #[test]
    fn test_on_turn_complete_flushes_text() {
        let mut app = App::new();
        app.on_text_delta("response text");
        app.on_turn_complete("end_turn");
        assert!(app.streaming_text.is_empty());
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].role, MessageRole::Assistant);
    }

    #[test]
    fn test_on_tool_use_start_flushes_text() {
        let mut app = App::new();
        app.on_text_delta("some text");
        app.on_tool_use_start("id1", "Read", &serde_json::json!({"file_path": "/tmp/foo.rs"}));
        assert!(app.streaming_text.is_empty());
        assert!(app.active_tool.is_some());
        let tool = app.active_tool.as_ref().unwrap();
        assert_eq!(tool.name, "Read");
        assert_eq!(tool.input_summary, "/tmp/foo.rs");
    }

    #[test]
    fn test_on_tool_result() {
        let mut app = App::new();
        app.on_tool_use_start("id1", "Bash", &serde_json::json!({"command": "ls"}));
        app.on_tool_result("id1", &serde_json::json!("ok"), false);
        assert!(app.active_tool.is_none());
        assert_eq!(app.messages.len(), 1);
    }

    #[test]
    fn test_on_error() {
        let mut app = App::new();
        app.on_error("something broke");
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].role, MessageRole::System);
    }

    #[test]
    fn test_handle_enter_submits() {
        let mut app = App::new();
        app.handle_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        let action = app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(action, AppAction::Submit("hi".to_string()));
    }

    #[test]
    fn test_handle_enter_empty_continues() {
        let mut app = App::new();
        let action = app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(action, AppAction::Continue);
    }

    #[test]
    fn test_handle_ctrl_c_quits() {
        let mut app = App::new();
        let action =
            app.handle_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert_eq!(action, AppAction::Quit);
    }

    #[test]
    fn test_esc_switches_to_scroll_mode() {
        let mut app = App::new();
        app.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Scrolling);
    }

    #[test]
    fn test_scroll_mode_i_returns_to_input() {
        let mut app = App::new();
        app.mode = AppMode::Scrolling;
        app.handle_key_event(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Input);
    }

    #[test]
    fn test_permission_prompt_y_accepts() {
        let mut app = App::new();
        app.show_permission_prompt("Bash", "Run command", "ls");
        assert_eq!(app.mode, AppMode::PermissionPrompt);
        let action = app.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        assert_eq!(action, AppAction::PermissionResponse(true));
        assert_eq!(app.mode, AppMode::Input);
    }

    #[test]
    fn test_permission_prompt_n_rejects() {
        let mut app = App::new();
        app.show_permission_prompt("Bash", "Run command", "rm -rf");
        let action = app.handle_key_event(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
        assert_eq!(action, AppAction::PermissionResponse(false));
    }

    #[test]
    fn test_scroll_state_auto_follow() {
        let mut scroll = ScrollState::default();
        assert!(scroll.auto_follow);

        scroll.set_content_size(100, 20);
        scroll.scroll_up(5);
        assert!(!scroll.auto_follow);
        assert_eq!(scroll.offset, 5);

        // Scrolling back to bottom re-enables auto_follow
        scroll.scroll_down(5);
        assert!(scroll.auto_follow);
        assert_eq!(scroll.offset, 0);
    }

    #[test]
    fn test_usage_update() {
        let mut app = App::new();
        app.on_usage_update(100, 50);
        app.on_usage_update(200, 100);
        assert_eq!(app.usage.input_tokens, 300);
        assert_eq!(app.usage.output_tokens, 150);
    }

    #[test]
    fn test_summarize_tool_result_short() {
        let result = serde_json::json!("ok");
        assert_eq!(summarize_tool_result(&result), "ok");
    }

    #[test]
    fn test_summarize_tool_result_long_single_line() {
        let long = "x".repeat(200);
        let result = serde_json::json!(long);
        let summary = summarize_tool_result(&result);
        assert!(summary.ends_with("..."));
        assert!(summary.len() <= 70);
    }

    #[test]
    fn test_summarize_tool_result_multiline() {
        let content = (0..20).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
        let result = serde_json::json!(content);
        let summary = summarize_tool_result(&result);
        assert!(summary.contains("20 lines"));
    }

    #[test]
    fn test_extract_input_summary_bash() {
        let input = serde_json::json!({"command": "cargo test"});
        assert_eq!(extract_input_summary("Bash", &input), "cargo test");
    }

    #[test]
    fn test_extract_input_summary_read() {
        let input = serde_json::json!({"file_path": "/src/main.rs"});
        assert_eq!(extract_input_summary("Read", &input), "/src/main.rs");
    }

    #[test]
    fn test_extract_input_summary_edit() {
        let input = serde_json::json!({"file_path": "/src/lib.rs", "old_string": "foo", "new_string": "bar"});
        assert_eq!(extract_input_summary("Edit", &input), "/src/lib.rs");
    }

    #[test]
    fn test_extract_input_summary_write() {
        let input = serde_json::json!({"file_path": "/out/result.txt", "content": "hello"});
        assert_eq!(extract_input_summary("Write", &input), "/out/result.txt");
    }

    #[test]
    fn test_extract_input_summary_grep() {
        let input = serde_json::json!({"pattern": "fn main"});
        assert_eq!(extract_input_summary("Grep", &input), "fn main");
    }

    #[test]
    fn test_extract_input_summary_glob() {
        let input = serde_json::json!({"pattern": "**/*.rs"});
        assert_eq!(extract_input_summary("Glob", &input), "**/*.rs");
    }

    #[test]
    fn test_extract_input_summary_unknown_empty() {
        // An empty object gives no useful info — function returns "" to avoid noise
        let input = serde_json::json!({});
        assert_eq!(extract_input_summary("UnknownTool", &input), "");
    }

    #[test]
    fn test_extract_input_summary_truncates_long() {
        let long_cmd = "a".repeat(200);
        let input = serde_json::json!({"command": long_cmd});
        let summary = extract_input_summary("Bash", &input);
        assert!(summary.ends_with("..."));
        assert!(summary.len() <= 83); // 80 chars + "..."
    }

    #[test]
    fn test_on_tool_result_carries_input_summary() {
        let mut app = App::new();
        app.on_tool_use_start(
            "id1",
            "Edit",
            &serde_json::json!({"file_path": "/src/foo.rs", "old_string": "x", "new_string": "y"}),
        );
        app.on_tool_result("id1", &serde_json::json!("edited"), false);
        assert_eq!(app.messages.len(), 1);
        match &app.messages[0].blocks[0] {
            ContentBlock::ToolUse {
                input_summary,
                result,
                ..
            } => {
                assert_eq!(input_summary, "/src/foo.rs");
                assert_eq!(result.as_ref().unwrap().file_path.as_deref(), Some("/src/foo.rs"));
            }
            _ => panic!("expected ToolUse block"),
        }
    }

    #[test]
    fn test_toggle_last_tool_block_no_tool() {
        // When there are no tool blocks, toggle_last_tool_block should be a no-op.
        let mut app = App::new();
        app.add_user_message("hello");
        app.toggle_last_tool_block(); // should not panic
    }

    #[test]
    fn test_toggle_last_tool_block_collapses_and_expands() {
        let mut app = App::new();
        app.on_tool_use_start("id1", "Bash", &serde_json::json!({"command": "ls"}));
        app.on_tool_result("id1", &serde_json::json!("file.txt"), false);

        // Block starts collapsed.
        match &app.messages[0].blocks[0] {
            ContentBlock::ToolUse { collapsed, .. } => assert!(*collapsed),
            _ => panic!("expected ToolUse block"),
        }

        // First toggle: expand.
        app.toggle_last_tool_block();
        match &app.messages[0].blocks[0] {
            ContentBlock::ToolUse { collapsed, .. } => assert!(!*collapsed),
            _ => panic!("expected ToolUse block"),
        }

        // Second toggle: collapse again.
        app.toggle_last_tool_block();
        match &app.messages[0].blocks[0] {
            ContentBlock::ToolUse { collapsed, .. } => assert!(*collapsed),
            _ => panic!("expected ToolUse block"),
        }
    }

    #[test]
    fn test_toggle_last_tool_block_targets_most_recent() {
        let mut app = App::new();
        // First tool.
        app.on_tool_use_start("id1", "Read", &serde_json::json!({"file_path": "/a.rs"}));
        app.on_tool_result("id1", &serde_json::json!("content"), false);
        // Second tool in the same assistant message.
        app.on_tool_use_start("id2", "Bash", &serde_json::json!({"command": "pwd"}));
        app.on_tool_result("id2", &serde_json::json!("/home"), false);

        // Toggle should affect the second (most recent) block only.
        app.toggle_last_tool_block();
        let blocks = &app.messages[0].blocks;
        match &blocks[0] {
            ContentBlock::ToolUse { collapsed, .. } => assert!(*collapsed, "first block should remain collapsed"),
            _ => panic!("expected ToolUse"),
        }
        match &blocks[1] {
            ContentBlock::ToolUse { collapsed, .. } => assert!(!*collapsed, "second block should be expanded"),
            _ => panic!("expected ToolUse"),
        }
    }

    #[test]
    fn test_scroll_mode_e_toggles_tool_block() {
        let mut app = App::new();
        app.on_tool_use_start("id1", "Bash", &serde_json::json!({"command": "echo hi"}));
        app.on_tool_result("id1", &serde_json::json!("hi"), false);

        app.mode = AppMode::Scrolling;
        let action = app.handle_key_event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
        assert_eq!(action, AppAction::Continue);
        assert_eq!(app.mode, AppMode::Scrolling, "mode should not change");

        match &app.messages[0].blocks[0] {
            ContentBlock::ToolUse { collapsed, .. } => assert!(!*collapsed, "block should be expanded after 'e'"),
            _ => panic!("expected ToolUse block"),
        }
    }

    // ─── Regression tests ──────────────────────────────────────────

    /// Regression: after on_turn_complete, streaming_text is flushed into messages so
    /// the next turn can produce a fresh assistant message rather than appending to nothing.
    #[test]
    fn regression_multi_turn_text_flush() {
        let mut app = App::new();

        // First turn: text delta then complete
        app.on_text_delta("first response");
        app.on_turn_complete("end_turn");

        // streaming_text must be empty (flushed)
        assert!(app.streaming_text.is_empty(), "streaming_text must be empty after turn complete");
        // The text must be stored in a message block
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].role, MessageRole::Assistant);
        match &app.messages[0].blocks[0] {
            ContentBlock::Text(t) => assert_eq!(t, "first response"),
            _ => panic!("expected Text block in first turn"),
        }

        // Second turn: simulate user message then another assistant response
        app.add_user_message("follow-up question");
        app.on_text_delta("second response");
        app.on_turn_complete("end_turn");

        assert!(app.streaming_text.is_empty(), "streaming_text must be empty after second turn complete");
        // Now there should be assistant + user + assistant messages
        assert_eq!(app.messages.len(), 3);
        assert_eq!(app.messages[1].role, MessageRole::User);
        assert_eq!(app.messages[2].role, MessageRole::Assistant);
        match &app.messages[2].blocks[0] {
            ContentBlock::Text(t) => assert_eq!(t, "second response"),
            _ => panic!("expected Text block in second turn"),
        }
    }

    /// Regression: Up/Down arrows cycle through completions and wrap around correctly.
    #[test]
    fn regression_completion_navigation_up_down() {
        let mut app = App::new();
        app.command_completions = vec!["help".to_string(), "clear".to_string(), "quit".to_string()];

        // Initially no selection
        assert_eq!(app.completion_index, None);

        // Down selects first entry
        app.completion_next();
        assert_eq!(app.completion_index, Some(0));

        // Down again selects second
        app.completion_next();
        assert_eq!(app.completion_index, Some(1));

        // Down again selects third
        app.completion_next();
        assert_eq!(app.completion_index, Some(2));

        // Down again wraps back to first
        app.completion_next();
        assert_eq!(app.completion_index, Some(0));

        // Up from first wraps to last
        app.completion_prev();
        assert_eq!(app.completion_index, Some(2));

        // Up steps backwards
        app.completion_prev();
        assert_eq!(app.completion_index, Some(1));

        // Up from None also goes to last
        app.completion_index = None;
        app.completion_prev();
        assert_eq!(app.completion_index, Some(2));
    }

    /// Regression: pressing Enter on a selected completion fills the input with "/cmd "
    /// and clears the completions dropdown.
    #[test]
    fn regression_completion_enter_fills_input() {
        let mut app = App::new();
        app.command_completions = vec!["help".to_string(), "clear".to_string()];
        app.completion_index = Some(1); // "clear" is selected

        let action = app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        // Action must be Continue (not Submit) — just fills the input
        assert_eq!(action, AppAction::Continue);
        // Input should now contain "/clear "
        assert_eq!(app.input.content(), "/clear ");
        // Completions must be cleared
        assert!(app.command_completions.is_empty());
        assert_eq!(app.completion_index, None);
    }

    /// Regression: on_thinking_delta sets thinking=true; on_text_delta resets it to false.
    /// on_turn_complete clears the accumulated thinking text.
    #[test]
    fn regression_thinking_state_transitions() {
        let mut app = App::new();

        // thinking delta sets thinking=true and accumulates text
        app.on_thinking_delta("let me think...");
        assert!(app.thinking, "thinking should be true after on_thinking_delta");
        assert_eq!(app.streaming_thinking, "let me think...");

        // text delta resets thinking=false
        app.on_text_delta("answer");
        assert!(!app.thinking, "thinking should be false after on_text_delta");

        // thinking text accumulated separately from main text
        assert_eq!(app.streaming_thinking, "let me think...");
        assert_eq!(app.streaming_text, "answer");

        // turn complete clears both
        app.on_turn_complete("end_turn");
        assert!(!app.thinking, "thinking should be false after on_turn_complete");
        assert!(app.streaming_thinking.is_empty(), "thinking text must be flushed on turn complete");
        assert!(app.streaming_text.is_empty(), "streaming text must be flushed on turn complete");
    }

    /// Regression: auto_follow=true causes follow_if_needed to keep offset at 0 (newest
    /// content visible). Scrolling up disables it. Scrolling back to 0 re-enables it.
    #[test]
    fn regression_auto_scroll_follows_new_content() {
        let mut scroll = ScrollState::default();
        assert!(scroll.auto_follow);

        scroll.set_content_size(200, 20);

        // With auto_follow=true, follow_if_needed keeps offset at 0
        scroll.offset = 50;
        scroll.target = 50;
        scroll.follow_if_needed();
        assert_eq!(scroll.offset, 0, "follow_if_needed must reset offset to 0 when auto_follow=true");
        assert_eq!(scroll.target, 0);

        // Scrolling up disables auto_follow
        scroll.scroll_up(10);
        assert!(!scroll.auto_follow, "scroll_up must disable auto_follow");
        assert_eq!(scroll.offset, 10);

        // follow_if_needed must NOT move offset when auto_follow=false
        scroll.follow_if_needed();
        assert_eq!(scroll.offset, 10, "follow_if_needed must not change offset when auto_follow=false");

        // Scrolling back to 0 re-enables auto_follow
        scroll.scroll_down(10);
        assert!(scroll.auto_follow, "scrolling back to 0 must re-enable auto_follow");
        assert_eq!(scroll.offset, 0);

        // Now follow_if_needed actively tracks again
        scroll.offset = 30;
        scroll.follow_if_needed();
        assert_eq!(scroll.offset, 0, "follow_if_needed must reset offset again after re-enabling auto_follow");
    }

    /// Regression: extract_input_summary("Bash", ...) returns the command field value.
    #[test]
    fn regression_tool_input_summary_bash() {
        let input = serde_json::json!({"command": "cargo test"});
        assert_eq!(extract_input_summary("Bash", &input), "cargo test");
    }

    /// Regression: a command longer than 80 characters is truncated with "...".
    #[test]
    fn regression_tool_input_summary_truncation() {
        // Construct a command that is exactly 81 characters (> 80 threshold)
        let long_command = "x".repeat(81);
        let input = serde_json::json!({"command": long_command});
        let summary = extract_input_summary("Bash", &input);
        assert!(
            summary.ends_with("..."),
            "summary of a long command must end with '...'"
        );
        // Total length: 80 chars + "..." = 83
        assert_eq!(summary.len(), 83, "truncated summary must be exactly 83 chars (80 + '...')");
    }
}
