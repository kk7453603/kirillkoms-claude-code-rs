/// Track permission denials per tool to avoid repeatedly asking the user.
use std::collections::HashMap;

/// Tracks how many times each tool has been denied permission.
#[derive(Debug)]
pub struct DenialTracker {
    denials: HashMap<String, u32>,
    max_denials: u32,
}

impl Default for DenialTracker {
    fn default() -> Self {
        Self::new(3)
    }
}

impl DenialTracker {
    /// Create a new tracker with the given maximum denial threshold.
    ///
    /// Once a tool has been denied `max_denials` times, `should_stop_asking`
    /// will return `true` for that tool.
    pub fn new(max_denials: u32) -> Self {
        Self {
            denials: HashMap::new(),
            max_denials,
        }
    }

    /// Record a denial for the given tool.
    pub fn record_denial(&mut self, tool_name: &str) {
        let count = self.denials.entry(tool_name.to_string()).or_insert(0);
        *count = count.saturating_add(1);
    }

    /// Return the number of times the given tool has been denied.
    pub fn denial_count(&self, tool_name: &str) -> u32 {
        self.denials.get(tool_name).copied().unwrap_or(0)
    }

    /// Whether the denial count for this tool has reached the threshold.
    pub fn should_stop_asking(&self, tool_name: &str) -> bool {
        self.denial_count(tool_name) >= self.max_denials
    }

    /// Reset all denial counts.
    pub fn reset(&mut self) {
        self.denials.clear();
    }

    /// Reset the denial count for a specific tool.
    pub fn reset_tool(&mut self, tool_name: &str) {
        self.denials.remove(tool_name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tracker_has_zero_counts() {
        let tracker = DenialTracker::new(3);
        assert_eq!(tracker.denial_count("Bash"), 0);
        assert_eq!(tracker.denial_count("Edit"), 0);
    }

    #[test]
    fn test_record_and_count() {
        let mut tracker = DenialTracker::new(3);
        tracker.record_denial("Bash");
        assert_eq!(tracker.denial_count("Bash"), 1);
        tracker.record_denial("Bash");
        assert_eq!(tracker.denial_count("Bash"), 2);
        tracker.record_denial("Edit");
        assert_eq!(tracker.denial_count("Edit"), 1);
        assert_eq!(tracker.denial_count("Bash"), 2);
    }

    #[test]
    fn test_should_stop_asking_threshold() {
        let mut tracker = DenialTracker::new(3);
        assert!(!tracker.should_stop_asking("Bash"));

        tracker.record_denial("Bash");
        assert!(!tracker.should_stop_asking("Bash"));

        tracker.record_denial("Bash");
        assert!(!tracker.should_stop_asking("Bash"));

        tracker.record_denial("Bash");
        assert!(tracker.should_stop_asking("Bash"));

        // Other tools unaffected
        assert!(!tracker.should_stop_asking("Edit"));
    }

    #[test]
    fn test_should_stop_asking_at_exactly_max() {
        let mut tracker = DenialTracker::new(1);
        assert!(!tracker.should_stop_asking("Bash"));
        tracker.record_denial("Bash");
        assert!(tracker.should_stop_asking("Bash"));
    }

    #[test]
    fn test_reset_clears_all() {
        let mut tracker = DenialTracker::new(3);
        tracker.record_denial("Bash");
        tracker.record_denial("Edit");
        tracker.reset();
        assert_eq!(tracker.denial_count("Bash"), 0);
        assert_eq!(tracker.denial_count("Edit"), 0);
        assert!(!tracker.should_stop_asking("Bash"));
    }

    #[test]
    fn test_reset_tool_clears_one() {
        let mut tracker = DenialTracker::new(3);
        tracker.record_denial("Bash");
        tracker.record_denial("Bash");
        tracker.record_denial("Edit");

        tracker.reset_tool("Bash");
        assert_eq!(tracker.denial_count("Bash"), 0);
        assert_eq!(tracker.denial_count("Edit"), 1);
    }

    #[test]
    fn test_reset_tool_nonexistent_is_noop() {
        let mut tracker = DenialTracker::new(3);
        tracker.reset_tool("NonExistent"); // should not panic
        assert_eq!(tracker.denial_count("NonExistent"), 0);
    }

    #[test]
    fn test_default_max_denials() {
        let tracker = DenialTracker::default();
        // Default is 3
        assert!(!tracker.should_stop_asking("Bash"));
    }

    #[test]
    fn test_zero_max_denials_always_stops() {
        let tracker = DenialTracker::new(0);
        // With max_denials=0, count(0) >= 0 is true immediately
        assert!(tracker.should_stop_asking("Bash"));
    }

    #[test]
    fn test_saturating_add() {
        let mut tracker = DenialTracker::new(u32::MAX);
        // Record many denials -- should not overflow
        for _ in 0..10 {
            tracker.record_denial("Bash");
        }
        assert_eq!(tracker.denial_count("Bash"), 10);
    }

    #[test]
    fn test_multiple_tools_independent() {
        let mut tracker = DenialTracker::new(2);
        tracker.record_denial("Bash");
        tracker.record_denial("Bash");
        tracker.record_denial("Edit");

        assert!(tracker.should_stop_asking("Bash"));
        assert!(!tracker.should_stop_asking("Edit"));

        tracker.record_denial("Edit");
        assert!(tracker.should_stop_asking("Edit"));
    }
}
