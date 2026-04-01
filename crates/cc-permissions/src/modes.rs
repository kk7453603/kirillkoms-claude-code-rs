use serde::{Deserialize, Serialize};

/// Represents the permission mode that controls how tool usage is authorized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub enum PermissionMode {
    #[default]
    Default,
    Auto,
    Plan,
    AcceptEdits,
    BypassPermissions,
    DontAsk,
}

impl PermissionMode {
    /// Whether this mode allows read-only tools without prompting.
    pub fn allows_read_only(&self) -> bool {
        matches!(
            self,
            Self::Default | Self::Auto | Self::AcceptEdits | Self::BypassPermissions | Self::DontAsk
        )
    }

    /// Whether this mode allows file edits without prompting.
    pub fn allows_edits(&self) -> bool {
        matches!(
            self,
            Self::AcceptEdits | Self::BypassPermissions | Self::DontAsk
        )
    }

    /// Whether this mode allows all operations without prompting.
    pub fn allows_all(&self) -> bool {
        matches!(self, Self::BypassPermissions | Self::DontAsk)
    }

    /// Whether this mode blocks all write operations (plan mode).
    pub fn is_read_only_mode(&self) -> bool {
        matches!(self, Self::Plan)
    }

    /// Parse from string, returning None if unrecognized.
    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "default" | "Default" => Some(Self::Default),
            "auto" | "Auto" => Some(Self::Auto),
            "plan" | "Plan" => Some(Self::Plan),
            "acceptEdits" | "accept-edits" | "AcceptEdits" | "accept_edits" => {
                Some(Self::AcceptEdits)
            }
            "bypassPermissions" | "bypass-permissions" | "BypassPermissions"
            | "bypass_permissions" => Some(Self::BypassPermissions),
            "dontAsk" | "dont-ask" | "DontAsk" | "dont_ask" => Some(Self::DontAsk),
            _ => None,
        }
    }

    /// Return the canonical string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Auto => "auto",
            Self::Plan => "plan",
            Self::AcceptEdits => "acceptEdits",
            Self::BypassPermissions => "bypassPermissions",
            Self::DontAsk => "dontAsk",
        }
    }
}


impl std::fmt::Display for PermissionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allows_read_only() {
        assert!(PermissionMode::Default.allows_read_only());
        assert!(PermissionMode::Auto.allows_read_only());
        assert!(!PermissionMode::Plan.allows_read_only());
        assert!(PermissionMode::AcceptEdits.allows_read_only());
        assert!(PermissionMode::BypassPermissions.allows_read_only());
        assert!(PermissionMode::DontAsk.allows_read_only());
    }

    #[test]
    fn test_allows_edits() {
        assert!(!PermissionMode::Default.allows_edits());
        assert!(!PermissionMode::Auto.allows_edits());
        assert!(!PermissionMode::Plan.allows_edits());
        assert!(PermissionMode::AcceptEdits.allows_edits());
        assert!(PermissionMode::BypassPermissions.allows_edits());
        assert!(PermissionMode::DontAsk.allows_edits());
    }

    #[test]
    fn test_allows_all() {
        assert!(!PermissionMode::Default.allows_all());
        assert!(!PermissionMode::Auto.allows_all());
        assert!(!PermissionMode::Plan.allows_all());
        assert!(!PermissionMode::AcceptEdits.allows_all());
        assert!(PermissionMode::BypassPermissions.allows_all());
        assert!(PermissionMode::DontAsk.allows_all());
    }

    #[test]
    fn test_is_read_only_mode() {
        assert!(!PermissionMode::Default.is_read_only_mode());
        assert!(PermissionMode::Plan.is_read_only_mode());
        assert!(!PermissionMode::BypassPermissions.is_read_only_mode());
    }

    #[test]
    fn test_from_str_opt() {
        assert_eq!(
            PermissionMode::from_str_opt("default"),
            Some(PermissionMode::Default)
        );
        assert_eq!(
            PermissionMode::from_str_opt("auto"),
            Some(PermissionMode::Auto)
        );
        assert_eq!(
            PermissionMode::from_str_opt("plan"),
            Some(PermissionMode::Plan)
        );
        assert_eq!(
            PermissionMode::from_str_opt("acceptEdits"),
            Some(PermissionMode::AcceptEdits)
        );
        assert_eq!(
            PermissionMode::from_str_opt("accept-edits"),
            Some(PermissionMode::AcceptEdits)
        );
        assert_eq!(
            PermissionMode::from_str_opt("bypassPermissions"),
            Some(PermissionMode::BypassPermissions)
        );
        assert_eq!(
            PermissionMode::from_str_opt("dontAsk"),
            Some(PermissionMode::DontAsk)
        );
        assert_eq!(PermissionMode::from_str_opt("unknown"), None);
    }

    #[test]
    fn test_as_str_roundtrip() {
        let modes = [
            PermissionMode::Default,
            PermissionMode::Auto,
            PermissionMode::Plan,
            PermissionMode::AcceptEdits,
            PermissionMode::BypassPermissions,
            PermissionMode::DontAsk,
        ];
        for mode in modes {
            let s = mode.as_str();
            assert_eq!(PermissionMode::from_str_opt(s), Some(mode));
        }
    }

    #[test]
    fn test_display() {
        assert_eq!(PermissionMode::Default.to_string(), "default");
        assert_eq!(PermissionMode::AcceptEdits.to_string(), "acceptEdits");
    }

    #[test]
    fn test_serde_roundtrip() {
        let mode = PermissionMode::AcceptEdits;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"acceptEdits\"");
        let deserialized: PermissionMode = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, mode);
    }

    #[test]
    fn test_serde_all_variants() {
        let cases = [
            (PermissionMode::Default, "\"default\""),
            (PermissionMode::Auto, "\"auto\""),
            (PermissionMode::Plan, "\"plan\""),
            (PermissionMode::AcceptEdits, "\"acceptEdits\""),
            (PermissionMode::BypassPermissions, "\"bypassPermissions\""),
            (PermissionMode::DontAsk, "\"dontAsk\""),
        ];
        for (mode, expected_json) in cases {
            let json = serde_json::to_string(&mode).unwrap();
            assert_eq!(json, expected_json, "serialization failed for {:?}", mode);
            let back: PermissionMode = serde_json::from_str(&json).unwrap();
            assert_eq!(back, mode);
        }
    }

    #[test]
    fn test_default() {
        assert_eq!(PermissionMode::default(), PermissionMode::Default);
    }
}
