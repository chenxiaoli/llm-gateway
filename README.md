# LLM Gateway

A unified API gateway for LLM providers with OpenAI and Anthropic compatible endpoints, API key management, per-key rate limiting, token-based billing, and a web management dashboard.

## Features

- **Dual protocol proxy** — OpenAI (`/v1/chat/completions`) and Anthropic (`/v1/messages`) compatible APIs
- **Multi-provider routing** — route requests to multiple upstream LLM providers
- **API key management** — create, rotate, enable/disable keys with hashed storage
- **Per-key rate limiting** — global and per-model RPM/TPM limits
- **Token billing** — track input/output token usage and cost per request
- **Audit logging** — full request logging with latency and token tracking
- **Streaming support** — SSE pass-through for streaming responses
- **Web dashboard** — React management UI for keys, providers, models, usage, and logs

## Architecture

```
┌──────────┐     ┌─────────────────────────────────────┐     ┌──────────────┐
│  Client   │────▶│           LLM Gateway               │────▶│  Upstream    │
│  (SDK/App)│     │  Auth → Rate Limit → Route → Proxy  │     │  Providers   │
└──────────┘     └─────────────────────────────────────┘     └──────────────┘
                      │              │
                  ┌───┴──┐      ┌────┴────┐
                  │ SQLite│      │  React  │
                  │  DB   │      │Dashboard│
                  └──────┘      └─────────┘
```

Built as a Rust workspace with 8 crates:

| Crate | Responsibility |
|-------|---------------|
| `gateway` | Server bootstrap, config, routing |
| `api` | HTTP handlers, middleware |
| `storage` | SQLite storage trait + migrations |
| `auth` | API key generation and verification (SHA-256) |
| `ratelimit` | In-memory sliding window rate limiter |
| `billing` | Token/request cost calculation |
| `audit` | Request audit logging |
| `provider` | Provider trait and upstream proxy |

## Quick Start

### From binary

Download the latest release from [GitHub Releases](https://github.com/chenxiaoli/llm-gateway/releases).

```bash
# Linux / macOS
chmod +x llm-gateway
./llm-gateway

# Windows
llm-gateway.exe
```

### From source

Requires Rust 1.75+.

```bash
git clone https://github.com/chenxiaoli/llm-gateway.git
cd llm-gateway
cargo build --release
./target/release/llm-gateway
```

### Configuration

Create `config.toml` in the working directory:

```toml
[server]
host = "0.0.0.0"
port = 8080

[admin]
token = "your-secure-admin-token"

[database]
driver = "sqlite"
sqlite_path = "./data/gateway.db"

[rate_limit]
flush_interval_secs = 30
window_size_secs = 60

[upstream]
timeout_secs = 30

[audit]
retention_days = 90
```

## API Reference

### Proxy Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/v1/chat/completions` | OpenAI compatible chat completions |
| GET | `/v1/models` | List available models (OpenAI format) |
| POST | `/v1/messages` | Anthropic compatible messages |
| GET | `/v1/models` | List available models (Anthropic format) |

Authenticate proxy requests with `Authorization: Bearer <api-key>`.

### Management API

All management endpoints require `Authorization: Bearer <admin-token>`.

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/keys` | Create API key |
| GET | `/api/v1/keys` | List API keys |
| GET | `/api/v1/keys/{id}` | Get API key |
| PATCH | `/api/v1/keys/{id}` | Update API key |
| DELETE | `/api/v1/keys/{id}` | Delete API key |
| POST | `/api/v1/providers` | Create provider |
| GET | `/api/v1/providers` | List providers |
| GET | `/api/v1/providers/{id}` | Get provider |
| PATCH | `/api/v1/providers/{id}` | Update provider |
| DELETE | `/api/v1/providers/{id}` | Delete provider |
| POST | `/api/v1/providers/{id}/models` | Add model to provider |
| PATCH | `/api/v1/providers/{id}/models/{name}` | Update model |
| DELETE | `/api/v1/providers/{id}/models/{name}` | Delete model |
| GET | `/api/v1/usage` | Query usage records |
| GET | `/api/v1/logs` | Query audit logs |

### Example: Create an API key

```bash
curl -X POST http://localhost:8080/api/v1/keys \
  -H "Authorization: Bearer your-secure-admin-token" \
  -H "Content-Type: application/json" \
  -d '{"name": "my-app", "rate_limit": 60, "budget_monthly": 100.0}'
```

### Example: Add a provider

```bash
curl -X POST http://localhost:8080/api/v1/providers \
  -H "Authorization: Bearer your-secure-admin-token" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "openai",
    "api_key": "sk-...",
    "openai_base_url": "https://api.openai.com",
    "enabled": true
  }'
```

### Example: Proxy a chat completion

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer <api-key-from-create-step>" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello"}]
  }'
```

## Web Dashboard

The management dashboard is available at `/` when the frontend is built and served. To build it:

```bash
cd web
npm ci
npm run build
```

The built files are in `web/dist/`. Serve them alongside the binary or with a static file server.

## Development

```bash
# Run tests
cargo test --workspace

# Build release binary
cargo build --release

# Run dev server with frontend
cargo run           # backend on :8080
cd web && npm run dev  # frontend on :5173
```

## License

MIT
