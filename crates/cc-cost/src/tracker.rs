use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::model_costs;

/// Usage statistics for a session, optionally scoped to a single model.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub web_search_requests: u64,
    pub cost_usd: f64,
}

impl SessionUsage {
    /// Accumulate another usage record into this one.
    pub fn accumulate(&mut self, other: &SessionUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cache_read_input_tokens += other.cache_read_input_tokens;
        self.cache_creation_input_tokens += other.cache_creation_input_tokens;
        self.web_search_requests += other.web_search_requests;
        self.cost_usd += other.cost_usd;
    }

    /// Total tokens (input + output).
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

/// Tracks API costs, token usage, and durations for a session.
#[derive(Debug)]
pub struct CostTracker {
    usage_by_model: HashMap<String, SessionUsage>,
    total_api_duration: Duration,
    total_tool_duration: Duration,
    start_time: Instant,
    max_budget_usd: Option<f64>,
}

impl CostTracker {
    /// Create a new cost tracker with an optional maximum budget in USD.
    pub fn new(max_budget: Option<f64>) -> Self {
        Self {
            usage_by_model: HashMap::new(),
            total_api_duration: Duration::ZERO,
            total_tool_duration: Duration::ZERO,
            start_time: Instant::now(),
            max_budget_usd: max_budget,
        }
    }

    /// Record API usage for a model. Cost is calculated automatically from token counts.
    pub fn record_api_usage(
        &mut self,
        model: &str,
        input_tokens: u64,
        output_tokens: u64,
        cache_read_tokens: u64,
        cache_creation_tokens: u64,
        duration: Duration,
    ) {
        let cost = model_costs::calculate_cost(
            model,
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cache_creation_tokens,
        );

        let entry = self.usage_by_model.entry(model.to_string()).or_default();
        entry.input_tokens += input_tokens;
        entry.output_tokens += output_tokens;
        entry.cache_read_input_tokens += cache_read_tokens;
        entry.cache_creation_input_tokens += cache_creation_tokens;
        entry.cost_usd += cost;

        self.total_api_duration += duration;
    }

    /// Record a web search request for a model.
    pub fn record_web_search(&mut self, model: &str) {
        let entry = self.usage_by_model.entry(model.to_string()).or_default();
        entry.web_search_requests += 1;
    }

    /// Record tool execution duration.
    pub fn record_tool_duration(&mut self, duration: Duration) {
        self.total_tool_duration += duration;
    }

    /// Get total cost in USD across all models.
    pub fn total_cost_usd(&self) -> f64 {
        self.usage_by_model.values().map(|u| u.cost_usd).sum()
    }

    /// Get total input tokens across all models.
    pub fn total_input_tokens(&self) -> u64 {
        self.usage_by_model.values().map(|u| u.input_tokens).sum()
    }

    /// Get total output tokens across all models.
    pub fn total_output_tokens(&self) -> u64 {
        self.usage_by_model.values().map(|u| u.output_tokens).sum()
    }

    /// Get total API duration.
    pub fn total_api_duration(&self) -> Duration {
        self.total_api_duration
    }

    /// Get total tool duration.
    pub fn total_tool_duration(&self) -> Duration {
        self.total_tool_duration
    }

    /// Get elapsed time since session started.
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Check if the budget has been exceeded.
    pub fn is_budget_exceeded(&self) -> bool {
        match self.max_budget_usd {
            Some(budget) => self.total_cost_usd() >= budget,
            None => false,
        }
    }

    /// Get remaining budget in USD. Returns `None` if no budget is set.
    pub fn remaining_budget(&self) -> Option<f64> {
        self.max_budget_usd
            .map(|budget| (budget - self.total_cost_usd()).max(0.0))
    }

    /// Get usage broken down by model.
    pub fn usage_by_model(&self) -> &HashMap<String, SessionUsage> {
        &self.usage_by_model
    }

    /// Get total usage across all models.
    pub fn total_usage(&self) -> SessionUsage {
        let mut total = SessionUsage::default();
        for usage in self.usage_by_model.values() {
            total.accumulate(usage);
        }
        total
    }

    /// Reset all counters and timers (except start_time and budget).
    pub fn reset(&mut self) {
        self.usage_by_model.clear();
        self.total_api_duration = Duration::ZERO;
        self.total_tool_duration = Duration::ZERO;
        self.start_time = Instant::now();
    }
}

/// Thread-safe wrapper around [`CostTracker`].
#[derive(Debug, Clone)]
pub struct SharedCostTracker {
    inner: Arc<RwLock<CostTracker>>,
}

impl SharedCostTracker {
    /// Create a new shared cost tracker.
    pub fn new(max_budget: Option<f64>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(CostTracker::new(max_budget))),
        }
    }

    /// Record API usage for a model.
    pub fn record_api_usage(
        &self,
        model: &str,
        input_tokens: u64,
        output_tokens: u64,
        cache_read_tokens: u64,
        cache_creation_tokens: u64,
        duration: Duration,
    ) {
        self.inner
            .write()
            .expect("cost tracker lock poisoned")
            .record_api_usage(
                model,
                input_tokens,
                output_tokens,
                cache_read_tokens,
                cache_creation_tokens,
                duration,
            );
    }

    /// Record a web search request.
    pub fn record_web_search(&self, model: &str) {
        self.inner
            .write()
            .expect("cost tracker lock poisoned")
            .record_web_search(model);
    }

    /// Record tool execution duration.
    pub fn record_tool_duration(&self, duration: Duration) {
        self.inner
            .write()
            .expect("cost tracker lock poisoned")
            .record_tool_duration(duration);
    }

    /// Get total cost in USD.
    pub fn total_cost_usd(&self) -> f64 {
        self.inner
            .read()
            .expect("cost tracker lock poisoned")
            .total_cost_usd()
    }

    /// Get total input tokens.
    pub fn total_input_tokens(&self) -> u64 {
        self.inner
            .read()
            .expect("cost tracker lock poisoned")
            .total_input_tokens()
    }

    /// Get total output tokens.
    pub fn total_output_tokens(&self) -> u64 {
        self.inner
            .read()
            .expect("cost tracker lock poisoned")
            .total_output_tokens()
    }

    /// Check if the budget has been exceeded.
    pub fn is_budget_exceeded(&self) -> bool {
        self.inner
            .read()
            .expect("cost tracker lock poisoned")
            .is_budget_exceeded()
    }

    /// Get total usage across all models.
    pub fn total_usage(&self) -> SessionUsage {
        self.inner
            .read()
            .expect("cost tracker lock poisoned")
            .total_usage()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_usage_default() {
        let u = SessionUsage::default();
        assert_eq!(u.input_tokens, 0);
        assert_eq!(u.output_tokens, 0);
        assert_eq!(u.total_tokens(), 0);
        assert_eq!(u.cost_usd, 0.0);
    }

    #[test]
    fn session_usage_accumulate() {
        let mut a = SessionUsage {
            input_tokens: 100,
            output_tokens: 50,
            cache_read_input_tokens: 10,
            cache_creation_input_tokens: 5,
            web_search_requests: 1,
            cost_usd: 0.01,
        };
        let b = SessionUsage {
            input_tokens: 200,
            output_tokens: 100,
            cache_read_input_tokens: 20,
            cache_creation_input_tokens: 10,
            web_search_requests: 2,
            cost_usd: 0.02,
        };
        a.accumulate(&b);
        assert_eq!(a.input_tokens, 300);
        assert_eq!(a.output_tokens, 150);
        assert_eq!(a.cache_read_input_tokens, 30);
        assert_eq!(a.cache_creation_input_tokens, 15);
        assert_eq!(a.web_search_requests, 3);
        assert!((a.cost_usd - 0.03).abs() < f64::EPSILON);
    }

    #[test]
    fn session_usage_total_tokens() {
        let u = SessionUsage {
            input_tokens: 100,
            output_tokens: 200,
            ..Default::default()
        };
        assert_eq!(u.total_tokens(), 300);
    }

    #[test]
    fn session_usage_serde_roundtrip() {
        let u = SessionUsage {
            input_tokens: 500,
            output_tokens: 200,
            cache_read_input_tokens: 50,
            cache_creation_input_tokens: 25,
            web_search_requests: 3,
            cost_usd: 1.23,
        };
        let json = serde_json::to_string(&u).unwrap();
        let back: SessionUsage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.input_tokens, 500);
        assert_eq!(back.output_tokens, 200);
        assert_eq!(back.web_search_requests, 3);
        assert!((back.cost_usd - 1.23).abs() < f64::EPSILON);
    }

    #[test]
    fn tracker_new_defaults() {
        let t = CostTracker::new(None);
        assert_eq!(t.total_cost_usd(), 0.0);
        assert_eq!(t.total_input_tokens(), 0);
        assert_eq!(t.total_output_tokens(), 0);
        assert_eq!(t.total_api_duration(), Duration::ZERO);
        assert_eq!(t.total_tool_duration(), Duration::ZERO);
        assert!(!t.is_budget_exceeded());
        assert!(t.remaining_budget().is_none());
        assert!(t.usage_by_model().is_empty());
    }

    #[test]
    fn tracker_record_api_usage() {
        let mut t = CostTracker::new(None);
        t.record_api_usage(
            "claude-opus-4-6",
            1000,
            500,
            100,
            50,
            Duration::from_millis(200),
        );

        assert_eq!(t.total_input_tokens(), 1000);
        assert_eq!(t.total_output_tokens(), 500);
        assert!(t.total_cost_usd() > 0.0);
        assert_eq!(t.total_api_duration(), Duration::from_millis(200));

        let usage = t.usage_by_model().get("claude-opus-4-6").unwrap();
        assert_eq!(usage.input_tokens, 1000);
        assert_eq!(usage.output_tokens, 500);
        assert_eq!(usage.cache_read_input_tokens, 100);
        assert_eq!(usage.cache_creation_input_tokens, 50);
    }

    #[test]
    fn tracker_multiple_models() {
        let mut t = CostTracker::new(None);
        t.record_api_usage(
            "claude-opus-4-6",
            1000,
            500,
            0,
            0,
            Duration::from_millis(100),
        );
        t.record_api_usage(
            "claude-sonnet-4-6",
            2000,
            1000,
            0,
            0,
            Duration::from_millis(150),
        );

        assert_eq!(t.total_input_tokens(), 3000);
        assert_eq!(t.total_output_tokens(), 1500);
        assert_eq!(t.total_api_duration(), Duration::from_millis(250));
        assert_eq!(t.usage_by_model().len(), 2);

        let total = t.total_usage();
        assert_eq!(total.input_tokens, 3000);
        assert_eq!(total.output_tokens, 1500);
    }

    #[test]
    fn tracker_accumulates_same_model() {
        let mut t = CostTracker::new(None);
        t.record_api_usage(
            "claude-opus-4-6",
            1000,
            500,
            0,
            0,
            Duration::from_millis(100),
        );
        t.record_api_usage(
            "claude-opus-4-6",
            2000,
            1000,
            0,
            0,
            Duration::from_millis(200),
        );

        assert_eq!(t.usage_by_model().len(), 1);
        let opus = t.usage_by_model().get("claude-opus-4-6").unwrap();
        assert_eq!(opus.input_tokens, 3000);
        assert_eq!(opus.output_tokens, 1500);
    }

    #[test]
    fn tracker_web_search() {
        let mut t = CostTracker::new(None);
        t.record_web_search("claude-opus-4-6");
        t.record_web_search("claude-opus-4-6");
        t.record_web_search("claude-sonnet-4-6");

        let opus = t.usage_by_model().get("claude-opus-4-6").unwrap();
        assert_eq!(opus.web_search_requests, 2);
        let sonnet = t.usage_by_model().get("claude-sonnet-4-6").unwrap();
        assert_eq!(sonnet.web_search_requests, 1);
    }

    #[test]
    fn tracker_tool_duration() {
        let mut t = CostTracker::new(None);
        t.record_tool_duration(Duration::from_millis(100));
        t.record_tool_duration(Duration::from_millis(200));
        assert_eq!(t.total_tool_duration(), Duration::from_millis(300));
    }

    #[test]
    fn tracker_budget_not_exceeded() {
        let mut t = CostTracker::new(Some(100.0));
        t.record_api_usage(
            "claude-opus-4-6",
            1000,
            500,
            0,
            0,
            Duration::from_millis(100),
        );
        assert!(!t.is_budget_exceeded());
        let remaining = t.remaining_budget().unwrap();
        assert!(remaining > 0.0);
        assert!(remaining < 100.0);
    }

    #[test]
    fn tracker_budget_exceeded() {
        let mut t = CostTracker::new(Some(0.001));
        // Record enough usage to exceed $0.001
        t.record_api_usage(
            "claude-opus-4-6",
            1_000_000,
            1_000_000,
            0,
            0,
            Duration::from_millis(100),
        );
        assert!(t.is_budget_exceeded());
        assert_eq!(t.remaining_budget().unwrap(), 0.0);
    }

    #[test]
    fn tracker_no_budget_never_exceeded() {
        let mut t = CostTracker::new(None);
        t.record_api_usage(
            "claude-opus-4-6",
            1_000_000,
            1_000_000,
            0,
            0,
            Duration::from_millis(100),
        );
        assert!(!t.is_budget_exceeded());
        assert!(t.remaining_budget().is_none());
    }

    #[test]
    fn tracker_reset() {
        let mut t = CostTracker::new(Some(10.0));
        t.record_api_usage(
            "claude-opus-4-6",
            1000,
            500,
            0,
            0,
            Duration::from_millis(100),
        );
        t.record_tool_duration(Duration::from_millis(50));

        t.reset();

        assert_eq!(t.total_cost_usd(), 0.0);
        assert_eq!(t.total_input_tokens(), 0);
        assert_eq!(t.total_output_tokens(), 0);
        assert_eq!(t.total_api_duration(), Duration::ZERO);
        assert_eq!(t.total_tool_duration(), Duration::ZERO);
        assert!(t.usage_by_model().is_empty());
        // Budget should still be set
        assert!(!t.is_budget_exceeded());
        assert!(t.remaining_budget().is_some());
    }

    #[test]
    fn tracker_elapsed_is_positive() {
        let t = CostTracker::new(None);
        // Elapsed should be very small but non-negative
        assert!(t.elapsed().as_nanos() >= 0);
    }

    #[test]
    fn shared_tracker_basic() {
        let st = SharedCostTracker::new(Some(100.0));
        st.record_api_usage(
            "claude-opus-4-6",
            1000,
            500,
            0,
            0,
            Duration::from_millis(100),
        );
        st.record_web_search("claude-opus-4-6");
        st.record_tool_duration(Duration::from_millis(50));

        assert!(st.total_cost_usd() > 0.0);
        assert_eq!(st.total_input_tokens(), 1000);
        assert_eq!(st.total_output_tokens(), 500);
        assert!(!st.is_budget_exceeded());

        let total = st.total_usage();
        assert_eq!(total.input_tokens, 1000);
        assert_eq!(total.web_search_requests, 1);
    }

    #[test]
    fn shared_tracker_thread_safety() {
        use std::thread;

        let st = SharedCostTracker::new(None);
        let mut handles = vec![];

        for i in 0..10 {
            let tracker = st.clone();
            handles.push(thread::spawn(move || {
                tracker.record_api_usage(
                    "claude-opus-4-6",
                    100,
                    50,
                    0,
                    0,
                    Duration::from_millis(10),
                );
                if i % 2 == 0 {
                    tracker.record_web_search("claude-opus-4-6");
                }
                tracker.record_tool_duration(Duration::from_millis(5));
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(st.total_input_tokens(), 1000);
        assert_eq!(st.total_output_tokens(), 500);
        let total = st.total_usage();
        assert_eq!(total.web_search_requests, 5);
    }

    #[test]
    fn shared_tracker_budget_exceeded() {
        let st = SharedCostTracker::new(Some(0.0001));
        st.record_api_usage(
            "claude-opus-4-6",
            1_000_000,
            0,
            0,
            0,
            Duration::from_millis(100),
        );
        assert!(st.is_budget_exceeded());
    }

    #[test]
    fn tracker_cost_calculation_correctness() {
        let mut t = CostTracker::new(None);
        // 1M input tokens of opus = $15
        t.record_api_usage(
            "claude-opus-4-6",
            1_000_000,
            0,
            0,
            0,
            Duration::from_millis(100),
        );
        assert!((t.total_cost_usd() - 15.0).abs() < 1e-6);
    }

    #[test]
    fn tracker_unknown_model_zero_cost() {
        let mut t = CostTracker::new(None);
        t.record_api_usage(
            "gpt-4",
            1_000_000,
            1_000_000,
            0,
            0,
            Duration::from_millis(100),
        );
        assert_eq!(t.total_cost_usd(), 0.0);
        // Tokens should still be tracked
        assert_eq!(t.total_input_tokens(), 1_000_000);
        assert_eq!(t.total_output_tokens(), 1_000_000);
    }
}
