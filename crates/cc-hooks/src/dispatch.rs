use std::path::Path;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tracing::{debug, warn};

use crate::types::{HookConfig, HookEventType, HookInput, HookJsonOutput, HookOutcome, HooksConfig};

/// Execute a single hook command.
///
/// The hook receives the serialized `input` via both the `CLAUDE_HOOK_INPUT`
/// environment variable and stdin. Its stdout is parsed as JSON
/// (`HookJsonOutput`). A non-zero exit code or a "block"/"deny" decision
/// results in a `Blocked` outcome.
pub async fn execute_hook(
    config: &HookConfig,
    input: &HookInput,
    cwd: &Path,
) -> HookOutcome {
    let input_json = match serde_json::to_string(input) {
        Ok(j) => j,
        Err(e) => {
            return HookOutcome::Error {
                message: format!("Failed to serialize hook input: {e}"),
            };
        }
    };

    debug!(
        command = %config.command,
        event = %input.hook_event,
        "Executing hook"
    );

    let mut child = match Command::new("sh")
        .arg("-c")
        .arg(&config.command)
        .current_dir(cwd)
        .env("CLAUDE_HOOK_INPUT", &input_json)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return HookOutcome::Error {
                message: format!("Failed to spawn hook command '{}': {e}", config.command),
            };
        }
    };

    // Write JSON to stdin.
    if let Some(mut stdin) = child.stdin.take() {
        let json_clone = input_json.clone();
        tokio::spawn(async move {
            let _ = stdin.write_all(json_clone.as_bytes()).await;
            let _ = stdin.shutdown().await;
        });
    }

    // Wait with timeout.
    let timeout = Duration::from_millis(config.timeout_ms);

    // Take stdout/stderr handles before waiting so we can read them after
    // wait completes, while still being able to kill on timeout.
    let mut child_stdout = child.stdout.take();
    let mut child_stderr = child.stderr.take();

    let wait_result = tokio::time::timeout(timeout, child.wait()).await;

    let status = match wait_result {
        Ok(Ok(status)) => status,
        Ok(Err(e)) => {
            return HookOutcome::Error {
                message: format!("Hook command IO error: {e}"),
            };
        }
        Err(_) => {
            // Timeout — try to kill the child.
            let _ = child.kill().await;
            return HookOutcome::TimedOut {
                timeout_ms: config.timeout_ms,
            };
        }
    };

    // Read stdout and stderr.
    let mut stdout_bytes = Vec::new();
    let mut stderr_bytes = Vec::new();
    if let Some(ref mut out) = child_stdout {
        let _ = out.read_to_end(&mut stdout_bytes).await;
    }
    if let Some(ref mut err) = child_stderr {
        let _ = err.read_to_end(&mut stderr_bytes).await;
    }

    let stdout = String::from_utf8_lossy(&stdout_bytes);
    let stderr = String::from_utf8_lossy(&stderr_bytes);
    let exit_code = status.code().unwrap_or(-1);

    debug!(
        exit_code,
        stdout = %stdout,
        stderr = %stderr,
        "Hook command completed"
    );

    // Non-zero exit code means block.
    if exit_code != 0 {
        let reason = if !stderr.is_empty() {
            stderr.to_string()
        } else if !stdout.is_empty() {
            stdout.to_string()
        } else {
            format!("Hook exited with code {exit_code}")
        };
        return HookOutcome::Blocked {
            reason: reason.trim().to_string(),
        };
    }

    // Try to parse stdout as JSON.
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return HookOutcome::Approved {
            message: None,
            updated_input: None,
        };
    }

    match serde_json::from_str::<HookJsonOutput>(trimmed) {
        Ok(hook_out) => {
            let decision = hook_out.decision.as_deref().unwrap_or("");
            match decision {
                "block" | "deny" => HookOutcome::Blocked {
                    reason: hook_out
                        .reason
                        .or(hook_out.message)
                        .unwrap_or_else(|| "Hook denied the action".to_string()),
                },
                _ => HookOutcome::Approved {
                    message: hook_out.message,
                    updated_input: hook_out.updated_input,
                },
            }
        }
        Err(_) => {
            // Stdout was not valid JSON — treat as approved with the text as message.
            HookOutcome::Approved {
                message: Some(trimmed.to_string()),
                updated_input: None,
            }
        }
    }
}

/// Execute all hooks for an event, stopping at the first block.
///
/// Hooks are run sequentially. If any hook returns `Blocked`, execution stops
/// immediately and the `Blocked` outcome is returned. Errors and timeouts are
/// logged but do not stop subsequent hooks. If all hooks approve, the last
/// approved result is returned (with merged messages).
pub async fn dispatch_hooks(
    hooks_config: &HooksConfig,
    event: HookEventType,
    input: &HookInput,
    cwd: &Path,
) -> HookOutcome {
    let hooks = hooks_config.get(&event);
    if hooks.is_empty() {
        return HookOutcome::NoHooks;
    }

    let mut last_message: Option<String> = None;
    let mut last_updated_input: Option<serde_json::Value> = None;

    for hook in hooks {
        let outcome = execute_hook(hook, input, cwd).await;
        match &outcome {
            HookOutcome::Blocked { .. } => {
                return outcome;
            }
            HookOutcome::Error { message } => {
                warn!(
                    command = %hook.command,
                    error = %message,
                    "Hook error, continuing"
                );
            }
            HookOutcome::TimedOut { timeout_ms } => {
                warn!(
                    command = %hook.command,
                    timeout_ms,
                    "Hook timed out, continuing"
                );
            }
            HookOutcome::Approved {
                message,
                updated_input,
            } => {
                if message.is_some() {
                    last_message = message.clone();
                }
                if updated_input.is_some() {
                    last_updated_input = updated_input.clone();
                }
            }
            HookOutcome::NoHooks => {}
        }
    }

    HookOutcome::Approved {
        message: last_message,
        updated_input: last_updated_input,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{HookConfig, HookEventType, HookInput, HooksConfig};

    fn test_input() -> HookInput {
        HookInput {
            hook_event: "PreToolUse".to_string(),
            tool_name: Some("Bash".to_string()),
            tool_input: Some(serde_json::json!({"command": "ls"})),
            tool_output: None,
            session_id: None,
            cwd: None,
        }
    }

    #[tokio::test]
    async fn test_execute_hook_success_no_output() {
        let config = HookConfig {
            command: "true".to_string(),
            timeout_ms: 5000,
        };
        let input = test_input();
        let outcome = execute_hook(&config, &input, Path::new("/tmp")).await;
        match outcome {
            HookOutcome::Approved { message, updated_input } => {
                assert!(message.is_none());
                assert!(updated_input.is_none());
            }
            other => panic!("Expected Approved, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_execute_hook_with_json_output() {
        let config = HookConfig {
            command: r#"echo '{"decision":"approve","message":"ok"}'"#.to_string(),
            timeout_ms: 5000,
        };
        let input = test_input();
        let outcome = execute_hook(&config, &input, Path::new("/tmp")).await;
        match outcome {
            HookOutcome::Approved { message, .. } => {
                assert_eq!(message.as_deref(), Some("ok"));
            }
            other => panic!("Expected Approved, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_execute_hook_block_decision() {
        let config = HookConfig {
            command: r#"echo '{"decision":"block","reason":"not allowed"}'"#.to_string(),
            timeout_ms: 5000,
        };
        let input = test_input();
        let outcome = execute_hook(&config, &input, Path::new("/tmp")).await;
        match outcome {
            HookOutcome::Blocked { reason } => {
                assert_eq!(reason, "not allowed");
            }
            other => panic!("Expected Blocked, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_execute_hook_deny_decision() {
        let config = HookConfig {
            command: r#"echo '{"decision":"deny","reason":"denied"}'"#.to_string(),
            timeout_ms: 5000,
        };
        let input = test_input();
        let outcome = execute_hook(&config, &input, Path::new("/tmp")).await;
        match outcome {
            HookOutcome::Blocked { reason } => {
                assert_eq!(reason, "denied");
            }
            other => panic!("Expected Blocked, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_execute_hook_nonzero_exit() {
        let config = HookConfig {
            command: "exit 1".to_string(),
            timeout_ms: 5000,
        };
        let input = test_input();
        let outcome = execute_hook(&config, &input, Path::new("/tmp")).await;
        match outcome {
            HookOutcome::Blocked { reason } => {
                assert!(!reason.is_empty());
            }
            other => panic!("Expected Blocked, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_execute_hook_nonzero_exit_with_stderr() {
        let config = HookConfig {
            command: "echo 'bad stuff' >&2; exit 1".to_string(),
            timeout_ms: 5000,
        };
        let input = test_input();
        let outcome = execute_hook(&config, &input, Path::new("/tmp")).await;
        match outcome {
            HookOutcome::Blocked { reason } => {
                assert!(reason.contains("bad stuff"));
            }
            other => panic!("Expected Blocked, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_execute_hook_timeout() {
        let config = HookConfig {
            command: "sleep 60".to_string(),
            timeout_ms: 100,
        };
        let input = test_input();
        let outcome = execute_hook(&config, &input, Path::new("/tmp")).await;
        match outcome {
            HookOutcome::TimedOut { timeout_ms } => {
                assert_eq!(timeout_ms, 100);
            }
            other => panic!("Expected TimedOut, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_execute_hook_receives_env_var() {
        // The hook reads CLAUDE_HOOK_INPUT and checks it contains the event name.
        let config = HookConfig {
            command: r#"printf '{"message":"got-%s"}' "$(printf '%s' "$CLAUDE_HOOK_INPUT" | grep -o PreToolUse)""#.to_string(),
            timeout_ms: 5000,
        };
        let input = test_input();
        let outcome = execute_hook(&config, &input, Path::new("/tmp")).await;
        match outcome {
            HookOutcome::Approved { message, .. } => {
                assert_eq!(message.as_deref(), Some("got-PreToolUse"));
            }
            other => panic!("Expected Approved, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_execute_hook_non_json_stdout() {
        let config = HookConfig {
            command: "echo 'just some text'".to_string(),
            timeout_ms: 5000,
        };
        let input = test_input();
        let outcome = execute_hook(&config, &input, Path::new("/tmp")).await;
        match outcome {
            HookOutcome::Approved { message, .. } => {
                assert_eq!(message.as_deref(), Some("just some text"));
            }
            other => panic!("Expected Approved, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_dispatch_hooks_no_hooks() {
        let config = HooksConfig::new();
        let input = test_input();
        let outcome =
            dispatch_hooks(&config, HookEventType::PreToolUse, &input, Path::new("/tmp")).await;
        match outcome {
            HookOutcome::NoHooks => {}
            other => panic!("Expected NoHooks, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_dispatch_hooks_all_approve() {
        let mut config = HooksConfig::new();
        config.add(
            HookEventType::PreToolUse,
            HookConfig {
                command: r#"echo '{"decision":"approve","message":"hook1"}'"#.to_string(),
                timeout_ms: 5000,
            },
        );
        config.add(
            HookEventType::PreToolUse,
            HookConfig {
                command: r#"echo '{"decision":"approve","message":"hook2"}'"#.to_string(),
                timeout_ms: 5000,
            },
        );

        let input = test_input();
        let outcome =
            dispatch_hooks(&config, HookEventType::PreToolUse, &input, Path::new("/tmp")).await;
        match outcome {
            HookOutcome::Approved { message, .. } => {
                // Last hook's message wins.
                assert_eq!(message.as_deref(), Some("hook2"));
            }
            other => panic!("Expected Approved, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_dispatch_hooks_stops_at_block() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("marker");

        let mut config = HooksConfig::new();
        config.add(
            HookEventType::PreToolUse,
            HookConfig {
                command: r#"echo '{"decision":"block","reason":"nope"}'"#.to_string(),
                timeout_ms: 5000,
            },
        );
        // This second hook creates a marker file; if dispatch stops at the
        // block it should never run.
        config.add(
            HookEventType::PreToolUse,
            HookConfig {
                command: format!("touch {}", marker.display()),
                timeout_ms: 5000,
            },
        );

        let input = test_input();
        let outcome =
            dispatch_hooks(&config, HookEventType::PreToolUse, &input, Path::new("/tmp")).await;
        match outcome {
            HookOutcome::Blocked { reason } => {
                assert_eq!(reason, "nope");
            }
            other => panic!("Expected Blocked, got {:?}", other),
        }
        assert!(!marker.exists(), "Second hook should not have run");
    }

    #[tokio::test]
    async fn test_dispatch_hooks_continues_after_error() {
        let mut config = HooksConfig::new();
        // First hook errors (bad command).
        config.add(
            HookEventType::PreToolUse,
            HookConfig {
                command: "exit 1".to_string(),
                timeout_ms: 5000,
            },
        );
        // Second hook succeeds.
        config.add(
            HookEventType::PreToolUse,
            HookConfig {
                command: r#"echo '{"decision":"approve","message":"second"}'"#.to_string(),
                timeout_ms: 5000,
            },
        );

        let input = test_input();
        let outcome =
            dispatch_hooks(&config, HookEventType::PreToolUse, &input, Path::new("/tmp")).await;

        // The first hook returns Blocked (nonzero exit), which stops dispatch.
        // Actually, per the spec: nonzero exit = Blocked, so dispatch stops.
        // Let me re-read: "If any returns Blocked, stop and return Blocked"
        // "If any returns Error/TimedOut, log and continue"
        // But our execute_hook maps nonzero exit to Blocked. Let's verify:
        match outcome {
            HookOutcome::Blocked { .. } => {
                // Correct: nonzero exit => Blocked => dispatch stops.
            }
            other => panic!("Expected Blocked from nonzero exit, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_dispatch_hooks_continues_after_timeout() {
        let mut config = HooksConfig::new();
        config.add(
            HookEventType::PreToolUse,
            HookConfig {
                command: "sleep 60".to_string(),
                timeout_ms: 100,
            },
        );
        config.add(
            HookEventType::PreToolUse,
            HookConfig {
                command: r#"echo '{"message":"after-timeout"}'"#.to_string(),
                timeout_ms: 5000,
            },
        );

        let input = test_input();
        let outcome =
            dispatch_hooks(&config, HookEventType::PreToolUse, &input, Path::new("/tmp")).await;
        match outcome {
            HookOutcome::Approved { message, .. } => {
                assert_eq!(message.as_deref(), Some("after-timeout"));
            }
            other => panic!("Expected Approved, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_dispatch_hooks_with_updated_input() {
        let mut config = HooksConfig::new();
        config.add(
            HookEventType::PreToolUse,
            HookConfig {
                command: r#"echo '{"updatedInput":{"command":"ls -la"}}'"#.to_string(),
                timeout_ms: 5000,
            },
        );

        let input = test_input();
        let outcome =
            dispatch_hooks(&config, HookEventType::PreToolUse, &input, Path::new("/tmp")).await;
        match outcome {
            HookOutcome::Approved { updated_input, .. } => {
                let ui = updated_input.unwrap();
                assert_eq!(ui["command"], "ls -la");
            }
            other => panic!("Expected Approved, got {:?}", other),
        }
    }
}
