/// State for rendering a side-by-side or unified diff view.
#[derive(Debug, Clone)]
pub struct DiffView {
    pub old_text: String,
    pub new_text: String,
    pub file_path: Option<String>,
    pub scroll_offset: usize,
}

impl DiffView {
    pub fn new(old_text: &str, new_text: &str) -> Self {
        Self {
            old_text: old_text.to_string(),
            new_text: new_text.to_string(),
            file_path: None,
            scroll_offset: 0,
        }
    }

    pub fn with_file_path(mut self, path: &str) -> Self {
        self.file_path = Some(path.to_string());
        self
    }

    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset += lines;
    }

    /// Produce a simple unified diff representation as lines.
    pub fn unified_diff_lines(&self) -> Vec<DiffLine> {
        let old_lines: Vec<&str> = self.old_text.lines().collect();
        let new_lines: Vec<&str> = self.new_text.lines().collect();

        let mut result = Vec::new();

        if let Some(path) = &self.file_path {
            result.push(DiffLine::Header(format!("--- {path}")));
            result.push(DiffLine::Header(format!("+++ {path}")));
        }

        // Simple line-by-line comparison (not a real diff algorithm)
        let max_len = old_lines.len().max(new_lines.len());
        for i in 0..max_len {
            match (old_lines.get(i), new_lines.get(i)) {
                (Some(old), Some(new)) if old == new => {
                    result.push(DiffLine::Context(old.to_string()));
                }
                (Some(old), Some(new)) => {
                    result.push(DiffLine::Removed(old.to_string()));
                    result.push(DiffLine::Added(new.to_string()));
                }
                (Some(old), None) => {
                    result.push(DiffLine::Removed(old.to_string()));
                }
                (None, Some(new)) => {
                    result.push(DiffLine::Added(new.to_string()));
                }
                (None, None) => {}
            }
        }

        result
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffLine {
    Header(String),
    Context(String),
    Added(String),
    Removed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_view_identical() {
        let dv = DiffView::new("hello\nworld", "hello\nworld");
        let lines = dv.unified_diff_lines();
        assert!(lines.iter().all(|l| matches!(l, DiffLine::Context(_))));
    }

    #[test]
    fn test_diff_view_changes() {
        let dv = DiffView::new("aaa\nbbb", "aaa\nccc");
        let lines = dv.unified_diff_lines();
        assert_eq!(lines[0], DiffLine::Context("aaa".to_string()));
        assert_eq!(lines[1], DiffLine::Removed("bbb".to_string()));
        assert_eq!(lines[2], DiffLine::Added("ccc".to_string()));
    }

    #[test]
    fn test_diff_view_with_file_path() {
        let dv = DiffView::new("a", "b").with_file_path("test.rs");
        let lines = dv.unified_diff_lines();
        assert!(matches!(&lines[0], DiffLine::Header(h) if h.contains("test.rs")));
    }

    #[test]
    fn test_scroll() {
        let mut dv = DiffView::new("", "");
        dv.scroll_down(5);
        assert_eq!(dv.scroll_offset, 5);
        dv.scroll_up(3);
        assert_eq!(dv.scroll_offset, 2);
        dv.scroll_up(10);
        assert_eq!(dv.scroll_offset, 0);
    }
}
