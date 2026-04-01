/// A simple text input widget state.
#[derive(Debug, Clone)]
pub struct TextInput {
    content: String,
    cursor: usize,
}

impl TextInput {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor: 0,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.content.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn delete_char(&mut self) {
        if self.cursor > 0 {
            // Find the previous character boundary
            let prev = self.content[..self.cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.content.remove(prev);
            self.cursor = prev;
        }
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor = 0;
    }

    pub fn cursor_position(&self) -> usize {
        self.cursor
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.content[..self.cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.content.len() {
            self.cursor += self.content[self.cursor..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
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
    }

    #[test]
    fn test_insert_char() {
        let mut input = TextInput::new();
        input.insert_char('h');
        input.insert_char('i');
        assert_eq!(input.content(), "hi");
        assert_eq!(input.cursor_position(), 2);
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
    fn test_delete_char_at_start() {
        let mut input = TextInput::new();
        input.delete_char(); // should be no-op
        assert_eq!(input.content(), "");
    }

    #[test]
    fn test_clear() {
        let mut input = TextInput::new();
        input.insert_char('x');
        input.clear();
        assert!(input.is_empty());
        assert_eq!(input.cursor_position(), 0);
    }

    #[test]
    fn test_move_left_right() {
        let mut input = TextInput::new();
        input.insert_char('a');
        input.insert_char('b');
        assert_eq!(input.cursor_position(), 2);

        input.move_left();
        assert_eq!(input.cursor_position(), 1);

        input.move_left();
        assert_eq!(input.cursor_position(), 0);

        input.move_left(); // no-op at start
        assert_eq!(input.cursor_position(), 0);

        input.move_right();
        assert_eq!(input.cursor_position(), 1);
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
}
