# LLM Gateway Design

## Overview

A production-grade LLM gateway built in Rust that provides unified OpenAI and Anthropic compatible API endpoints, proxies requests to multiple upstream LLM providers (official and third-party compatible), with per-key authentication, per-key-per-model rate limiting, token/request-based billing, full audit logging, and a React-based management dashboard.

## Requirements Summary

| Dimension | Decision |
|---|---|
| Language | Rust |
| Endpoints | OpenAI compatible (`/v1/chat/completions`) + Anthropic compatible (`/v1/messages`) |
| Upstream Providers | Official + third-party compatible for both protocols |
| Auth | API Keys (SHA-256 hashed) |
| Billing | Token-based + request-based |
| Storage | SQLite default, abstracted for PostgreSQL |
| Rate Limiting | Per API Key + Per Model |
| Audit Logging | Full request/response body |
| Content Filtering | Not included |
| Deployment | Single binary + TOML config |
| Management | HTTP management API + React SPA (embedded) |
| Streaming | SSE pass-through |

## Architecture

### Layered Architecture

```
┌─────────────┐     ┌──────────────────────────┐
│  React SPA  │────▶│      Management API       │
└─────────────┘     │  (key mgmt / usage / logs) │
                    └──────────┬───────────────┘
                               │
┌──────────────────────────────────────────────┐
│                  Gateway Core                  │
│                                              │
│  ┌─────────┐  ┌──────────┐  ┌────────────┐  │
│  │  Auth   │─▶│ Router   │─▶│  Upstream   │  │
│  │ (Keys)  │  │ (Model)  │  │  Providers  │  │
│  └─────────┘  └──────────┘  └────────────┘  │
│       │             │              │         │
│  ┌─────────┐  ┌──────────┐  ┌────────────┐  │
│  │RateLimit│  │ Billing  │  │   Audit    │  │
│  └─────────┘  └──────────┘  │   Logger   │  │
│                              └────────────┘  │
│                    ┌──────────────┐          │
│                    │ Storage Layer│          │
│                    │(SQLite/PG)   │          │
│                    └──────────────┘          │
└──────────────────────────────────────────────┘
```

### Project Structure

```
llm-gateway/
├── Cargo.toml
├── config.toml
├── crates/
│   ├── gateway/                 # Main entry point, assembles modules
│   │   └── src/
│   │       └── main.rs
│   ├── api/                     # HTTP layer (Axum)
│   │   └── src/
│   │       ├── openai.rs        # /v1/chat/completions endpoint
│   │       ├── anthropic.rs     # /v1/messages endpoint
│   │       ├── management.rs    # Management API
│   │       └── middleware.rs     # Auth, RateLimit middleware
│   ├── provider/                # Upstream provider abstraction
│   │   └── src/
│   │       ├── mod.rs           # Provider trait
│   │       ├── openai.rs        # OpenAI compatible provider
│   │       └── anthropic.rs     # Anthropic compatible provider
│   ├── auth/                    # API Key authentication
│   │   └── src/
│   ├── ratelimit/               # Rate limiting (per key + per model)
│   │   └── src/
│   ├── billing/                 # Billing (token + request)
│   │   └── src/
│   ├── audit/                   # Audit logging (full request/response)
│   │   └── src/
│   └── storage/                 # Storage abstraction (SQLite / PostgreSQL)
│       └── src/
├── migrations/
├── web/                         # React + TypeScript frontend
│   ├── package.json
│   └── src/
└── docs/
```

### Core Dependencies

| Crate | Purpose |
|---|---|
| `axum` | HTTP framework |
| `tokio` | Async runtime |
| `reqwest` | Upstream HTTP requests + SSE streaming |
| `sqlx` | Database access (compile-time SQL checking) |
| `serde` / `serde_json` | Serialization |
| `toml` | Config file parsing |
| `tracing` | Logging and observability |

## API Endpoints

### Proxy Endpoints (for downstream clients)

**OpenAI Compatible:**
- `POST /v1/chat/completions` — Chat completion (supports `stream: true`)
- `GET /v1/models` — List available models

**Anthropic Compatible:**
- `POST /v1/messages` — Messages (supports `stream: true`)
- `GET /v1/models` — List available models (shared endpoint)

Models list is dynamically aggregated based on the API key's permissions and configured providers.

### Management API (`/api/v1/...`)

Auth: `Authorization: Bearer <admin_token>` from config.

**API Key Management:**
- `POST /api/v1/keys` — Create key
- `GET /api/v1/keys` — List all keys
- `GET /api/v1/keys/:id` — Get key details
- `PATCH /api/v1/keys/:id` — Update key (name, rate limit, budget)
- `DELETE /api/v1/keys/:id` — Disable/delete key

**Provider Management:**
- `POST /api/v1/providers` — Create provider
- `GET /api/v1/providers` — List providers
- `GET /api/v1/providers/:id` — Get provider details
- `PATCH /api/v1/providers/:id` — Update provider
- `DELETE /api/v1/providers/:id` — Delete provider
- `POST /api/v1/providers/:id/models` — Add model to provider
- `PATCH /api/v1/providers/:id/models/:model_name` — Update model config
- `DELETE /api/v1/providers/:id/models/:model_name` — Remove model

**Usage & Billing:**
- `GET /api/v1/usage` — Usage stats (filter by key/model/time range)
- `GET /api/v1/usage/:key_id` — Single key usage detail

**Audit Logs:**
- `GET /api/v1/logs` — Query logs (paginated, filter by key/model/time)

Rate limit config is set on API keys via the key management API. No separate CRUD for rate limits.

## Provider Data Model

A single provider can support both OpenAI and Anthropic protocols with different base URLs.

```
Provider
├── id
├── name                  # "MiniMax"
├── api_key               # Upstream API key (encrypted at rest)
├── openai_base_url       # "https://api.minimax.chat/v1" (optional)
├── anthropic_base_url    # "https://api.minimax.chat/v1/anthropic" (optional)
├── models[]
│   ├── name              # "minimax-m2.5"
│   ├── billing_type      # "token" | "request"
│   ├── input_price       # Per 1M tokens (for token billing)
│   └── output_price      # Per 1M tokens (for token billing)
└── enabled
```

**Routing logic:**
- Request to `/v1/chat/completions` + model `minimax-m2.5` → find model → get provider → forward via `openai_base_url`
- Request to `/v1/messages` + model `minimax-m2.5` → find model → get provider → forward via `anthropic_base_url`

## Request Processing Flow

```
Client Request
    │
    ▼
┌─────────────┐     fail     ┌──────────┐
│   Auth      │─────────────▶│ 401/403  │
│  Middleware  │              └──────────┘
└──────┬──────┘
       │ pass
       ▼
┌─────────────┐     limited  ┌──────────┐
│ Rate Limit  │─────────────▶│ 429      │
│  Middleware  │              └──────────┘
└──────┬──────┘
       │ pass
       ▼
┌─────────────┐
│   Router    │  Parse model name → find provider + protocol + base_url
└──────┬──────┘
       │
       ▼
┌─────────────────┐
│  Request Transform │  Convert request body for upstream (if needed)
└──────┬────────────┘
       │
       ▼
┌─────────────────┐     fail     ┌──────────────┐
│  Upstream Proxy  │─────────────▶│ 502          │
│  (reqwest)       │              └──────────────┘
└──────┬────────────┘
       │ success
       ▼
┌─────────────────┐
│  Response Stream │  SSE: pass-through chunks to client
│  (non-SSE: full) │  Non-streaming: buffer full response, then forward
└──────┬────────────┘
       │
       ▼
┌─────────────────┐
│  Post-Process    │  Async (non-blocking):
│  (async)         │  1. Write audit log (full request/response)
│                  │  2. Calculate cost and write billing record
└─────────────────┘
```

### Streaming (SSE)

```
Client ◀──SSE──▶ Gateway ◀──SSE──▶ Upstream Provider
```

- Gateway pass-throughs SSE events without buffering the full response.
- Extracts `usage` fields from events (when present) for billing.
- After stream ends (`[DONE]` signal), triggers async audit log and billing write.

### Non-Streaming

- Waits for complete upstream response.
- Records full request/response to audit log.
- Extracts `usage` info for billing.
- Forwards response to client.

### Upstream Error Handling

- Upstream 4xx: pass through error code and message to client.
- Upstream 5xx or network timeout: return 502, no retry (avoid duplicate billing).
- No response: 30s timeout, return 504.

### Provider Adapter Trait

```rust
trait ProviderAdapter {
    fn transform_request(&self, req: &mut Request) {}
    fn transform_response(&self, res: &mut Response) {}
}
```

Default is pass-through. Specific providers can override (e.g., Azure OpenAI needs `api-version` query param).

## Storage Layer

### Storage Trait

```rust
#[async_trait]
trait Storage: Send + Sync {
    // API Keys
    async fn create_key(&self, key: ApiKey) -> Result<ApiKey>;
    async fn get_key(&self, id: &str) -> Result<Option<ApiKey>>;
    async fn get_key_by_hash(&self, hash: &str) -> Result<Option<ApiKey>>;
    async fn list_keys(&self) -> Result<Vec<ApiKey>>;
    async fn update_key(&self, key: ApiKey) -> Result<ApiKey>;
    async fn delete_key(&self, id: &str) -> Result<()>;

    // Providers
    async fn create_provider(&self, provider: Provider) -> Result<Provider>;
    async fn get_provider(&self, id: &str) -> Result<Option<Provider>>;
    async fn list_providers(&self) -> Result<Vec<Provider>>;
    async fn update_provider(&self, provider: Provider) -> Result<Provider>;
    async fn delete_provider(&self, id: &str) -> Result<()>;

    // Models
    async fn get_model(&self, name: &str) -> Result<Option<Model>>;
    async fn list_models(&self) -> Result<Vec<ModelWithProvider>>;

    // Usage / Billing
    async fn record_usage(&self, usage: UsageRecord) -> Result<()>;
    async fn query_usage(&self, filter: UsageFilter) -> Result<Vec<UsageRecord>>;

    // Audit Logs
    async fn insert_log(&self, log: AuditLog) -> Result<()>;
    async fn query_logs(&self, filter: LogFilter) -> Result<Vec<AuditLog>>;

    // Rate Limit
    async fn check_rate_limit(&self, key_id: &str, model: &str) -> Result<bool>;
    async fn increment_rate_limit(&self, key_id: &str, model: &str) -> Result<()>;
}
```

Two implementations: `SqliteStorage` and `PostgresStorage`, selected via config.

### Database Schema

```sql
-- API Keys
CREATE TABLE api_keys (
    id           TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    key_hash     TEXT NOT NULL UNIQUE,
    rate_limit   INTEGER,
    budget_monthly REAL,
    enabled      BOOLEAN NOT NULL DEFAULT true,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);

-- Providers
CREATE TABLE providers (
    id                 TEXT PRIMARY KEY,
    name               TEXT NOT NULL,
    api_key            TEXT NOT NULL,
    openai_base_url    TEXT,
    anthropic_base_url TEXT,
    enabled            BOOLEAN NOT NULL DEFAULT true,
    created_at         TEXT NOT NULL,
    updated_at         TEXT NOT NULL
);

-- Models
CREATE TABLE models (
    name          TEXT PRIMARY KEY,
    provider_id   TEXT NOT NULL REFERENCES providers(id),
    billing_type  TEXT NOT NULL CHECK(billing_type IN ('token', 'request')),
    input_price   REAL NOT NULL DEFAULT 0,
    output_price  REAL NOT NULL DEFAULT 0,
    request_price REAL NOT NULL DEFAULT 0,
    enabled       BOOLEAN NOT NULL DEFAULT true,
    created_at    TEXT NOT NULL
);

-- Per-key per-model rate limits
CREATE TABLE key_model_rate_limits (
    key_id     TEXT NOT NULL REFERENCES api_keys(id),
    model_name TEXT NOT NULL REFERENCES models(name),
    rpm        INTEGER NOT NULL,
    tpm        INTEGER NOT NULL,
    PRIMARY KEY (key_id, model_name)
);

-- Usage Records
CREATE TABLE usage_records (
    id            TEXT PRIMARY KEY,
    key_id        TEXT NOT NULL REFERENCES api_keys(id),
    model_name    TEXT NOT NULL,
    provider_id   TEXT NOT NULL,
    protocol      TEXT NOT NULL,
    input_tokens  INTEGER,
    output_tokens INTEGER,
    cost          REAL NOT NULL,
    created_at    TEXT NOT NULL
);
CREATE INDEX idx_usage_key_date ON usage_records(key_id, created_at);
CREATE INDEX idx_usage_model_date ON usage_records(model_name, created_at);

-- Audit Logs
CREATE TABLE audit_logs (
    id            TEXT PRIMARY KEY,
    key_id        TEXT NOT NULL REFERENCES api_keys(id),
    model_name    TEXT NOT NULL,
    provider_id   TEXT NOT NULL,
    protocol      TEXT NOT NULL,
    request_body  TEXT NOT NULL,
    response_body TEXT NOT NULL,
    status_code   INTEGER NOT NULL,
    latency_ms    INTEGER NOT NULL,
    input_tokens  INTEGER,
    output_tokens INTEGER,
    created_at    TEXT NOT NULL
);
CREATE INDEX idx_audit_key_date ON audit_logs(key_id, created_at);
CREATE INDEX idx_audit_model_date ON audit_logs(model_name, created_at);

-- Rate Limit Counters
CREATE TABLE rate_limit_counters (
    key_id     TEXT NOT NULL,
    model_name TEXT NOT NULL,
    window     TEXT NOT NULL,
    count      INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (key_id, model_name, window)
);
```

### Rate Limiting Strategy

- **Hot path**: In-memory sliding window counter per key+model combination.
- **Persistence**: Flush counters to DB every 30s. Recover from DB on gateway restart.
- **Priority**: `key_model_rate_limits` table overrides the global `rate_limit` on `api_keys`.

## Configuration

Config file (`config.toml`) handles infrastructure only. All business data (keys, providers, models, rate limits) managed via Management API / Web UI.

```toml
[server]
host = "0.0.0.0"
port = 8080

[admin]
token = "admin-secret-token"

[database]
driver = "sqlite"
sqlite_path = "./data/gateway.db"
# url = "postgres://user:pass@localhost:5432/llm_gateway"

[rate_limit]
flush_interval = 30
window_size = 60

[upstream]
timeout = 30

[audit]
retention_days = 90
```

### Startup Flow

1. Read `config.toml`, start server.
2. Auto-run database migrations (create tables).
3. Gateway ready. Add first provider and key via Management API.

## Web Management UI

### Pages

```
Web UI
├── Dashboard
│   ├── Today's requests / total requests
│   ├── Today's cost / monthly cost
│   ├── Model request distribution chart
│   └── Recent request logs (latest 20)
│
├── API Keys
│   ├── Key list (name, status, monthly usage, monthly cost)
│   ├── Create Key
│   └── Key detail/edit
│       ├── Basic info (name, enabled toggle)
│       ├── Rate limit config (global RPM, per-model RPM/TPM)
│       ├── Monthly budget
│       └── Usage trend chart (by day/week/month)
│
├── Providers
│   ├── Provider list (name, protocol tags, model count, status)
│   ├── Add Provider
│   └── Provider detail/edit
│       ├── Basic info (name, API key, base URLs)
│       ├── Model management (add/edit/delete models, set pricing)
│       └── Status toggle
│
├── Usage
│   ├── Time range filter
│   ├── Aggregate by API Key / Model / Provider
│   ├── Cost summary table
│   └── Trend charts (requests, cost, token usage)
│
└── Logs
    ├── Filter by time / key / model / status code
    ├── Paginated browsing
    └── Click to view full request/response JSON
```

### Frontend Tech Stack

| Item | Choice |
|---|---|
| Framework | React 18 + TypeScript |
| Build | Vite |
| Routing | React Router |
| State/Data | React Query |
| UI Components | Ant Design |
| Charts | ECharts / Recharts |
| API Client | Axios + OpenAPI generated types |

### Frontend-Backend Integration

- Frontend builds to static files.
- Gateway serves static files via Axum `ServeDir` at `/admin/`.
- During development: `vite dev` proxies API calls to gateway backend.
- Final deployment: single binary with embedded management UI.
