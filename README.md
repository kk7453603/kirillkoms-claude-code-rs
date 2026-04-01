<p align="center">
  <h1 align="center">Claude Code RS</h1>
  <p align="center">AI-powered coding agent CLI &mdash; rewritten in Rust</p>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-SBSL%20v1.0-blue.svg" alt="License: SBSL v1.0"></a>
  <a href="https://github.com/kk7453603/kirillkoms-claude-code-rs/actions/workflows/ci.yml"><img src="https://github.com/kk7453603/kirillkoms-claude-code-rs/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://codecov.io/gh/kk7453603/kirillkoms-claude-code-rs"><img src="https://codecov.io/gh/kk7453603/kirillkoms-claude-code-rs/branch/main/graph/badge.svg" alt="Coverage"></a>
  <img src="https://img.shields.io/badge/tests-1342%20passing-brightgreen.svg" alt="Tests: 1342 passing">
  <img src="https://img.shields.io/badge/crates-18-orange.svg" alt="Crates: 18">
  <img src="https://img.shields.io/badge/tools-31-purple.svg" alt="Tools: 31">
  <img src="https://img.shields.io/badge/commands-99-teal.svg" alt="Commands: 99">
  <img src="https://img.shields.io/badge/rust-1.85%2B-red.svg" alt="Rust: 1.85+">
  <img src="https://img.shields.io/badge/platforms-linux%20%7C%20macos%20%7C%20windows-lightgrey.svg" alt="Platforms: linux | macos | windows">
</p>

---

## Overview

Claude Code RS is a complete Rust rewrite of the Claude Code CLI agent. It provides an interactive AI-powered coding assistant that can read, write, and edit files, execute commands, search the web, and interact with language servers — all from your terminal.

## Features

- **Multi-turn conversations** with Claude (Opus, Sonnet, Haiku) and any OpenAI-compatible model
- **31 tools**: Bash, File Read/Write/Edit, Grep, Glob, Web Search (SearXNG), Web Fetch, LSP, MCP, Agent spawning, Tasks, Skills, Notebooks, and more
- **99 slash commands**: `/help`, `/commit`, `/diff`, `/doctor`, `/model`, `/status`, `/init`, `/branch`, `/tasks`, `/export`, etc.
- **5 API providers**: Anthropic Direct, AWS Bedrock, Google Vertex AI, Azure Foundry, OpenAI-compatible (OpenAI, OpenRouter, Ollama, LM Studio, vLLM, Together AI, etc.)
- **Permission system**: Interactive approval for destructive operations, configurable modes (default/plan/auto/bypass)
- **Hook system**: PreToolUse/PostToolUse hooks around every tool call
- **Session persistence**: JSONL transcripts, `--resume` to continue conversations
- **Streaming**: Real-time SSE streaming from the API
- **Auto-compaction**: Automatic context management when token limits are approached
- **LSP integration**: Connect to rust-analyzer, typescript-language-server, pylsp, gopls
- **MCP support**: JSON-RPC stdio client for Model Context Protocol servers
- **Web search**: SearXNG integration (free, no API key required)
- **Cost tracking**: Per-model token accounting with USD pricing
- **File history**: Track every edit with timestamps

## Architecture

```
claude-code-rs/
  Cargo.toml                     # Workspace root
  crates/
    cc-cli/                      # Binary entrypoint, CLI args (clap)
    cc-engine/                   # QueryEngine, query loop, orchestration
    cc-api/                      # API client (5 providers, SSE streaming, retry)
    cc-tools/                    # Tool trait + 31 tool implementations
    cc-commands/                 # 99 slash commands
    cc-permissions/              # Permission system (modes, rules, bash security)
    cc-hooks/                    # Hook dispatch (PreToolUse, PostToolUse, etc.)
    cc-config/                   # Settings, CLAUDE.md, env vars, model config
    cc-types/                    # Shared types (messages, IDs, permissions, errors)
    cc-cost/                     # Cost tracking, model pricing
    cc-utils/                    # 20 utility modules (shell, git, diff, LSP, etc.)
    cc-compact/                  # Auto-compaction via API
    cc-mcp/                      # MCP client (JSON-RPC stdio)
    cc-session/                  # Session persistence, resume, history
    cc-skills/                   # Skill loader, bundled skills, plugins
    cc-tasks/                    # Task manager, worktree, agent tasks
    cc-analytics/                # Analytics events, telemetry
    cc-tui/                      # Terminal UI (ratatui)
```

### Crate Dependency Graph

```
cc-cli
  |-- cc-engine
  |     |-- cc-api (streaming, retry, 5 providers)
  |     |-- cc-tools (31 tools)
  |     |     |-- cc-permissions (modes, rules, bash security)
  |     |     |-- cc-mcp (MCP client)
  |     |     |-- cc-skills (bundled skills)
  |     |     |-- cc-tasks (task state)
  |     |-- cc-hooks (event dispatch)
  |     |-- cc-compact (auto-compaction)
  |     |-- cc-cost (token tracking)
  |     |-- cc-session (persistence)
  |-- cc-tui (terminal UI)
  |-- cc-commands (99 commands)
  |-- cc-config (settings, env)
  |-- cc-types (shared types)
  |-- cc-utils (20 utility modules)
  |-- cc-analytics (telemetry)
```

## Quick Start

### Prerequisites

- Rust 1.85+ (`rustup update stable`)
- An Anthropic API key **or** any OpenAI-compatible API key

### Build

```bash
git clone https://github.com/kk7453603/kirillkoms-claude-code-rs.git
cd kirillkoms-claude-code-rs
cargo build --release
```

### Run

```bash
# Set your API key (Anthropic)
export ANTHROPIC_API_KEY=sk-ant-...

# Or use any OpenAI-compatible API
export OPENAI_API_KEY=sk-...
export OPENAI_BASE_URL=https://api.openai.com   # or http://localhost:11434 for Ollama
export OPENAI_MODEL=gpt-4o                       # optional default model

# Interactive REPL
./target/release/claude-code

# Single prompt (pipe mode)
./target/release/claude-code -p "Explain this codebase" --print

# With model selection
./target/release/claude-code -m opus              # Anthropic alias
./target/release/claude-code -m gpt-4o            # OpenAI model
./target/release/claude-code -m llama3.1          # Ollama model

# Resume previous session
./target/release/claude-code --resume <session-id>

# Dump system prompt
./target/release/claude-code --dump-system-prompt
```

### Configuration

```bash
# SearXNG web search (default: searx.be)
export SEARXNG_URL=https://your-searxng-instance.com

# AWS Bedrock
export CLAUDE_CODE_USE_BEDROCK=1
export AWS_REGION=us-east-1

# Google Vertex AI
export CLAUDE_CODE_USE_VERTEX=1
export ANTHROPIC_VERTEX_PROJECT_ID=my-project
export CLOUD_ML_REGION=us-east5

# Azure Foundry
export CLAUDE_CODE_USE_FOUNDRY=1
export ANTHROPIC_FOUNDRY_BASE_URL=https://...
export ANTHROPIC_FOUNDRY_RESOURCE=my-resource

# OpenAI-compatible (OpenAI, OpenRouter, Ollama, LM Studio, vLLM, Together AI)
export OPENAI_API_KEY=sk-...
export OPENAI_BASE_URL=https://api.openai.com    # or any compatible endpoint
export OPENAI_MODEL=gpt-4o                        # default model (optional)

# Permission mode
./target/release/claude-code --permission-mode bypass
```

### OpenAI-Compatible Providers

Any API implementing the OpenAI Chat Completions format works out of the box:

| Provider | `OPENAI_BASE_URL` | Notes |
|----------|-------------------|-------|
| OpenAI | `https://api.openai.com` (default) | GPT-4o, GPT-4-turbo, etc. |
| OpenRouter | `https://openrouter.ai/api` | 100+ models from multiple providers |
| Ollama | `http://localhost:11434` | Local models, use `OPENAI_API_KEY=ollama` |
| LM Studio | `http://localhost:1234` | Local models |
| vLLM | `http://localhost:8000` | High-throughput serving |
| Together AI | `https://api.together.xyz` | Open-source models |
| Groq | `https://api.groq.com/openai` | Ultra-fast inference |

The provider translates Anthropic Messages API types to/from OpenAI Chat Completions format transparently. Tool use, streaming, and multi-turn conversations work as expected. Thinking blocks and cache control are silently skipped for non-Anthropic providers.

## Tools

| Tool | Description |
|------|-------------|
| **Bash** | Execute shell commands with timeout and security analysis |
| **Read** | Read files with line offset/limit support |
| **Write** | Create/overwrite files with parent directory creation |
| **Edit** | In-place string replacement in files |
| **Grep** | Recursive regex search with context lines and output modes |
| **Glob** | File pattern matching |
| **WebSearch** | SearXNG-powered web search (free, no API key) |
| **WebFetch** | HTTP content fetching with HTML-to-text conversion |
| **LSP** | Language server integration (definition, references, hover, symbols) |
| **Agent** | Spawn sub-agent processes for parallel work |
| **MCP** | Connect to Model Context Protocol servers |
| **Tasks** | Create, update, list, stop background tasks |
| **Skills** | Execute bundled skills (commit, review-pr, etc.) |
| **Notebook** | Edit Jupyter notebook cells |
| **Sleep** | Async delay for polling workflows |
| **PlanMode** | Enter/exit read-only planning mode |
| **Worktree** | Git worktree management for isolated work |
| **TodoWrite** | Session todo list management |
| **ToolSearch** | Search available tools by keyword |
| **Config** | Get/set configuration values |
| **AskUser** | Interactive user prompts |

## Tests

```bash
# Run all 1342 tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p cc-tools

# Run with all features
cargo test --workspace --all-features
```

## License

This project is licensed under the **Small Business Source License (SBSL) v1.0**.

- **Small businesses** (< 50 employees, < $5M revenue): **Free** for any use, including commercial
- **Enterprises**: Must obtain a [commercial license](mailto:kirillkoms@github.com)
- **30-day evaluation** period for enterprises

See [LICENSE](LICENSE) for full terms.
