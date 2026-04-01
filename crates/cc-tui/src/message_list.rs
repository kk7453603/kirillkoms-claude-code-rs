/// Tracks scroll state for the message list display.
#[derive(Debug, Clone)]
pub struct MessageListState {
    /// Offset from the bottom (0 = fully scrolled to bottom)
    pub scroll_offset: usize,
    /// Total number of lines in the rendered message list
    pub total_lines: usize,
    /// Visible height of the viewport
    pub viewport_height: usize,
}

impl MessageListState {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0,
            total_lines: 0,
            viewport_height: 0,
        }
    }

    pub fn scroll_up(&mut self, lines: usize) {
        let max_offset = self.total_lines.saturating_sub(self.viewport_height);
        self.scroll_offset = (self.scroll_offset + lines).min(max_offset);
    }

    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn is_at_bottom(&self) -> bool {
        self.scroll_offset == 0
    }

    pub fn set_content_size(&mut self, total_lines: usize, viewport_height: usize) {
        self.total_lines = total_lines;
        self.viewport_height = viewport_height;
        // Clamp scroll offset
        let max_offset = total_lines.saturating_sub(viewport_height);
        if self.scroll_offset > max_offset {
            self.scroll_offset = max_offset;
        }
    }
}

impl Default for MessageListState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_at_bottom() {
        let state = MessageListState::new();
        assert!(state.is_at_bottom());
    }

    #[test]
    fn test_scroll_up_and_down() {
        let mut state = MessageListState::new();
        state.set_content_size(100, 20);

        state.scroll_up(5);
        assert_eq!(state.scroll_offset, 5);
        assert!(!state.is_at_bottom());

        state.scroll_down(3);
        assert_eq!(state.scroll_offset, 2);

        state.scroll_to_bottom();
        assert!(state.is_at_bottom());
    }

    #[test]
    fn test_scroll_up_clamped() {
        let mut state = MessageListState::new();
        state.set_content_size(30, 20);

        state.scroll_up(100);
        assert_eq!(state.scroll_offset, 10); // max = 30 - 20
    }

    #[test]
    fn test_scroll_down_clamped() {
        let mut state = MessageListState::new();
        state.scroll_down(100);
        assert_eq!(state.scroll_offset, 0);
    }
}
