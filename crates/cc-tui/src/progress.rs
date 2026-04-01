pub struct Spinner {
    frames: Vec<&'static str>,
    current: usize,
    message: String,
}

impl Spinner {
    pub fn new(message: &str) -> Self {
        Self {
            frames: vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            current: 0,
            message: message.to_string(),
        }
    }

    pub fn tick(&mut self) -> &str {
        let frame = self.frames[self.current];
        self.current = (self.current + 1) % self.frames.len();
        frame
    }

    pub fn set_message(&mut self, msg: &str) {
        self.message = msg.to_string();
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_new() {
        let spinner = Spinner::new("Loading...");
        assert_eq!(spinner.message(), "Loading...");
    }

    #[test]
    fn test_spinner_tick_cycles() {
        let mut spinner = Spinner::new("test");
        let first = spinner.tick().to_string();
        let second = spinner.tick().to_string();
        assert_ne!(first, second);

        // Cycle through all frames and back to start
        for _ in 0..8 {
            spinner.tick();
        }
        let cycled = spinner.tick().to_string();
        // After 10 more ticks from frame 0, should be back at frame 1
        assert_eq!(cycled, second);
    }

    #[test]
    fn test_spinner_set_message() {
        let mut spinner = Spinner::new("old");
        spinner.set_message("new");
        assert_eq!(spinner.message(), "new");
    }
}
