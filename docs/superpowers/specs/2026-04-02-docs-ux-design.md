# Documentation UX Improvements — Design Spec

**Date:** 2026-04-02
**Status:** Approved

---

## Problem

The README has two main UX gaps identified by users:

1. **Configuration is fragmented** — environment variables are scattered across provider sections with no single reference.
2. **No concrete usage examples** — it's unclear what the agent can actually do in practice.

Additionally, 99 slash commands are mentioned but never documented.

---

## Approach

Hybrid: extend README with a Configuration Reference and Usage Scenarios; move the full slash commands list to a separate `docs/commands.md` to keep the README readable.

---

## Target Audience

All three: new users, experienced users, contributors.

---

## Scope

### 1. Configuration Reference (README)

A complete table of all environment variables, grouped by category. Each row includes: variable name, type/values, default, and description.

**Groups:**
- Anthropic Direct (`ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`, `CLAUDE_CODE_OAUTH_TOKEN`, `ANTHROPIC_BASE_URL`, `ANTHROPIC_MODEL` / `CLAUDE_MODEL`)
- Provider selection flags (`CLAUDE_CODE_USE_BEDROCK` / `CLAUDE_USE_BEDROCK`, `CLAUDE_CODE_USE_VERTEX` / `CLAUDE_USE_VERTEX`, `CLAUDE_CODE_USE_FOUNDRY` / `CLAUDE_USE_FOUNDRY`, `CLAUDE_CODE_USE_OPENAI`)
- AWS Bedrock (`AWS_REGION` / `AWS_DEFAULT_REGION`, `AWS_PROFILE`, `ANTHROPIC_BEDROCK_BASE_URL`)
- Google Vertex AI (`ANTHROPIC_VERTEX_PROJECT_ID` / `CLOUD_ML_PROJECT_ID` / `GOOGLE_CLOUD_PROJECT`, `CLOUD_ML_REGION`, `ANTHROPIC_VERTEX_BASE_URL`, `GOOGLE_ACCESS_TOKEN`)
- Azure Foundry (`ANTHROPIC_FOUNDRY_BASE_URL` / `AZURE_FOUNDRY_BASE_URL`, `ANTHROPIC_FOUNDRY_RESOURCE` / `AZURE_FOUNDRY_RESOURCE`, `AZURE_AD_TOKEN`)
- OpenAI-compatible (`OPENAI_API_KEY`, `OPENAI_BASE_URL` / `OPENAI_API_BASE`, `OPENAI_MODEL`)
- Features (`CLAUDE_MAX_THINKING_TOKENS`, `CLAUDE_MAX_OUTPUT_TOKENS`, `CLAUDE_BASH_DEFAULT_TIMEOUT_MS`, `CLAUDE_BASH_MAX_TIMEOUT_MS`, `CLAUDE_BASH_MAX_OUTPUT_LENGTH`, `CLAUDE_CODE_TURN_TIMEOUT`, `SEARXNG_URL`)
- Behavior (`CLAUDE_DISABLE_TELEMETRY` / `DISABLE_TELEMETRY`, `CLAUDE_CODE_ENABLE_TELEMETRY`, `CLAUDE_DISABLE_ERROR_REPORTING`, `CLAUDE_DISABLE_AUTO_UPDATER`, `CLAUDE_SIMPLE_MODE`, `CLAUDE_CONFIG_DIR`, `CI` / `CLAUDE_CI`, `CLAUDE_SANDBOX` / `SANDBOX`)
- MCP auth (`MCP_{NAME}_API_KEY`, `MCP_{NAME}_BEARER_TOKEN`)

Notable inconsistencies found in code (document both forms):
- Provider selection: `CLAUDE_CODE_USE_*` (auth.rs) vs `CLAUDE_USE_*` (env.rs) — both work
- Foundry URL: `ANTHROPIC_FOUNDRY_BASE_URL` (env.rs) vs `AZURE_FOUNDRY_BASE_URL` (client.rs) — both work

### 2. Usage Scenarios (README)

7 scenarios, each with: task description + commands + expected result (~10-15 lines each).

1. Рефакторинг функции
2. Code Review перед PR (`/review-pr`)
3. Дебаггинг по стектрейсу
4. Добавление unit-тестов
5. Работа с git (`/commit`, `/branch`)
6. Поиск по кодовой базе
7. Многоагентная задача (Agent tool)

### 3. Slash Commands Reference (`docs/commands.md`)

All 99 commands in a table grouped by category, extracted from `CommandDef` definitions. README gets a single link line.

**Categories:**
| Category | Commands |
|----------|---------|
| Основные | `help`, `clear`, `exit`, `status`, `version` |
| Модель и конфиг | `model`, `config`, `fast`, `effort`, `env` |
| Git и код | `commit`, `commit-push-pr`, `diff`, `branch`, `review`, `autofix-pr`, `pr-comments`, `security-review`, `release-notes` |
| Сессии | `session`, `resume`, `rename`, `rewind`, `compact`, `summary`, `export`, `share`, `tag` |
| Контекст | `context`, `ctx_viz`, `files`, `add-dir`, `memory`, `teleport` |
| AI-режимы | `plan`, `ultraplan`, `bughunter`, `advisor`, `thinkback`, `thinkback-play` |
| Задачи и агенты | `tasks`, `agents`, `skills` |
| Интеграции | `mcp`, `hooks`, `ide`, `chrome`, `bridge`, `bridge-kick`, `remote-env`, `remote-setup` |
| Настройка | `theme`, `color`, `vim`, `keybindings`, `permissions`, `statusline`, `terminal-setup`, `sandbox-toggle` |
| Аккаунт | `login`, `logout`, `upgrade`, `install`, `install-github-app`, `install-slack-app`, `passes`, `usage`, `cost`, `stats`, `oauth-refresh`, `privacy-settings`, `rate-limit-options`, `feedback`, `bug`, `onboarding` |
| Плагины | `plugin`, `reload-plugins` |
| Внутренние | `ant-trace`, `backfill-sessions`, `break-cache`, `debug-tool-call`, `heapdump`, `mock-limits`, `reset-limits`, `perf-issue` |
| Утилиты | `copy`, `btw`, `brief`, `good-claude`, `stickers`, `insights`, `extra-usage`, `init`, `init-verifiers`, `issue` |

---

## Implementation Order

1. `docs/commands.md` — generate from source `CommandDef` descriptions
2. Configuration Reference section in README — full env vars table
3. Usage Scenarios section in README — 7 scenarios

---

## What Is Already Done

- Local Providers section (Ollama, LM Studio, vLLM) — added
- Cloud Providers section (Bedrock, Vertex, Foundry) — added
- Ollama usage example with thinking model and tool-fallback notes — added
- `CLAUDE_CODE_TURN_TIMEOUT` in Quick Start Configuration — added
