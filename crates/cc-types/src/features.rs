/// Feature flags for optional functionality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Feature {
    Repl,
    Proactive,
    Kairos,
    KairosWebhooks,
    AgentTriggers,
    AgentTriggersRemote,
    MonitorTool,
    ContextCollapse,
    TerminalPanel,
    WebBrowser,
    CoordinatorMode,
    HistorySnip,
    UdsInbox,
    WorkflowScripts,
    VerifyPlan,
    BridgeMode,
    VoiceMode,
}

impl Feature {
    /// Check if this feature is enabled via compile-time feature flags.
    pub fn is_enabled(&self) -> bool {
        match self {
            #[cfg(feature = "repl")]
            Feature::Repl => return true,
            #[cfg(feature = "proactive")]
            Feature::Proactive => return true,
            #[cfg(feature = "kairos")]
            Feature::Kairos => return true,
            #[cfg(feature = "kairos_webhooks")]
            Feature::KairosWebhooks => return true,
            #[cfg(feature = "agent_triggers")]
            Feature::AgentTriggers => return true,
            #[cfg(feature = "agent_triggers_remote")]
            Feature::AgentTriggersRemote => return true,
            #[cfg(feature = "monitor_tool")]
            Feature::MonitorTool => return true,
            #[cfg(feature = "context_collapse")]
            Feature::ContextCollapse => return true,
            #[cfg(feature = "terminal_panel")]
            Feature::TerminalPanel => return true,
            #[cfg(feature = "web_browser")]
            Feature::WebBrowser => return true,
            #[cfg(feature = "coordinator_mode")]
            Feature::CoordinatorMode => return true,
            #[cfg(feature = "history_snip")]
            Feature::HistorySnip => return true,
            #[cfg(feature = "uds_inbox")]
            Feature::UdsInbox => return true,
            #[cfg(feature = "workflow_scripts")]
            Feature::WorkflowScripts => return true,
            #[cfg(feature = "verify_plan")]
            Feature::VerifyPlan => return true,
            #[cfg(feature = "bridge_mode")]
            Feature::BridgeMode => return true,
            #[cfg(feature = "voice_mode")]
            Feature::VoiceMode => return true,
            _ => false,
        }
    }

    /// Return the name of this feature as a static string.
    pub fn name(&self) -> &'static str {
        match self {
            Feature::Repl => "repl",
            Feature::Proactive => "proactive",
            Feature::Kairos => "kairos",
            Feature::KairosWebhooks => "kairos_webhooks",
            Feature::AgentTriggers => "agent_triggers",
            Feature::AgentTriggersRemote => "agent_triggers_remote",
            Feature::MonitorTool => "monitor_tool",
            Feature::ContextCollapse => "context_collapse",
            Feature::TerminalPanel => "terminal_panel",
            Feature::WebBrowser => "web_browser",
            Feature::CoordinatorMode => "coordinator_mode",
            Feature::HistorySnip => "history_snip",
            Feature::UdsInbox => "uds_inbox",
            Feature::WorkflowScripts => "workflow_scripts",
            Feature::VerifyPlan => "verify_plan",
            Feature::BridgeMode => "bridge_mode",
            Feature::VoiceMode => "voice_mode",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn features_default_disabled() {
        let features = [
            Feature::Repl,
            Feature::Proactive,
            Feature::Kairos,
            Feature::KairosWebhooks,
            Feature::AgentTriggers,
            Feature::AgentTriggersRemote,
            Feature::MonitorTool,
            Feature::ContextCollapse,
            Feature::TerminalPanel,
            Feature::WebBrowser,
            Feature::CoordinatorMode,
            Feature::HistorySnip,
            Feature::UdsInbox,
            Feature::WorkflowScripts,
            Feature::VerifyPlan,
            Feature::BridgeMode,
            Feature::VoiceMode,
        ];
        for feature in features {
            assert!(
                !feature.is_enabled(),
                "{:?} should be disabled by default",
                feature
            );
        }
    }

    #[test]
    fn feature_names() {
        assert_eq!(Feature::Repl.name(), "repl");
        assert_eq!(Feature::Proactive.name(), "proactive");
        assert_eq!(Feature::Kairos.name(), "kairos");
        assert_eq!(Feature::KairosWebhooks.name(), "kairos_webhooks");
        assert_eq!(Feature::AgentTriggers.name(), "agent_triggers");
        assert_eq!(Feature::AgentTriggersRemote.name(), "agent_triggers_remote");
        assert_eq!(Feature::MonitorTool.name(), "monitor_tool");
        assert_eq!(Feature::ContextCollapse.name(), "context_collapse");
        assert_eq!(Feature::TerminalPanel.name(), "terminal_panel");
        assert_eq!(Feature::WebBrowser.name(), "web_browser");
        assert_eq!(Feature::CoordinatorMode.name(), "coordinator_mode");
        assert_eq!(Feature::HistorySnip.name(), "history_snip");
        assert_eq!(Feature::UdsInbox.name(), "uds_inbox");
        assert_eq!(Feature::WorkflowScripts.name(), "workflow_scripts");
        assert_eq!(Feature::VerifyPlan.name(), "verify_plan");
        assert_eq!(Feature::BridgeMode.name(), "bridge_mode");
        assert_eq!(Feature::VoiceMode.name(), "voice_mode");
    }

    #[test]
    fn feature_equality() {
        assert_eq!(Feature::Repl, Feature::Repl);
        assert_ne!(Feature::Repl, Feature::Proactive);
    }

    #[test]
    fn feature_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Feature::Repl);
        set.insert(Feature::Proactive);
        set.insert(Feature::Repl); // duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn feature_clone_and_copy() {
        let f = Feature::WebBrowser;
        let f2 = f; // Copy
        let f3 = f.clone();
        assert_eq!(f, f2);
        assert_eq!(f, f3);
    }
}
