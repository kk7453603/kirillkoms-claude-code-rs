# Slash Commands Reference

A complete reference for all 99 slash commands available in Claude Code, organized into 13 categories.

---

## Основные

| Команда | Описание |
|---------|----------|
| `/help` | Show help information |
| `/clear` | Clear conversation history |
| `/exit` | Exit the application |
| `/copy` | Copy last response to clipboard |
| `/version` | Show version information |

---

## Модель и конфигурация

| Команда | Описание |
|---------|----------|
| `/model` | View or change the current model |
| `/config` | View or modify configuration |
| `/effort` | Set reasoning effort level |
| `/fast` | Toggle fast mode (use faster model) |
| `/color` | Set output color/style preferences |

---

## Git и код

| Команда | Описание |
|---------|----------|
| `/commit` | Create a git commit with AI-generated message |
| `/commit-push-pr` | Commit changes, push, and create a pull request |
| `/branch` | Create or switch git branch |
| `/diff` | Show changes made in this session |
| `/review` | Review a pull request |
| `/pr-comments` | Review PR comments |
| `/autofix-pr` | Automatically fix issues in a pull request |
| `/security-review` | Complete a security review of pending changes on the current branch |
| `/release-notes` | Generate release notes from git history |

---

## Сессии

| Команда | Описание |
|---------|----------|
| `/session` | Manage sessions |
| `/resume` | Resume a previous session |
| `/rename` | Rename current session |
| `/tag` | Tag current point in conversation for later reference |
| `/rewind` | Undo the last conversation turn |
| `/compact` | Compact conversation to save context |
| `/summary` | Summarize the current conversation |
| `/export` | Export session transcript |
| `/share` | Share session transcript |

---

## Контекст

| Команда | Описание |
|---------|----------|
| `/context` | Show context information |
| `/ctx_viz` | Visualize the current context window usage |
| `/files` | List recently modified files |
| `/memory` | View or edit CLAUDE.md memory files |
| `/add-dir` | Add additional working directory |
| `/cost` | Show token usage and cost for this session |

---

## AI-режимы

| Команда | Описание |
|---------|----------|
| `/plan` | Toggle plan mode (read-only) |
| `/ultraplan` | Create a comprehensive implementation plan using extended thinking |
| `/brief` | Toggle brief output mode for shorter responses |
| `/thinkback` | Show Claude's extended thinking from the last response |
| `/thinkback-play` | Replay Claude's thinking process step by step |
| `/btw` | Send a side note without changing conversation context |

---

## Задачи и агенты

| Команда | Описание |
|---------|----------|
| `/tasks` | Task management |
| `/agents` | List agent definitions |
| `/skills` | List available skills |

---

## Интеграции

| Команда | Описание |
|---------|----------|
| `/mcp` | Manage MCP servers |
| `/ide` | IDE integration info |
| `/chrome` | Open Claude in Chrome with MCP integration |
| `/desktop` | Open Claude Code in the desktop app |
| `/mobile` | Show QR code to download the Claude mobile app |
| `/voice` | Toggle voice input mode |
| `/bridge` | Manage bidirectional bridge connection |
| `/remote-env` | Configure the default remote environment for teleport sessions |
| `/remote-setup` | Set up remote development environment |
| `/teleport` | Change working directory |

---

## Настройка

| Команда | Описание |
|---------|----------|
| `/theme` | Change the color theme |
| `/keybindings` | Show keyboard shortcuts |
| `/vim` | Toggle vim keybinding mode |
| `/hooks` | Manage event hooks |
| `/permissions` | View or manage tool permissions |
| `/terminal-setup` | Configure terminal integration for Claude Code |
| `/statusline` | Set up Claude Code's status line UI |
| `/sandbox-toggle` | Toggle sandbox mode for command execution |

---

## Аккаунт

| Команда | Описание |
|---------|----------|
| `/login` | Login to Anthropic |
| `/logout` | Logout from Anthropic |
| `/oauth-refresh` | Refresh OAuth tokens |
| `/usage` | Show token usage details |
| `/extra-usage` | Show extended usage information and tips |
| `/passes` | Show available Claude Code passes and subscription info |
| `/rate-limit-options` | Show options when rate limit is reached |
| `/privacy-settings` | View and manage privacy settings |
| `/install` | Install or update Claude Code |
| `/install-github-app` | Install the Claude GitHub App |
| `/install-slack-app` | Install the Claude Slack app |
| `/upgrade` | Check for updates |
| `/feedback` | Send feedback to the team |
| `/bug` | Report a bug |
| `/onboarding` | Run the onboarding flow for new users |
| `/stickers` | Show available Claude Code stickers |

---

## Плагины

| Команда | Описание |
|---------|----------|
| `/plugin` | Plugin management |
| `/reload-plugins` | Activate pending plugin changes in the current session |

---

## Утилиты

| Команда | Описание |
|---------|----------|
| `/status` | Show current session status |
| `/stats` | Show session statistics |
| `/doctor` | Check system health and configuration |
| `/env` | Show environment variables relevant to Claude Code |
| `/init` | Initialize Claude in project |
| `/init-verifiers` | Initialize verification hooks for the project |
| `/insights` | Show insights about agent tool usage and patterns |
| `/advisor` | Get advice on how to approach a coding task |
| `/bughunter` | Systematically hunt for bugs in the codebase |
| `/good-claude` | Give positive feedback for the last response |

---

## Внутренние

| Команда | Описание |
|---------|----------|
| `/ant-trace` | Show internal trace information (Anthropic-only) |
| `/backfill-sessions` | Backfill session data (internal) |
| `/break-cache` | Invalidate prompt cache (internal) |
| `/debug-tool-call` | Debug information for tool calls (internal) |
| `/heapdump` | Capture a heap dump (internal) |
| `/mock-limits` | Simulate rate limits for testing (internal) |
| `/reset-limits` | Reset rate limit counters (internal) |
| `/perf-issue` | Report a performance issue (internal) |
