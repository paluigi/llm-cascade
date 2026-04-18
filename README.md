> **⚠️ Early Stage / Alpha Software**
>
> This project is in active early development. The API surface may change, and not all edge cases are handled yet. Contributions are welcome — see [Contributing](#contributing) below. Use at your own risk.

---

# llm-cascade

**Resilient, cascading LLM inference across multiple providers — failover, circuit breaking, and retry cooldowns built in.**

`llm-cascade` is a Rust library and CLI that sends prompts to an ordered list of LLM providers (OpenAI, Anthropic, Google Gemini, Ollama, and any OpenAI-compatible endpoint like Groq or Together). If one provider is rate-limited or down, it automatically falls through to the next, tracks per-entry cooldowns in SQLite, and persists failed prompts as JSON files.

---

## Features

- **Cascading failover** — define ordered provider/model lists; the first successful response wins
- **OpenAI-compatible providers** — point any `openai`-type provider at a custom `base_url` (Groq, Together, Z.AI, vLLM, etc.)
- **Per-entry circuit breaker** — cooldowns tracked per `provider/model` pair in SQLite
- **429-aware backoff** — parses `retry-after` headers; falls back to exponential backoff (30 s base, 1 h cap)
- **Cross-process state** — cooldown state persists across CLI invocations via SQLite
- **Secret management** — OS keyring (via `keyring`) with environment variable fallback
- **Failure persistence** — total cascade failures saved as timestamped `.json` files
- **Full audit log** — every attempt logged with timestamp, status, latency, and token counts
- **Dual interface** — use as a CLI tool or as an async library in your own Rust projects

---

## How It Works

```
┌──────────┐       ┌─────────────────────────────────────────────┐
│  CLI /   │       │              Cascade Engine                  │
│  Library │──────▶│                                             │
│  Caller  │       │  ┌───────────┐  ┌───────────┐  ┌─────────┐ │
└──────────┘       │  │ openai/   │─▶│ anthropic/│─▶│ ollama/ │ │
                   │  │ gpt-4o    │  │ claude…   │  │ llama3  │ │
                   │  └────┬──────┘  └────┬──────┘  └────┬────┘ │
                   │       │              │              │       │
                   │  ┌────▼──────────────▼──────────────▼────┐  │
                   │  │           SQLite Database              │  │
                   │  │  • attempt_log (audit trail)           │  │
                   │  │  • cooldown    (circuit breaker state)  │  │
                   │  └───────────────────────────────────────┘  │
                   │                                             │
                   │  On total failure:                          │
                   │  ┌───────────────────────────────────────┐  │
                    │  │  failed_prompts/cascade_20260414.json │  │
                   │  └───────────────────────────────────────┘  │
                   └─────────────────────────────────────────────┘
```

1. **Load config** from `~/.config/llm-cascade/config.toml` (or a custom path).
2. **Initialize SQLite** — creates `attempt_log` and `cooldown` tables if missing.
3. **Iterate cascade entries** — for each `provider/model` in the named cascade:
   - Check if the entry is on cooldown in the DB → skip if so.
   - Resolve the API key (keyring → env var).
   - Send the `Conversation` to the provider's API.
   - Log the attempt (status, latency, tokens).
   - On success → return `LlmResponse` immediately.
   - On failure → set cooldown (from `retry-after` header or exponential backoff) and continue.
4. **Total failure** → persist the `Conversation` as a `.json` file, return `CascadeError` with the file path.

---

## Installation

### From source

```sh
git clone https://github.com/paluigi/llm-cascade.git
cd llm-cascade
cargo install --path .
```

### As a library dependency

```toml
# Cargo.toml
[dependencies]
llm-cascade = "0.1"
```

> Requires Rust **1.85+** (edition 2024).

---

## Configuration

Run the setup command to scaffold the default configuration:

```sh
llm-cascade setup
```

Or use the interactive wizard to configure providers and cascades step-by-step:

```sh
llm-cascade setup --interactive
```

This creates `~/.config/llm-cascade/config.toml` with sensible defaults. Edit it to customize:

```toml
# ── Provider Definitions ────────────────────────────────────
# Each block defines an endpoint (type, base_url, auth).
# Providers are referenced by name in cascades and can be
# reused with different models — no need to duplicate config.

[providers.openai]
type = "openai"
api_key_service = "openai"          # keyring entry name
api_key_env = "OPENAI_API_KEY"      # env var fallback
# base_url defaults to https://api.openai.com/v1

[providers.anthropic]
type = "anthropic"
api_key_service = "anthropic"
api_key_env = "ANTHROPIC_API_KEY"
# base_url defaults to https://api.anthropic.com

[providers.gemini]
type = "gemini"
api_key_service = "gemini"
api_key_env = "GOOGLE_API_KEY"
# base_url defaults to https://generativelanguage.googleapis.com

[providers.groq]
type = "openai"                     # reuse OpenAI-compatible protocol
base_url = "https://api.groq.com/openai/v1"
api_key_service = "groq"
api_key_env = "GROQ_API_KEY"

[providers.ollama]
type = "ollama"
base_url = "http://localhost:11434"
# No API key needed

# ── Cascades ───────────────────────────────────────────────
# Each entry references a provider by name and specifies a model.
# The same provider can appear multiple times with different models.

[cascades.creative_task]
entries = [
    { provider = "openai", model = "gpt-4o" },
    { provider = "anthropic", model = "claude-sonnet-4-20250514" },
    { provider = "gemini", model = "gemini-2.0-flash" },
]

[cascades.fast_task]
entries = [
    { provider = "ollama", model = "llama3" },
    { provider = "groq", model = "llama-3.3-70b-versatile" },
    { provider = "openai", model = "gpt-4o-mini" },
]

[cascades.resilient_task]
entries = [
    { provider = "openai", model = "gpt-4o" },
    { provider = "openai", model = "gpt-4o-mini" },       # same provider, different model
    { provider = "groq", model = "llama-3.3-70b-versatile" },
    { provider = "anthropic", model = "claude-sonnet-4-20250514" },
    { provider = "ollama", model = "llama3" },
]

# ── Persistence ────────────────────────────────────────────

[database]
path = "~/.local/share/llm-cascade/db.sqlite"

[failure_persistence]
dir = "~/.local/share/llm-cascade/failed_prompts"
```

### Provider types

| Type | Description | Default `base_url` |
|------|-------------|-------------------|
| `openai` | OpenAI Chat Completions API | `https://api.openai.com/v1` |
| `anthropic` | Anthropic Messages API | `https://api.anthropic.com` |
| `gemini` | Google Gemini generateContent API | `https://generativelanguage.googleapis.com` |
| `ollama` | Ollama local inference | `http://localhost:11434` |

Any provider with `type = "openai"` can be pointed at a custom `base_url` to use OpenAI-compatible services such as **Groq**, **Together AI**, **Z.AI**, **vLLM**, **LiteLLM**, etc.

### API Keys

| Method | How it works |
|--------|-------------|
| **OS Keyring** (preferred) | Set via `llm-cascade key set <provider>`. The `api_key_service` field is the keyring entry name. |
| **Environment variable** | Export the variable named in `api_key_env` (e.g., `export OPENAI_API_KEY=sk-...`). |
| **Ollama** | No API key needed for local models. |

The library tries the keyring first and falls back to the environment variable automatically. Use `llm-cascade key list` to check the status of all providers.

---

## CLI Usage

### Subcommands

`llm-cascade` uses subcommands for all operations:

```
llm-cascade run       -C <cascade> -p <prompt>   Run a cascade
llm-cascade setup     [--interactive]              Initialize configuration
llm-cascade key set   <provider>                   Store an API key
llm-cascade key get   <provider> [--show-full]     Retrieve an API key
llm-cascade key list                               Show key status for all providers
llm-cascade key delete <provider>                   Remove an API key
```

### Running a cascade

**Basic prompt:**

```sh
llm-cascade run -C creative_task -p "Write a haiku about Rust"
```

**From a JSON conversation file:**

```sh
llm-cascade run -C creative_task -f conversation.json
```

The JSON file must match the `Conversation` schema:

```json
{
  "messages": [
    { "role": "system", "content": "You are a helpful assistant." },
    { "role": "user", "content": "What is 2 + 2?" }
  ],
  "tools": [
    {
      "name": "get_weather",
      "description": "Get the current weather",
      "parameters": {
        "type": "object",
        "properties": {
          "location": { "type": "string" }
        },
        "required": ["location"]
      }
    }
  ]
}
```

**Custom config path:**

```sh
llm-cascade run -c /path/to/my/config.toml -C fast_task -p "Hello"
```

### Setup

**Default setup** — scaffolds the example config, creates directories, and initializes the database:

```sh
llm-cascade setup
```

**Interactive setup** — wizard for selecting providers, defining cascades, and setting API keys:

```sh
llm-cascade setup --interactive
```

### Key management

**Store an API key** (prompts with hidden input):

```sh
llm-cascade key set openai
```

**Retrieve an API key** (masked by default; use `--show-full` to reveal):

```sh
llm-cascade key get openai
llm-cascade key get openai --show-full
```

**List key status** for all providers (checks both keyring and env vars):

```sh
llm-cascade key list
```

**Delete an API key** from the keyring:

```sh
llm-cascade key delete openai
```

### Output

- **Text responses** are printed to stdout.
- **Tool call responses** are printed as pretty JSON to stdout.
- **Errors** (including `CascadeError` with the `.json` file path) are printed to stderr with exit code 1.

### Verbosity

Control log output via the `RUST_LOG` environment variable:

```sh
RUST_LOG=debug llm-cascade run -C creative_task -p "Hello"
RUST_LOG=llm_cascade=trace llm-cascade run -C creative_task -p "Hello"
```

---

## Library Usage

Use `llm-cascade` as an async library in any Rust project:

```rust
use llm_cascade::{run_cascade, load_config, db, Conversation, Message, MessageRole};

#[tokio::main]
async fn main() {
    let config = load_config(&"config.toml".into()).expect("config");
    let conn = db::init_db(&config.database.path).expect("db");

    let conversation = Conversation::new(vec![
        Message::system("You are a concise assistant."),
        Message::user("What is the capital of France?"),
    ]);

    match run_cascade("creative_task", &conversation, &config, &conn).await {
        Ok(response) => {
            println!("Model: {}", response.model);
            println!("Response: {}", response.text_only());
            if let (Some(in), Some(out)) = (response.input_tokens, response.output_tokens) {
                println!("Tokens: {} in / {} out", in, out);
            }
        }
        Err(e) => {
            eprintln!("Cascade failed: {}", e);
        }
    }
}
```

### With tool definitions

```rust
use llm_cascade::{run_cascade, load_config, db, Conversation, Message, ToolDefinition};
use serde_json::json;

let conversation = Conversation::new(vec![
    Message::user("What's the weather in Tokyo?"),
]).with_tools(vec![
    ToolDefinition {
        name: "get_weather".into(),
        description: "Get current weather for a location".into(),
        parameters: json!({
            "type": "object",
            "properties": {
                "location": { "type": "string" }
            },
            "required": ["location"]
        }),
    },
]);

let response = run_cascade("creative_task", &conversation, &config, &conn).await?;
```

### Key types

| Type | Description |
|------|-------------|
| `Conversation` | Holds `messages: Vec<Message>` and optional `tools: Vec<ToolDefinition>` |
| `Message` | A single message with `role` (`System`/`User`/`Assistant`/`Tool`), `content`, and optional `tool_call_id` |
| `ToolDefinition` | Tool name, description, and JSON Schema parameters |
| `LlmResponse` | Response with `content: Vec<ContentBlock>`, token counts, and model name |
| `ContentBlock` | Either `Text { text }` or `ToolCall { id, name, arguments }` |
| `CascadeError` | Contains cascade name, error message, and absolute path to the persisted `.json` file |
| `ProviderError` | HTTP status, body, optional `retry_after` seconds |

---

## API Reference

### `run_cascade`

```rust
pub async fn run_cascade(
    cascade_name: &str,
    conversation: &Conversation,
    config: &AppConfig,
    conn: &Connection,
) -> Result<LlmResponse, CascadeError>
```

The core entry point. Iterates through the named cascade's provider entries, skipping those on cooldown, and returns the first successful `LlmResponse`.

### `db::init_db`

```rust
pub fn init_db(path: &str) -> Result<Connection, String>
```

Opens (or creates) the SQLite database and ensures the schema exists. Expands `~` in the path.

### `db::log_attempt`

```rust
pub fn log_attempt(
    conn: &Connection,
    cascade_name: &str,
    provider_model: &str,
    http_status: Option<u16>,
    latency_ms: u64,
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
)
```

Inserts a row into the `attempt_log` table.

### `db::is_on_cooldown` / `db::set_cooldown`

```rust
pub fn is_on_cooldown(conn: &Connection, provider_model: &str) -> bool
pub fn set_cooldown(conn: &Connection, provider_model: &str, cooldown_until: &str)
```

Query and update the `cooldown` table. Timestamps are RFC 3339 strings.

### `load_config`

```rust
pub fn load_config(path: &Path) -> Result<AppConfig, String>
```

Reads and parses the TOML configuration file.

---

## Cooldown & Backoff Behavior

| Scenario | Cooldown Duration |
|----------|------------------|
| HTTP 429 with `retry-after` header | Value from header (seconds) |
| HTTP 429 without header | 30 s (doubles per consecutive failure, max 1 h) |
| Other HTTP error (4xx/5xx) | 30 s base, exponential doubling |
| Successful response | No cooldown set |

Cooldowns are **per entry** (e.g., `openai/gpt-4o` can be on cooldown while `openai/gpt-3.5-turbo` stays active) and **persisted in SQLite** so separate CLI invocations share the same state.

---

## Roadmap

- [ ] Streaming response support
- [ ] Configurable per-provider timeouts
- [ ] Token budget limits per cascade
- [ ] Retry with modified parameters (e.g., lower temperature)
- [ ] Prometheus metrics export
- [ ] Web dashboard for cooldown/attempt monitoring
- [ ] Additional native providers (Mistral, Cohere, AWS Bedrock, Azure OpenAI)
- [x] Published crate on crates.io

---

## Contributing

Contributions are welcome! This is an open-source project under the MIT license.

### Getting started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/paluigi/llm-cascade.git`
3. Create a branch: `git checkout -b feature/your-feature`
4. Build and test: `cargo build && cargo clippy -- -D warnings`

### Making changes

- Follow the existing code style (no comments unless necessary, concise naming)
- Ensure `cargo clippy -- -D warnings` passes with zero warnings
- Update this README if you change public API or configuration

### Submitting

1. Push to your fork: `git push origin feature/your-feature`
2. Open a Pull Request against the `main` branch
3. Describe your changes and the motivation behind them

### Reporting issues

Use the [GitHub issue tracker](https://github.com/paluigi/llm-cascade/issues) to report bugs, request features, or ask questions. Please include:

- Rust version (`rustc --version`)
- OS and version
- Minimal reproduction steps
- Relevant log output (with `RUST_LOG=debug`)

---

## License

[MIT](LICENSE) — Copyright (c) 2026 Luigi Palumbo
