# LLM Gateway

A unified API gateway for LLM providers with OpenAI and Anthropic compatible endpoints, multi-provider routing, per-key rate limiting, configurable pricing policies, and a React management dashboard.

## Features

- **Dual protocol proxy** — `/v1/chat/completions` (OpenAI) and `/v1/messages` (Anthropic) compatible
- **Multi-provider routing** — route requests through provider channels, each with its own API key and priority
- **Model ↔ Provider N:N routing** — models route through `channel_models` junction; a model can be served by multiple provider channels
- **Pricing policies** — 5 billing types: per-token, per-request, per-character, tiered-token, hybrid
- **Per-key rate limiting** — global and per-model RPM/TPM limits with in-memory sliding window
- **API key management** — create, rotate, enable/disable; keys stored as SHA-256 hashes; channel API keys encrypted at rest
- **Usage tracking** — per-request input/output/cache tokens, cost calculation, paginated and aggregated summaries
- **Audit logging** — full request/response bodies, latency, token counts, upstream URL; configurable retention
- **Streaming support** — SSE pass-through for streaming responses
- **User management** — admin/user roles, JWT auth, registration with first-user-becomes-admin
- **Web dashboard** — Keys, Usage, Channels, Providers, Models, Pricing Policies, Users, Settings, Audit Logs

## Architecture

```
┌──────────┐     ┌──────────────────────────────────────┐     ┌──────────────┐
│  Client  │────▶│            LLM Gateway               │────▶│  Upstream    │
│  (SDK)   │     │  Auth → Rate Limit → Route → Proxy  │     │  Providers   │
└──────────┘     └──────────────────────────────────────┘     └──────────────┘
                        │              │
                    ┌───┴──┐       ┌────┴────┐
                    │SQLite │       │  React  │
                    │/PgSQL │       │Dashboard│
                    └───────┘       └─────────┘
```

Built as a Rust workspace with 9 crates:

| Crate | Responsibility |
|-------|---------------|
| `gateway` | Server bootstrap, config, assembles all modules |
| `api` | HTTP handlers (Axum), middleware, routing |
| `storage` | SQLite/PostgreSQL storage trait + migrations |
| `auth` | API key generation, SHA-256 verification, JWT + refresh tokens |
| `ratelimit` | In-memory sliding window rate limiter |
| `billing` | Token/request cost calculation using typed pricing configs |
| `audit` | Async request audit logging, cost calculation |
| `provider` | Provider trait and upstream proxy (OpenAI + Anthropic adapters) |
| `encryption` | API key encryption at rest for channel credentials |

**Request flow:** Auth (API key) → Rate Limit → Router (model → channel + protocol) → Upstream Proxy → Response (streaming SSE or buffered) → Async audit + billing write.

**Data model:** `Model` ↔ `channel_models` (N:N) ↔ `Channel` → `Provider` (1:N). Both `Model` and `ChannelModel` reference a `PricingPolicy`.

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
encryption_key = "your-32-byte-hex-encoded-key-here"

[auth]
jwt_secret = "your-jwt-secret-min-32-chars"
allow_registration = true  # default: true

[database]
driver = "sqlite"   # or "postgres"
sqlite_path = "./data/gateway.db"
# url = "postgresql://user:pass@localhost/gateway"  # for postgres

[rate_limit]
flush_interval_secs = 30
window_size_secs = 60

[upstream]
timeout_secs = 30

[audit]
retention_days = 90
```

## API Reference

### Authentication

Proxy requests use an API key: `Authorization: Bearer <api-key>`.

Management endpoints use a JWT token (from `/api/v1/auth/login`): `Authorization: Bearer <jwt-token>`.

### Proxy Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/v1/chat/completions` | OpenAI compatible chat completions |
| GET | `/v1/models` | Available models (OpenAI format) |
| POST | `/v1/messages` | Anthropic compatible messages |
| GET | `/v1/models` | Available models (Anthropic format) |

### Management API

All management endpoints require `Authorization: Bearer <jwt-token>` (admin role for most endpoints).

#### Auth

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/auth/login` | Login, returns JWT |
| POST | `/api/v1/auth/register` | Register (first user becomes admin) |
| GET | `/api/v1/auth/me` | Current user info |
| POST | `/api/v1/auth/refresh` | Refresh JWT |
| POST | `/api/v1/auth/change-password` | Change password |

#### API Keys

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/keys` | List API keys |
| POST | `/api/v1/keys` | Create API key |
| GET | `/api/v1/keys/{id}` | Get API key |
| PATCH | `/api/v1/keys/{id}` | Update API key |
| DELETE | `/api/v1/keys/{id}` | Delete API key |

#### Providers

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/admin/providers` | List providers |
| POST | `/api/v1/admin/providers` | Create provider |
| GET | `/api/v1/admin/providers/{id}` | Get provider |
| PATCH | `/api/v1/admin/providers/{id}` | Update provider |
| DELETE | `/api/v1/admin/providers/{id}` | Delete provider |

#### Channels

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/admin/providers/{id}/channels` | List channels for a provider |
| POST | `/api/v1/admin/providers/{id}/channels` | Create channel for a provider |
| GET | `/api/v1/admin/channels` | List all channels |
| POST | `/api/v1/admin/channels` | Create channel |
| GET | `/api/v1/admin/channels/{id}` | Get channel |
| PATCH | `/api/v1/admin/channels/{id}` | Update channel |
| DELETE | `/api/v1/admin/channels/{id}` | Delete channel |
| PATCH | `/api/v1/admin/channels/{id}/api-key` | Update channel API key |

#### Models

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/admin/models` | List all models |
| POST | `/api/v1/admin/models` | Create global model |
| PATCH | `/api/v1/admin/models/{name}` | Update model |
| DELETE | `/api/v1/admin/models/{name}` | Delete model |

#### Channel Models (Model ↔ Channel Mappings)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/admin/providers/{id}/channel-models` | Mappings for a provider |
| POST | `/api/v1/admin/providers/{id}/channel-models` | Create mapping |
| GET | `/api/v1/admin/channels/{id}/channel-models` | Mappings for a channel |
| POST | `/api/v1/admin/channels/{id}/channel-models` | Create mapping |
| GET | `/api/v1/admin/channel-models/{id}` | Get mapping |
| PATCH | `/api/v1/admin/channel-models/{id}` | Update mapping |
| DELETE | `/api/v1/admin/channel-models/{id}` | Delete mapping |

#### Pricing Policies

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/admin/pricing-policies` | List pricing policies |
| POST | `/api/v1/admin/pricing-policies` | Create pricing policy |
| GET | `/api/v1/admin/pricing-policies/{id}` | Get pricing policy |
| PATCH | `/api/v1/admin/pricing-policies/{id}` | Update pricing policy |
| DELETE | `/api/v1/admin/pricing-policies/{id}` | Delete pricing policy |

Billing types: `per_token`, `per_request`, `per_character`, `tiered_token`, `hybrid`.

Config is a JSON object. Per-token example:
```json
{
  "input_per_1m": 3.0,
  "output_per_1m": 15.0,
  "cache_read_price": 1.0
}
```
Prices are in dollars per million tokens.

#### Users

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/admin/users` | List users |
| PATCH | `/api/v1/admin/users/{id}` | Update user (role, enabled) |
| DELETE | `/api/v1/admin/users/{id}` | Delete user |

#### Settings

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/admin/settings` | Get settings |
| PATCH | `/api/v1/admin/settings` | Update settings |

#### Usage

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/usage` | Paginated usage records |
| GET | `/api/v1/usage/summary` | Aggregated usage by model |

#### Audit Logs

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/admin/logs` | Query audit logs |

#### Other

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/admin/seed` | Static seed provider/model definitions |
| GET | `/api/v1/version` | Gateway version |

### Examples

#### Login and create an API key

```bash
# Login
TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "..."}' | jq -r '.token')

# Create API key
curl -X POST http://localhost:8080/api/v1/keys \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "my-app", "rate_limit": 60, "budget_monthly": 100.0}'
```

#### Create a provider and route a model

```bash
# 1. Create provider
curl -X POST http://localhost:8080/api/v1/admin/providers \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "openai", "endpoints": {"openai": "https://api.openai.com/v1"}}'

PROVIDER_ID="..."

# 2. Create channel (holds the upstream API key)
curl -X POST http://localhost:8080/api/v1/admin/providers/$PROVIDER_ID/channels \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "openai-gpt4", "api_key": "sk-...", "priority": 10}'

CHANNEL_ID="..."

# 3. Create model
curl -X POST http://localhost:8080/api/v1/admin/models \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "gpt-4o"}'

MODEL_ID="..."

# 4. Map model to channel
curl -X POST http://localhost:8080/api/v1/admin/channels/$CHANNEL_ID/channel-models \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"model_id": "'$MODEL_ID'", "upstream_model_name": "gpt-4o"}'
```

#### Proxy a chat completion

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer <api-key>" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Hello"}]
  }'
```

## Web Dashboard

The management dashboard is at `/`. It requires JWT authentication.

```bash
cd web
npm ci
npm run build
```

The built files in `web/dist/` are served automatically by the gateway. For development:

```bash
cd web && npm run dev   # frontend on :5173, proxies /api to :8080
```

## Development

```bash
# Backend
cargo test --workspace        # Run all tests
cargo build --release         # Build release binary
cargo run                     # Dev server on :8080

# Frontend
cd web
npm ci                       # Install dependencies
npm run dev                  # Dev server on :5173
npm run build                # TypeScript check + Vite build
npm test                     # Vitest unit tests
```

## License

MIT
