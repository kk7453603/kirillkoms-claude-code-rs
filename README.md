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
- **Tool-fallback retry**: Models without native function calling (e.g. dolphin-llama3) automatically fall back to text-based tool descriptions; result cached per session
- **Thinking model support**: Ollama Qwen3/Qwen3.5 `reasoning` field used as fallback when `content` is empty
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
export CLOUD_ML_PROJECT_ID=my-project
export CLOUD_ML_REGION=us-east5

# Azure Foundry
export CLAUDE_CODE_USE_FOUNDRY=1
export AZURE_FOUNDRY_BASE_URL=https://...
export AZURE_FOUNDRY_RESOURCE=my-resource

# OpenAI-compatible (OpenAI, OpenRouter, Ollama, LM Studio, vLLM, Together AI)
export OPENAI_API_KEY=sk-...
export OPENAI_BASE_URL=https://api.openai.com    # or any compatible endpoint
export OPENAI_MODEL=gpt-4o                        # default model (optional)

# Permission mode
./target/release/claude-code --permission-mode bypass

# Turn idle timeout (seconds, 0 = disabled)
export CLAUDE_CODE_TURN_TIMEOUT=120
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

---

## Local Providers

Claude Code RS поддерживает работу с локальными языковыми моделями через любой сервер, реализующий OpenAI Chat Completions API. Никакой интернет-доступ или платные ключи не требуются.

### Как это работает

Локальные провайдеры подключаются через тот же `OpenAiCompatibleClient`, что и облачные OpenAI-совместимые сервисы. На границе провайдера автоматически выполняется двусторонняя трансляция типов:

```
Внутренний формат (Anthropic Messages API)
         ↕  openai_translate
OpenAI Chat Completions API  →  локальный сервер
```

Трансляция покрывает:
- системные сообщения → `role: "system"`
- блоки `tool_use` / `tool_result` → `tool_calls` / `tool` роли
- `tool_choice: Any` → `"required"` в OpenAI-формате
- стриминг через SSE: накопительный буфер + `StreamTranslationState`
- поле `reasoning` как фолбэк к `content` для thinking-моделей (Qwen3, Qwen3.5)
- автоматический retry без инструментов для моделей без нативного function calling
- `thinking` блоки и `cache_control` — тихо игнорируются (не поддерживаются локальными моделями)

### Ollama

[Ollama](https://ollama.com) — наиболее распространённый способ запуска open-source моделей локально.

**Установка и запуск:**

```bash
# Установить Ollama (Linux/macOS)
curl -fsSL https://ollama.com/install.sh | sh

# Скачать модель
ollama pull llama3.1          # Meta Llama 3.1 8B
ollama pull mistral           # Mistral 7B
ollama pull qwen2.5-coder     # Qwen2.5 Coder 7B (рекомендуется для кода)
ollama pull deepseek-coder-v2 # DeepSeek Coder V2 16B
ollama pull qwen3:14b         # Qwen3 14B с поддержкой thinking

# Ollama автоматически запускается как сервис на порту 11434
```

**Настройка Claude Code RS:**

```bash
export OPENAI_BASE_URL=http://localhost:11434
export OPENAI_API_KEY=ollama   # Обязательно — любое непустое значение
export OPENAI_MODEL=llama3.1   # Модель по умолчанию (опционально)

./target/release/claude-code
```

**Выбор модели в сессии:**

```bash
# Через флаг запуска
./target/release/claude-code -m qwen2.5-coder

# Через команду /model внутри сессии
/model mistral
```

**Эндпоинт:** `http://localhost:11434/v1/chat/completions`

**Важно:** Ollama не требует реального API-ключа — передайте любое непустое значение (`ollama`, `local`, `test`). Поле обязательно по формату протокола.

**Thinking-модели (Qwen3):** Ollama-модели серии Qwen3/Qwen3.5 возвращают ответ в поле `reasoning` вместо `content`. Claude Code RS автоматически использует `reasoning` как фолбэк — дополнительной настройки не требуется.

**Модели без function calling:** Если модель не поддерживает нативный вызов инструментов (например, `dolphin-llama3`), агент автоматически делает retry с текстовыми описаниями инструментов в system prompt. Это поведение кешируется на уровне сессии — повторных попыток на каждый запрос нет.

**Пример сессии с Ollama:**

```bash
# 1. Запустить Ollama и скачать модель
ollama pull qwen2.5-coder:7b

# 2. Настроить Claude Code RS
export OPENAI_BASE_URL=http://localhost:11434
export OPENAI_API_KEY=ollama
export OPENAI_MODEL=qwen2.5-coder:7b

# 3. Запустить агента
./target/release/claude-code

# Пример диалога:
# > Объясни что делает функция parse_sse_line в crates/cc-api/src/streaming.rs
# Агент читает файл и объясняет реализацию парсера SSE-событий.
#
# > Найди все места, где используется ApiError::Timeout
# Агент запускает grep по всему проекту и показывает список с контекстом.
#
# > Добавь unit-тест для случая когда SSE-строка не содержит поле data
# Агент читает существующие тесты, пишет новый и предлагает добавить его в файл.
```

**Смена модели без перезапуска:**

```
/model llama3.1          # переключиться на другую модель
/model qwen2.5-coder:7b  # вернуться обратно
/cost                    # посмотреть расход токенов по текущей сессии
```

### LM Studio

[LM Studio](https://lmstudio.ai) предоставляет GUI для загрузки и запуска GGUF-моделей с встроенным OpenAI-совместимым сервером.

**Запуск сервера:**

1. Откройте LM Studio
2. Перейдите в раздел **Local Server** (значок `<->` в боковом меню)
3. Выберите загруженную модель
4. Нажмите **Start Server** (порт по умолчанию: `1234`)

**Настройка Claude Code RS:**

```bash
export OPENAI_BASE_URL=http://localhost:1234
export OPENAI_API_KEY=lm-studio  # Любое непустое значение
export OPENAI_MODEL=local-model  # Имя модели как отображается в LM Studio

./target/release/claude-code
```

**Эндпоинт:** `http://localhost:1234/v1/chat/completions`

### vLLM

[vLLM](https://github.com/vllm-project/vllm) — высокопроизводительный inference-сервер с постраничным вниманием (PagedAttention). Подходит для GPU-серверов и multi-GPU сетапов.

**Запуск сервера:**

```bash
pip install vllm

# Запуск с конкретной моделью (требует GPU)
python -m vllm.entrypoints.openai.api_server \
    --model meta-llama/Meta-Llama-3.1-8B-Instruct \
    --port 8000 \
    --api-key my-secret-key

# Для моделей с инструментами (tool use)
python -m vllm.entrypoints.openai.api_server \
    --model Qwen/Qwen2.5-72B-Instruct \
    --enable-auto-tool-choice \
    --tool-call-parser hermes
```

**Настройка Claude Code RS:**

```bash
export OPENAI_BASE_URL=http://localhost:8000
export OPENAI_API_KEY=my-secret-key
export OPENAI_MODEL=meta-llama/Meta-Llama-3.1-8B-Instruct

./target/release/claude-code
```

**Эндпоинт:** `http://localhost:8000/v1/chat/completions`

**Примечание по tool use:** Поддержка вызова инструментов зависит от модели и парсера (`--tool-call-parser`). Рекомендуемые парсеры: `hermes` (Qwen, Nous), `llama3_json` (Llama 3.x), `mistral` (Mistral).

### Сводная таблица локальных провайдеров

| Провайдер | Порт по умолчанию | `OPENAI_API_KEY` | Tool Use | Стриминг | Примечание |
|-----------|-------------------|-----------------|----------|----------|------------|
| Ollama | `11434` | Любое непустое | Зависит от модели | Да | Проще всего настроить |
| LM Studio | `1234` | Любое непустое | Зависит от модели | Да | GUI, GGUF-модели |
| vLLM | `8000` | Задаётся при запуске | Да (с флагом) | Да | Высокая пропускная способность |

### Поведение при ограничениях локальных моделей

Следующие возможности используются только с провайдером Anthropic Direct и тихо пропускаются для OpenAI-совместимых локальных моделей:

- **Extended Thinking** (`thinking` блоки) — игнорируется
- **Cache Control** (`cache_control` в сообщениях) — игнорируется
- **Prompt Caching** — не поддерживается

---

## Облачные провайдеры

### Anthropic Direct (по умолчанию)

Прямое подключение к Anthropic API. Поддерживает все возможности: стриминг, Extended Thinking, Prompt Caching, tool use.

```bash
export ANTHROPIC_API_KEY=sk-ant-...

./target/release/claude-code
./target/release/claude-code -m opus     # claude-opus-4-20250514
./target/release/claude-code -m sonnet   # claude-sonnet-4-20250514
./target/release/claude-code -m haiku    # claude-haiku-4-5-20251001
```

**Аутентификация:** `x-api-key` заголовок (API key) или `Authorization: Bearer` (OAuth token).

**Версия API:** `anthropic-version: 2023-06-01` (фиксирована в заголовке).

**Эндпоинт:** `https://api.anthropic.com/v1/messages`

### AWS Bedrock

Доступ к моделям Claude через Amazon Bedrock. Аутентификация выполняется через AWS Signature V4.

**Требования:**
- AWS-аккаунт с включённым доступом к Bedrock
- IAM-разрешения: `bedrock:InvokeModelWithResponseStream`
- Включённый доступ к модели в разделе **Model Access** консоли Bedrock

**Настройка:**

```bash
export CLAUDE_CODE_USE_BEDROCK=1
export AWS_REGION=us-east-1   # регион с доступом к моделям Claude

./target/release/claude-code -m anthropic.claude-sonnet-4-20250514-v1:0
```

**Дополнительные переменные окружения:**

```bash
# Явные учётные данные (если не настроен AWS CLI)
export AWS_ACCESS_KEY_ID=AKIA...
export AWS_SECRET_ACCESS_KEY=...
export AWS_SESSION_TOKEN=...    # для временных учётных данных STS
```

**Формат идентификаторов моделей Bedrock:**

| Модель | Bedrock Model ID |
|--------|-----------------|
| Claude Sonnet 4 | `anthropic.claude-sonnet-4-20250514-v1:0` |
| Claude Haiku 3.5 | `anthropic.claude-3-5-haiku-20241022-v1:0` |
| Claude Sonnet 3.7 | `anthropic.claude-3-7-sonnet-20250219-v1:0` |

**Эндпоинт:** `https://bedrock-runtime.{region}.amazonaws.com/model/{model-id}/invoke-with-response-stream`

**Важно:** Текущая реализация AWS Sig V4 является заглушкой (stub). Для production-использования необходима интеграция с реальным процессом подписи запросов (HMAC-SHA256 по заголовкам и телу запроса). Рабочая аутентификация требует корректно настроенных AWS credentials через `~/.aws/credentials` или IAM Role.

### Google Vertex AI

Доступ к моделям Claude через Google Cloud Vertex AI.

**Требования:**
- Google Cloud проект с включённым Vertex AI API
- Доступ к моделям Claude через Vertex AI Model Garden
- Application Default Credentials (ADC)

**Настройка:**

```bash
export CLAUDE_CODE_USE_VERTEX=1
export CLOUD_ML_PROJECT_ID=my-gcp-project
export CLOUD_ML_REGION=us-east5   # регион с поддержкой Claude

# Аутентификация через gcloud (рекомендуется)
gcloud auth application-default login

# Или через переменную окружения (для CI/CD)
export GOOGLE_ACCESS_TOKEN=$(gcloud auth print-access-token)

./target/release/claude-code -m claude-sonnet-4@20250514
```

**Формат идентификаторов моделей Vertex:**

| Модель | Vertex Model ID |
|--------|----------------|
| Claude Sonnet 4 | `claude-sonnet-4@20250514` |
| Claude Haiku 3.5 | `claude-3-5-haiku@20241022` |
| Claude Sonnet 3.7 | `claude-3-7-sonnet@20250219` |

**Эндпоинт:** `https://{region}-aiplatform.googleapis.com/v1/projects/{project}/locations/{region}/publishers/anthropic/models/{model}:streamRawPredict`

**Важно:** Текущая реализация ADC является заглушкой. Если переменная `GOOGLE_ACCESS_TOKEN` не установлена, используется placeholder-токен `ya29.stub-access-token`. Для production используйте `gcloud auth application-default login` или service account key.

### Azure Foundry (Azure AI)

Доступ к моделям Claude через Azure AI Foundry (ранее Azure OpenAI Service).

**Требования:**
- Azure-подписка с развёрнутой моделью Claude
- Azure AD токен или API ключ ресурса

**Настройка:**

```bash
export CLAUDE_CODE_USE_FOUNDRY=1
export AZURE_FOUNDRY_BASE_URL=https://my-resource.openai.azure.com
export AZURE_FOUNDRY_RESOURCE=my-claude-deployment

# Аутентификация через Azure AD (рекомендуется)
export AZURE_AD_TOKEN=$(az account get-access-token --query accessToken -o tsv)

./target/release/claude-code
```

**Эндпоинт:** `{base_url}/openai/deployments/{resource}/messages`

**Версия API:** `api-version: 2024-06-01` (фиксирована в заголовке).

**Важно:** Текущая реализация Azure AD токена является заглушкой. Если `AZURE_AD_TOKEN` не установлена, используется placeholder JWT. Для production-использования установите реальный токен через `az account get-access-token` или managed identity.

### Сводная таблица облачных провайдеров

| Провайдер | Переменная активации | Аутентификация | Статус реализации |
|-----------|---------------------|---------------|------------------|
| Anthropic Direct | — (по умолчанию) | `ANTHROPIC_API_KEY` | Полная |
| AWS Bedrock | `CLAUDE_CODE_USE_BEDROCK=1` | AWS Sig V4 (+ `AWS_ACCESS_KEY_ID`) | Stub подписи |
| Google Vertex AI | `CLAUDE_CODE_USE_VERTEX=1` | OAuth2 ADC / `GOOGLE_ACCESS_TOKEN` | Stub ADC |
| Azure Foundry | `CLAUDE_CODE_USE_FOUNDRY=1` | Azure AD / `AZURE_AD_TOKEN` | Stub токена |
| OpenAI-совместимые | — | `OPENAI_API_KEY` + `OPENAI_BASE_URL` | Полная |

## Configuration Reference

Полный список переменных окружения. Все булевы флаги принимают значения `1`, `true` или `yes`.

### Anthropic Direct

| Переменная | По умолчанию | Описание |
|-----------|-------------|----------|
| `ANTHROPIC_API_KEY` | — | API ключ (обязателен для Direct провайдера) |
| `ANTHROPIC_AUTH_TOKEN` / `CLAUDE_AUTH_TOKEN` | — | OAuth токен вместо API ключа |
| `CLAUDE_CODE_OAUTH_TOKEN` | — | OAuth токен (альтернативный алиас) |
| `ANTHROPIC_BASE_URL` | `https://api.anthropic.com` | Кастомный эндпоинт |
| `ANTHROPIC_MODEL` / `CLAUDE_MODEL` | — | Модель по умолчанию |

### Выбор провайдера

Провайдер определяется автоматически по наличию переменных. Порядок приоритета: Bedrock → Vertex → Foundry → OpenAI-compatible → Direct.

| Переменная | Описание |
|-----------|----------|
| `CLAUDE_CODE_USE_BEDROCK` | Включить AWS Bedrock |
| `CLAUDE_CODE_USE_VERTEX` | Включить Google Vertex AI |
| `CLAUDE_CODE_USE_FOUNDRY` | Включить Azure Foundry |
| `CLAUDE_CODE_USE_OPENAI` | Явно включить OpenAI-compatible |

### AWS Bedrock

| Переменная | По умолчанию | Описание |
|-----------|-------------|----------|
| `AWS_REGION` / `AWS_DEFAULT_REGION` | `us-east-1` | Регион |
| `AWS_PROFILE` | — | AWS credentials profile |
| `AWS_ACCESS_KEY_ID` | — | Access key (если не через `~/.aws/credentials`) |
| `AWS_SECRET_ACCESS_KEY` | — | Secret key |
| `AWS_SESSION_TOKEN` | — | Session token для временных credentials |
| `ANTHROPIC_BEDROCK_BASE_URL` | — | Кастомный Bedrock эндпоинт |

### Google Vertex AI

| Переменная | По умолчанию | Описание |
|-----------|-------------|----------|
| `CLOUD_ML_PROJECT_ID` / `GOOGLE_CLOUD_PROJECT` | — | GCP Project ID |
| `CLOUD_ML_REGION` | `us-central1` | Регион |
| `GOOGLE_ACCESS_TOKEN` | — | OAuth2 access token (иначе используется ADC stub) |
| `ANTHROPIC_VERTEX_BASE_URL` | — | Кастомный Vertex эндпоинт |

### Azure Foundry

| Переменная | По умолчанию | Описание |
|-----------|-------------|----------|
| `AZURE_FOUNDRY_BASE_URL` | — | Base URL ресурса (обязателен) |
| `AZURE_FOUNDRY_RESOURCE` | `claude-deployment` | Имя деплоя |
| `AZURE_AD_TOKEN` | — | Azure AD токен |

### OpenAI-совместимые

| Переменная | По умолчанию | Описание |
|-----------|-------------|----------|
| `OPENAI_API_KEY` | — | API ключ |
| `OPENAI_BASE_URL` / `OPENAI_API_BASE` | `https://api.openai.com` | Базовый URL |
| `OPENAI_MODEL` | — | Модель по умолчанию |

### Функциональность

| Переменная | По умолчанию | Описание |
|-----------|-------------|----------|
| `CLAUDE_MAX_THINKING_TOKENS` | — | Лимит токенов для Extended Thinking |
| `CLAUDE_MAX_OUTPUT_TOKENS` | — | Лимит токенов ответа |
| `CLAUDE_BASH_DEFAULT_TIMEOUT_MS` | — | Таймаут Bash по умолчанию (мс) |
| `CLAUDE_BASH_MAX_TIMEOUT_MS` | — | Максимальный таймаут Bash (мс) |
| `CLAUDE_BASH_MAX_OUTPUT_LENGTH` | — | Максимальная длина вывода Bash (символы) |
| `CLAUDE_CODE_TURN_TIMEOUT` | `0` (выкл) | Idle-таймаут на один turn (секунды) |
| `SEARXNG_URL` | `https://searx.be` | URL SearXNG для веб-поиска |

### Поведение

| Переменная | По умолчанию | Описание |
|-----------|-------------|----------|
| `CLAUDE_DISABLE_TELEMETRY` / `DISABLE_TELEMETRY` | `false` | Отключить телеметрию |
| `CLAUDE_CODE_ENABLE_TELEMETRY` | `false` | Явно включить телеметрию |
| `CLAUDE_DISABLE_ERROR_REPORTING` | `false` | Отключить отчёты об ошибках |
| `CLAUDE_DISABLE_AUTO_UPDATER` | `false` | Отключить автообновление |
| `CLAUDE_SIMPLE_MODE` | `false` | Упрощённый режим вывода |
| `CLAUDE_CONFIG_DIR` | `~/.claude` | Кастомная директория конфигурации |
| `CI` / `CLAUDE_CI` | `false` | Признак CI среды |
| `CLAUDE_SANDBOX` / `SANDBOX` | `false` | Sandbox режим |

### MCP аутентификация

Переменные вида `MCP_{NAME}_API_KEY` и `MCP_{NAME}_BEARER_TOKEN`, где `{NAME}` — имя сервера в верхнем регистре.

```bash
# Пример для сервера с именем "github"
export MCP_GITHUB_API_KEY=ghp_...
export MCP_GITHUB_BEARER_TOKEN=...
```

---

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
