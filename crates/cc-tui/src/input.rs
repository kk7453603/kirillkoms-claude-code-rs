/// A multi-line text input widget state.
///
/// Supports Shift+Enter for newlines, Enter for submit.
/// Tracks cursor position as (line, column) internally but exposes
/// a flat byte offset for compatibility.
#[derive(Debug, Clone)]
pub struct TextInput {
    lines: Vec<String>,
    /// Cursor row (line index).
    cursor_row: usize,
    /// Cursor column (byte offset within the line).
    cursor_col: usize,
}

impl TextInput {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
        }
    }

    /// Get the full content as a single string (lines joined by \n).
    pub fn content(&self) -> String {
        self.lines.join("\n")
    }

    /// Get individual lines for rendering.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn cursor_row(&self) -> usize {
        self.cursor_row
    }

    pub fn cursor_col(&self) -> usize {
        self.cursor_col
    }

    /// Legacy: flat cursor position (byte offset into the joined content).
    pub fn cursor_position(&self) -> usize {
        let mut pos = 0;
        for (i, line) in self.lines.iter().enumerate() {
            if i == self.cursor_row {
                return pos + self.cursor_col;
            }
            pos += line.len() + 1; // +1 for \n
        }
        pos
    }

    pub fn insert_char(&mut self, c: char) {
        let line = &mut self.lines[self.cursor_row];
        line.insert(self.cursor_col, c);
        self.cursor_col += c.len_utf8();
    }

    /// Insert a newline at cursor (for Shift+Enter).
    pub fn insert_newline(&mut self) {
        let current = &self.lines[self.cursor_row];
        let rest = current[self.cursor_col..].to_string();
        self.lines[self.cursor_row] = current[..self.cursor_col].to_string();
        self.cursor_row += 1;
        self.lines.insert(self.cursor_row, rest);
        self.cursor_col = 0;
    }

    pub fn delete_char(&mut self) {
        if self.cursor_col > 0 {
            let line = &mut self.lines[self.cursor_row];
            // Find previous char boundary
            let prev = line[..self.cursor_col]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            line.remove(prev);
            self.cursor_col = prev;
        } else if self.cursor_row > 0 {
            // Merge with previous line
            let current = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].push_str(&current);
        }
    }

    pub fn clear(&mut self) {
        self.lines = vec![String::new()];
        self.cursor_row = 0;
        self.cursor_col = 0;
    }

    pub fn move_left(&mut self) {
        if self.cursor_col > 0 {
            let line = &self.lines[self.cursor_row];
            self.cursor_col = line[..self.cursor_col]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
        }
    }

    pub fn move_right(&mut self) {
        let line = &self.lines[self.cursor_row];
        if self.cursor_col < line.len() {
            self.cursor_col += line[self.cursor_col..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
        } else if self.cursor_row < self.lines.len() - 1 {
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor_row < self.lines.len() - 1 {
            self.cursor_row += 1;
            self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
        }
    }

    pub fn move_home(&mut self) {
        self.cursor_col = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor_col = self.lines[self.cursor_row].len();
    }

    pub fn is_empty(&self) -> bool {
        self.lines.len() == 1 && self.lines[0].is_empty()
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
}

impl Default for TextInput {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_input_empty() {
        let input = TextInput::new();
        assert!(input.is_empty());
        assert_eq!(input.content(), "");
        assert_eq!(input.cursor_position(), 0);
        assert_eq!(input.line_count(), 1);
    }

    #[test]
    fn test_insert_char() {
        let mut input = TextInput::new();
        input.insert_char('h');
        input.insert_char('i');
        assert_eq!(input.content(), "hi");
        assert_eq!(input.cursor_col(), 2);
    }

    #[test]
    fn test_insert_newline() {
        let mut input = TextInput::new();
        input.insert_char('a');
        input.insert_char('b');
        input.insert_newline();
        input.insert_char('c');
        assert_eq!(input.content(), "ab\nc");
        assert_eq!(input.line_count(), 2);
        assert_eq!(input.cursor_row(), 1);
        assert_eq!(input.cursor_col(), 1);
    }

    #[test]
    fn test_insert_newline_mid_line() {
        let mut input = TextInput::new();
        input.insert_char('a');
        input.insert_char('c');
        input.move_left();
        input.insert_newline();
        assert_eq!(input.lines()[0], "a");
        assert_eq!(input.lines()[1], "c");
    }

    #[test]
    fn test_delete_char() {
        let mut input = TextInput::new();
        input.insert_char('a');
        input.insert_char('b');
        input.insert_char('c');
        input.delete_char();
        assert_eq!(input.content(), "ab");
    }

    #[test]
    fn test_delete_char_merges_lines() {
        let mut input = TextInput::new();
        input.insert_char('a');
        input.insert_newline();
        input.insert_char('b');
        // cursor at row 1, col 1 ("b|")
        input.move_home(); // row 1, col 0
        input.delete_char(); // should merge with previous line
        assert_eq!(input.content(), "ab");
        assert_eq!(input.line_count(), 1);
        assert_eq!(input.cursor_col(), 1); // after 'a'
    }

    #[test]
    fn test_delete_char_at_start() {
        let mut input = TextInput::new();
        input.delete_char(); // no-op
        assert_eq!(input.content(), "");
    }

    #[test]
    fn test_clear() {
        let mut input = TextInput::new();
        input.insert_char('x');
        input.insert_newline();
        input.insert_char('y');
        input.clear();
        assert!(input.is_empty());
        assert_eq!(input.cursor_position(), 0);
    }

    #[test]
    fn test_move_left_right() {
        let mut input = TextInput::new();
        input.insert_char('a');
        input.insert_char('b');
        assert_eq!(input.cursor_col(), 2);

        input.move_left();
        assert_eq!(input.cursor_col(), 1);

        input.move_left();
        assert_eq!(input.cursor_col(), 0);

        input.move_left(); // no-op at start
        assert_eq!(input.cursor_col(), 0);

        input.move_right();
        assert_eq!(input.cursor_col(), 1);
    }

    #[test]
    fn test_move_left_across_lines() {
        let mut input = TextInput::new();
        input.insert_char('a');
        input.insert_newline();
        input.insert_char('b');
        input.move_home(); // start of line 1
        input.move_left(); // should go to end of line 0
        assert_eq!(input.cursor_row(), 0);
        assert_eq!(input.cursor_col(), 1); // after 'a'
    }

    #[test]
    fn test_move_right_across_lines() {
        let mut input = TextInput::new();
        input.insert_char('a');
        input.insert_newline();
        input.insert_char('b');
        // Go to end of line 0
        input.move_up();
        input.move_end();
        input.move_right(); // should wrap to start of line 1
        assert_eq!(input.cursor_row(), 1);
        assert_eq!(input.cursor_col(), 0);
    }

    #[test]
    fn test_move_up_down() {
        let mut input = TextInput::new();
        input.insert_char('a');
        input.insert_char('b');
        input.insert_newline();
        input.insert_char('c');

        assert_eq!(input.cursor_row(), 1);
        input.move_up();
        assert_eq!(input.cursor_row(), 0);
        assert_eq!(input.cursor_col(), 1); // clamped to line length

        input.move_down();
        assert_eq!(input.cursor_row(), 1);
    }

    #[test]
    fn test_move_home_end() {
        let mut input = TextInput::new();
        input.insert_char('a');
        input.insert_char('b');
        input.insert_char('c');

        input.move_home();
        assert_eq!(input.cursor_col(), 0);

        input.move_end();
        assert_eq!(input.cursor_col(), 3);
    }

    #[test]
    fn test_cursor_position_multiline() {
        let mut input = TextInput::new();
        input.insert_char('a');
        input.insert_char('b');
        input.insert_newline(); // "ab\n"
        input.insert_char('c');
        // Content: "ab\nc", cursor after 'c' at row=1, col=1
        // Flat position: "ab" (2) + "\n" (1) + "c" (1) = 4
        assert_eq!(input.cursor_position(), 4);
    }

    #[test]
    fn test_insert_at_cursor_middle() {
        let mut input = TextInput::new();
        input.insert_char('a');
        input.insert_char('c');
        input.move_left();
        input.insert_char('b');
        assert_eq!(input.content(), "abc");
    }

    #[test]
    fn test_is_empty() {
        let mut input = TextInput::new();
        assert!(input.is_empty());
        input.insert_newline();
        assert!(!input.is_empty()); // has 2 lines, even if empty
    }
}
