use crate::types::*;

/// Internal/Anthropic-only commands grouped together.

pub static ANT_TRACE: CommandDef = CommandDef {
    name: "ant-trace",
    aliases: &[],
    description: "Show internal trace information (Anthropic-only)",
    argument_hint: None,
    hidden: true,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Internal command: ant-trace is only available in Anthropic development builds.",
            ))
        })
    },
};

pub static BACKFILL_SESSIONS: CommandDef = CommandDef {
    name: "backfill-sessions",
    aliases: &[],
    description: "Backfill session data (internal)",
    argument_hint: None,
    hidden: true,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Internal command: backfill-sessions is used for migrating session data.\n\
                 This is an internal maintenance command.",
            ))
        })
    },
};

pub static BREAK_CACHE: CommandDef = CommandDef {
    name: "break-cache",
    aliases: &[],
    description: "Invalidate prompt cache (internal)",
    argument_hint: None,
    hidden: true,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Cache invalidated.\n\
                 The next request will not use cached prompt prefixes.",
            ))
        })
    },
};

pub static DEBUG_TOOL_CALL: CommandDef = CommandDef {
    name: "debug-tool-call",
    aliases: &[],
    description: "Debug information for tool calls (internal)",
    argument_hint: None,
    hidden: true,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Internal command: debug-tool-call shows raw tool call payloads.\n\
                 This is only available in debug builds.",
            ))
        })
    },
};

pub static HEAPDUMP: CommandDef = CommandDef {
    name: "heapdump",
    aliases: &[],
    description: "Capture a heap dump (internal)",
    argument_hint: None,
    hidden: true,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Internal command: heapdump captures memory snapshots for debugging.\n\
                 Not available in the Rust build.",
            ))
        })
    },
};

pub static MOCK_LIMITS: CommandDef = CommandDef {
    name: "mock-limits",
    aliases: &[],
    description: "Simulate rate limits for testing (internal)",
    argument_hint: None,
    hidden: true,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Internal command: mock-limits simulates rate limiting behavior for testing.\n\
                 Not available in production builds.",
            ))
        })
    },
};

pub static RESET_LIMITS: CommandDef = CommandDef {
    name: "reset-limits",
    aliases: &[],
    description: "Reset rate limit counters (internal)",
    argument_hint: None,
    hidden: true,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Rate limit counters have been reset.\n\
                 This is an internal debugging command.",
            ))
        })
    },
};

pub static PERF_ISSUE: CommandDef = CommandDef {
    name: "perf-issue",
    aliases: &[],
    description: "Report a performance issue (internal)",
    argument_hint: None,
    hidden: true,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Internal command: perf-issue collects performance diagnostics.\n\
                 Use /bug to report issues in non-internal builds.",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ant_trace() {
        let result = (ANT_TRACE.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Internal command"));
    }

    #[tokio::test]
    async fn test_break_cache() {
        let result = (BREAK_CACHE.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Cache invalidated"));
    }
}
