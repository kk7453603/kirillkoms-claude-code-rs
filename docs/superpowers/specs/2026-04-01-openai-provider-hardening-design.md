# OpenAI-Compatible Provider Hardening

**Date:** 2026-04-01
**Status:** Approved
**Scope:** 4 independent improvements to OpenAI-compatible provider quality

## Context

Testing with local Ollama (qwen3:14b, qwen3-coder:30b, dolphin-llama3) revealed
4 gaps between the agent and Claude Code quality when using OpenAI-compatible
models. The agent infrastructure (tool dispatch, streaming, translation layer)
works correctly — all gaps are in edge case handling.

## Decision Summary

| Feature | Approach | Files |
|---------|----------|-------|
| Reasoning fallback | Pragmatic: empty content → use reasoning | openai_types.rs, openai_translate.rs |
| Universal system prompt | Provider-agnostic prompt for all models | context.rs |
| Tool-fallback retry | Smart retry with session-level caching | query_engine.rs |
| Turn timeout | Configurable idle timeout with hard cut | query_loop.rs, main.rs, env.rs |

## Feature 1: Reasoning Field Fallback

### Problem
Ollama thinking models (Qwen3, Qwen3.5) return responses in a `reasoning` field
instead of `content`. The content field is empty, causing blank responses.

### Design
- Add `reasoning: Option<String>` to `ResponseMessage` and `ResponseDelta`
- Non-streaming: if `content` is empty/None and `reasoning` is non-empty,
  use `reasoning` as the text content
- Streaming: if `delta.content` is empty but `delta.reasoning` is non-empty,
  translate reasoning deltas as `ContentDelta::TextDelta` events
- No parsing or splitting of reasoning content — show as-is
- No impact on Anthropic providers (field doesn't exist in their API)

### Tests
- Non-streaming: empty content + filled reasoning → Text block
- Streaming: reasoning deltas with empty content → TextDelta events
- Normal response with content → reasoning ignored

## Feature 2: Universal System Prompt

### Problem
Current prompt says "You are Claude, an AI assistant" — misleading for non-Claude
models and lacks tool-use guidance that helps weaker models.

### Design
Replace in `cc-engine/src/context.rs`:

```
"You are Claude, an AI assistant. Current date: {date}. OS: {os}. Working directory: {cwd}."
```

With:

```
"You are an AI coding assistant. You help users with software engineering tasks.
Current date: {date}. OS: {os}. Working directory: {cwd}.

When using tools, follow these principles:
- Read files before modifying them
- Use the appropriate tool for each task (Bash for commands, Read for files, Grep for search)
- Handle errors gracefully and report them clearly
- Be concise in your responses"
```

- Single prompt for all providers — no branching
- Git branch, git status, CLAUDE.md injection unchanged
- cache_control logic unchanged (handled in translation layer)

### Tests
- Update existing context.rs tests for new prompt text

## Feature 3: Tool-Fallback Retry

### Problem
Models like `dolphin-llama3` don't support function calling. The agent crashes
with "does not support tools" error. No graceful degradation.

### Design

**Session state** in `QueryEngine`:
```rust
tools_supported: Option<bool>  // None = unknown, Some(false) = no support
```

**Flow:**
1. First turn: send request with tools normally
2. If error message matches known patterns ("does not support tools",
   "does not support functions", "tool_use is not supported"):
   - Set `tools_supported = Some(false)`
   - Retry: strip `tools` from request, inject text-based tool descriptions
     as additional system block
   - Parse tool calls from response text
3. Subsequent turns: if `tools_supported == Some(false)`, skip tools directly
4. If first request succeeds: `tools_supported = Some(true)`, no overhead

**Text-based tool description** (injected as system block):
```
You have access to these tools. To use a tool, respond with a JSON block:

\```tool_call
{"name": "tool_name", "arguments": {"key": "value"}}
\```

Available tools:
{generated from ToolRegistry}
```

**Parsing tool calls from text:**
- Regex: ` ```tool_call\n(.*?)\n``` ` (multiline)
- Parse JSON → `ContentBlock::ToolUse`
- Invalid JSON → skip block, keep as text
- Text between blocks → `ContentBlock::Text`
- Multiple blocks → multiple ToolUse content blocks

**Error detection patterns:**
- `"does not support tools"`
- `"does not support functions"`
- `"tool_use is not supported"`
- `"tools is not supported"`

### Tests
- Normal request with tools → tools_supported = true, no changes
- Error "does not support tools" → retry without tools, tools_supported = false
- Parse tool_call blocks: valid JSON, invalid JSON, no blocks, multiple blocks
- Second turn after fallback → tools not sent

## Feature 4: Configurable Turn Timeout

### Problem
Thinking models (qwen3:14b) can enter reasoning loops, causing the agent to hang
indefinitely. No way to interrupt except Ctrl+C.

### Design

**Configuration:**
- Env: `CLAUDE_CODE_TURN_TIMEOUT` (seconds), default `0` (disabled)
- Read in `EnvConfig` as `turn_timeout_secs: Option<u64>`
- Passed to `QueryLoopParams` as `turn_timeout: Option<Duration>`

**Mechanism (idle timeout, not wall-clock):**
- Timer resets on every received SSE chunk (post-translation)
- If no chunk arrives within timeout duration → `cancel.cancel()`
- Collect partial response from `StreamAccumulator`
- Append `\n\n[Response truncated: no data received for {N}s]` to last text block
- Set `stop_reason = "timeout"` — engine does not retry

**Why idle timeout:**
- Wall-clock would kill slow but alive generation (30B at 5 tok/s)
- Idle timeout only fires when model is truly stuck
- Default `0` = disabled, zero risk to existing behavior

### Tests
- turn_timeout = None → stream works unchanged
- turn_timeout = 1s + stream that goes silent → partial response + warning
- turn_timeout = 60s + normal stream → timeout does not fire

## Backward Compatibility

| Provider | Changes | Unchanged |
|----------|---------|-----------|
| Direct (Anthropic) | Universal system prompt | HTTP, auth, streaming, types |
| Bedrock | Universal system prompt | Everything else |
| Vertex | Universal system prompt | Everything else |
| Foundry | Universal system prompt | Everything else |
| OpenAI-compatible | + reasoning fallback | HTTP, error handling, SSE |

**Graceful degradation:**
- reasoning field absent → `None`, no effect
- Model without tools + parser finds nothing → text shown as-is
- turn_timeout = 0 → disabled, identical to current behavior
- Ollama not running → existing `ConnectionError` unchanged

## Verification

1. All existing 1351 tests pass (except context.rs tests updated for new prompt)
2. New tests for each feature
3. `cargo clippy --workspace` clean in changed files
4. Manual test: Ollama qwen3:14b with all 4 features active
