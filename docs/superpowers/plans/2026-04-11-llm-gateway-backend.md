# LLM Gateway Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- ]`) syntax for tracking.

**Goal:** Build a production-grade LLM gateway in Rust that proxies OpenAI and Anthropic compatible API requests to multiple upstream providers, with API key auth, per-key-per-model rate limiting, token/request billing, full audit logging, and a management API.

**Architecture:** Layered architecture with separate crates for storage, auth, ratelimit, billing, audit, provider, api, and gateway. Axum handles HTTP, SQLx for database access (SQLite default), reqwest for upstream proxy with SSE streaming. Storage trait abstracts SQLite/PostgreSQL.

**Tech Stack:** Rust, Axum, Tokio, Reqwest, SQLx (SQLite), Serde, TOML, Tracing

---

## File Structure

```
llm-gateway/
├── Cargo.toml                          # Workspace root
├── config.toml                         # Runtime config
├── crates/
│   ├── gateway/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs                 # App bootstrap, router assembly
│   ├── api/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # Re-exports
│   │       ├── openai.rs               # POST /v1/chat/completions, GET /v1/models
│   │       ├── anthropic.rs            # POST /v1/messages
│   │       ├── management/
│   │       │   ├── mod.rs              # Router assembly
│   │       │   ├── keys.rs             # /api/v1/keys CRUD
│   │       │   ├── providers.rs        # /api/v1/providers CRUD
│   │       │   ├── models.rs           # /api/v1/providers/:id/models CRUD
│   │       │   ├── usage.rs            # /api/v1/usage queries
│   │       │   └── logs.rs             # /api/v1/logs queries
│   │       ├── error.rs                # Unified error types
│   │       ├── extractors.rs           # Axum request extractors (auth, etc.)
│   │       └── middleware.rs            # Auth + rate limit middleware
│   ├── provider/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # Re-exports + ProviderAdapter trait
│   │       ├── openai.rs               # OpenAI compatible upstream
│   │       └── anthropic.rs            # Anthropic compatible upstream
│   ├── auth/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs                  # API key hashing + verification
│   ├── ratelimit/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs                  # In-memory sliding window + DB persistence
│   ├── billing/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs                  # Cost calculation (token + request)
│   ├── audit/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs                  # Audit log recording
│   └── storage/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs                  # Storage trait + re-exports
│           ├── types.rs                # Domain types (ApiKey, Provider, Model, etc.)
│           ├── sqlite.rs               # SqliteStorage implementation
│           └── migrations/
│               └── init.sql            # Schema DDL
├── migrations/                         # (symlink or copy from storage crate)
└── tests/
    ├── common/
    │   └── mod.rs                      # Shared test helpers (test DB, etc.)
    ├── test_api_openai.rs
    ├── test_api_anthropic.rs
    ├── test_management_keys.rs
    ├── test_management_providers.rs
    ├── test_ratelimit.rs
    ├── test_billing.rs
    └── test_audit.rs
```

---

## Phase 1: Project Scaffold & Storage Foundation

### Task 1: Initialize Cargo workspace and all crates

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/gateway/Cargo.toml`
- Create: `crates/gateway/src/main.rs`
- Create: `crates/api/Cargo.toml`
- Create: `crates/api/src/lib.rs`
- Create: `crates/provider/Cargo.toml`
- Create: `crates/provider/src/lib.rs`
- Create: `crates/auth/Cargo.toml`
- Create: `crates/auth/src/lib.rs`
- Create: `crates/ratelimit/Cargo.toml`
- Create: `crates/ratelimit/src/lib.rs`
- Create: `crates/billing/Cargo.toml`
- Create: `crates/billing/src/lib.rs`
- Create: `crates/audit/Cargo.toml`
- Create: `crates/audit/src/lib.rs`
- Create: `crates/storage/Cargo.toml`
- Create: `crates/storage/src/lib.rs`

- [ ] **Step 1: Create workspace root Cargo.toml**

```toml
[workspace]
resolver = "2"
members = [
    "crates/gateway",
    "crates/api",
    "crates/provider",
    "crates/auth",
    "crates/ratelimit",
    "crates/billing",
    "crates/audit",
    "crates/storage",
]

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
axum = "0.8"
reqwest = { version = "0.12", features = ["json", "stream"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "postgres", "migrate"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
tracing = "0.1"
tracing-subscriber = "0.3"
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
sha2 = "0.10"
hex = "0.4"
thiserror = "2"
```

- [ ] **Step 2: Create each crate's Cargo.toml and lib.rs/main.rs**

Each crate's Cargo.toml follows this pattern (example for `storage`):

```toml
[package]
name = "llm-gateway-storage"
version = "0.1.0"
edition = "2021"

[dependencies]
sqlx = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true }
```

`gateway/Cargo.toml` depends on all other crates:
```toml
[package]
name = "llm-gateway"
version = "0.1.0"
edition = "2021"

[dependencies]
llm-gateway-api = { path = "../api" }
llm-gateway-provider = { path = "../provider" }
llm-gateway-auth = { path = "../auth" }
llm-gateway-ratelimit = { path = "../ratelimit" }
llm-gateway-billing = { path = "../billing" }
llm-gateway-audit = { path = "../audit" }
llm-gateway-storage = { path = "../storage" }
tokio = { workspace = true }
axum = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
toml = { workspace = true }
serde = { workspace = true }
```

Each `lib.rs` starts with a comment placeholder:
```rust
// TODO: implement in subsequent tasks
```

`gateway/src/main.rs`:
```rust
fn main() {
    println!("llm-gateway");
}
```

- [ ] **Step 3: Verify workspace compiles**

Run: `cargo check`
Expected: Compiles with no errors

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: initialize Cargo workspace with all crates"
```

---

### Task 2: Define domain types

**Files:**
- Create: `crates/storage/src/types.rs`
- Modify: `crates/storage/src/lib.rs`

- [ ] **Step 1: Write domain types**

```rust
// crates/storage/src/types.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// --- API Keys ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub key_hash: String,
    pub rate_limit: Option<i64>,       // global RPM, None = unlimited
    pub budget_monthly: Option<f64>,   // monthly budget cap, None = unlimited
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKey {
    pub name: String,
    pub rate_limit: Option<i64>,
    pub budget_monthly: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateApiKey {
    pub name: Option<String>,
    pub rate_limit: Option<Option<i64>>,
    pub budget_monthly: Option<Option<f64>>,
    pub enabled: Option<bool>,
}

// --- Providers ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub api_key: String,
    pub openai_base_url: Option<String>,
    pub anthropic_base_url: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProvider {
    pub name: String,
    pub api_key: String,
    pub openai_base_url: Option<String>,
    pub anthropic_base_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProvider {
    pub name: Option<String>,
    pub api_key: Option<String>,
    pub openai_base_url: Option<Option<String>>,
    pub anthropic_base_url: Option<Option<String>>,
    pub enabled: Option<bool>,
}

// --- Models ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub name: String,
    pub provider_id: String,
    pub billing_type: BillingType,
    pub input_price: f64,     // per 1M tokens
    pub output_price: f64,    // per 1M tokens
    pub request_price: f64,   // per request
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BillingType {
    Token,
    Request,
}

#[derive(Debug, Deserialize)]
pub struct CreateModel {
    pub name: String,
    pub billing_type: BillingType,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub request_price: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateModel {
    pub billing_type: Option<BillingType>,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub request_price: Option<f64>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelWithProvider {
    pub model: Model,
    pub provider_name: String,
    pub openai_compatible: bool,
    pub anthropic_compatible: bool,
}

// --- Key-Model Rate Limits ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyModelRateLimit {
    pub key_id: String,
    pub model_name: String,
    pub rpm: i64,
    pub tpm: i64,
}

// --- Usage Records ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub id: String,
    pub key_id: String,
    pub model_name: String,
    pub provider_id: String,
    pub protocol: Protocol,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cost: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Openai,
    Anthropic,
}

#[derive(Debug, Deserialize)]
pub struct UsageFilter {
    pub key_id: Option<String>,
    pub model_name: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
}

// --- Audit Logs ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: String,
    pub key_id: String,
    pub model_name: String,
    pub provider_id: String,
    pub protocol: Protocol,
    pub request_body: String,
    pub response_body: String,
    pub status_code: i32,
    pub latency_ms: i64,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct LogFilter {
    pub key_id: Option<String>,
    pub model_name: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

// --- Config ---

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub admin: AdminConfig,
    pub database: DatabaseConfig,
    pub rate_limit: RateLimitConfig,
    pub upstream: UpstreamConfig,
    pub audit: AuditConfig,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct AdminConfig {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub driver: String,
    pub sqlite_path: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RateLimitConfig {
    pub flush_interval_secs: i64,
    pub window_size_secs: i64,
}

#[derive(Debug, Deserialize)]
pub struct UpstreamConfig {
    pub timeout_secs: u64,
}

#[derive(Debug, Deserialize)]
pub struct AuditConfig {
    pub retention_days: Option<i64>,
}
```

- [ ] **Step 2: Update storage lib.rs to export types**

```rust
// crates/storage/src/lib.rs
pub mod types;

pub use types::*;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p llm-gateway-storage`
Expected: Compiles with no errors

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: define domain types for storage layer"
```

---

### Task 3: Storage trait + SQLite implementation

**Files:**
- Create: `crates/storage/src/lib.rs` (update)
- Create: `crates/storage/src/sqlite.rs`
- Create: `crates/storage/src/migrations/init.sql`

- [ ] **Step 1: Create the SQL migration file**

```sql
-- crates/storage/src/migrations/init.sql

CREATE TABLE IF NOT EXISTS api_keys (
    id             TEXT PRIMARY KEY,
    name           TEXT NOT NULL,
    key_hash       TEXT NOT NULL UNIQUE,
    rate_limit     INTEGER,
    budget_monthly REAL,
    enabled        BOOLEAN NOT NULL DEFAULT 1,
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS providers (
    id                 TEXT PRIMARY KEY,
    name               TEXT NOT NULL,
    api_key            TEXT NOT NULL,
    openai_base_url    TEXT,
    anthropic_base_url TEXT,
    enabled            BOOLEAN NOT NULL DEFAULT 1,
    created_at         TEXT NOT NULL,
    updated_at         TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS models (
    name          TEXT PRIMARY KEY,
    provider_id   TEXT NOT NULL REFERENCES providers(id),
    billing_type  TEXT NOT NULL CHECK(billing_type IN ('token', 'request')),
    input_price   REAL NOT NULL DEFAULT 0,
    output_price  REAL NOT NULL DEFAULT 0,
    request_price REAL NOT NULL DEFAULT 0,
    enabled       BOOLEAN NOT NULL DEFAULT 1,
    created_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS key_model_rate_limits (
    key_id     TEXT NOT NULL REFERENCES api_keys(id),
    model_name TEXT NOT NULL REFERENCES models(name),
    rpm        INTEGER NOT NULL,
    tpm        INTEGER NOT NULL,
    PRIMARY KEY (key_id, model_name)
);

CREATE TABLE IF NOT EXISTS usage_records (
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

CREATE INDEX IF NOT EXISTS idx_usage_key_date ON usage_records(key_id, created_at);
CREATE INDEX IF NOT EXISTS idx_usage_model_date ON usage_records(model_name, created_at);

CREATE TABLE IF NOT EXISTS audit_logs (
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

CREATE INDEX IF NOT EXISTS idx_audit_key_date ON audit_logs(key_id, created_at);
CREATE INDEX IF NOT EXISTS idx_audit_model_date ON audit_logs(model_name, created_at);

CREATE TABLE IF NOT EXISTS rate_limit_counters (
    key_id     TEXT NOT NULL,
    model_name TEXT NOT NULL,
    window     TEXT NOT NULL,
    count      INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (key_id, model_name, window)
);
```

- [ ] **Step 2: Write the Storage trait**

```rust
// crates/storage/src/lib.rs
pub mod types;

pub use types::*;

#[async_trait::async_trait]
pub trait Storage: Send + Sync {
    // Migrations
    async fn run_migrations(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // API Keys
    async fn create_key(&self, key: &ApiKey) -> Result<ApiKey, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_key(&self, id: &str) -> Result<Option<ApiKey>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_key_by_hash(&self, hash: &str) -> Result<Option<ApiKey>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_keys(&self) -> Result<Vec<ApiKey>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_key(&self, key: &ApiKey) -> Result<ApiKey, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_key(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Providers
    async fn create_provider(&self, provider: &Provider) -> Result<Provider, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_provider(&self, id: &str) -> Result<Option<Provider>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_providers(&self) -> Result<Vec<Provider>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_provider(&self, provider: &Provider) -> Result<Provider, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_provider(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Models
    async fn create_model(&self, model: &Model) -> Result<Model, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_model(&self, name: &str) -> Result<Option<Model>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_models(&self) -> Result<Vec<ModelWithProvider>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_model(&self, model: &Model) -> Result<Model, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_model(&self, name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Key-Model Rate Limits
    async fn set_key_model_rate_limit(&self, limit: &KeyModelRateLimit) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn get_key_model_rate_limit(&self, key_id: &str, model_name: &str) -> Result<Option<KeyModelRateLimit>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_key_model_rate_limits(&self, key_id: &str) -> Result<Vec<KeyModelRateLimit>, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_key_model_rate_limit(&self, key_id: &str, model_name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Usage
    async fn record_usage(&self, usage: &UsageRecord) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn query_usage(&self, filter: &UsageFilter) -> Result<Vec<UsageRecord>, Box<dyn std::error::Error + Send + Sync>>;

    // Audit
    async fn insert_log(&self, log: &AuditLog) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn query_logs(&self, filter: &LogFilter) -> Result<Vec<AuditLog>, Box<dyn std::error::Error + Send + Sync>>;

    // Rate Limit Counters
    async fn increment_rate_limit_counter(&self, key_id: &str, model_name: &str, window: &str) -> Result<i64, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_rate_limit_counter(&self, key_id: &str, model_name: &str, window: &str) -> Result<i64, Box<dyn std::error::Error + Send + Sync>>;
}
```

Add `async-trait` to `crates/storage/Cargo.toml` workspace dependencies:
```toml
[workspace.dependencies]
async-trait = "0.1"
```

- [ ] **Step 3: Write SqliteStorage implementation**

```rust
// crates/storage/src/sqlite.rs
use super::Storage;
use crate::types::*;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;

pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    pub async fn new(db_path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let opts = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path))?
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new().max_connections(5).connect_with(opts).await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
```

Then implement each `Storage` trait method for `SqliteStorage`. Each method uses `sqlx::query` or `sqlx::query_as` with the SQLite pool. The implementation pattern for each CRUD method is:

**Example — `create_key`:**
```rust
async fn create_key(&self, key: &ApiKey) -> Result<ApiKey, Box<dyn std::error::Error + Send + Sync>> {
    sqlx::query(
        "INSERT INTO api_keys (id, name, key_hash, rate_limit, budget_monthly, enabled, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&key.id)
    .bind(&key.name)
    .bind(&key.key_hash)
    .bind(key.rate_limit)
    .bind(key.budget_monthly)
    .bind(key.enabled)
    .bind(key.created_at.to_rfc3339())
    .bind(key.updated_at.to_rfc3339())
    .execute(&self.pool)
    .await?;
    Ok(key.clone())
}
```

**Example — `get_key_by_hash`:**
```rust
async fn get_key_by_hash(&self, hash: &str) -> Result<Option<ApiKey>, Box<dyn std::error::Error + Send + Sync>> {
    let row = sqlx::query_as::<_, SqliteKeyRow>(
        "SELECT id, name, key_hash, rate_limit, budget_monthly, enabled, created_at, updated_at FROM api_keys WHERE key_hash = ?"
    )
    .bind(hash)
    .fetch_optional(&self.pool)
    .await?;
    Ok(row.map(|r| r.into()))
}
```

Use an internal `SqliteKeyRow` struct to map SQLite rows to domain types, handling the TEXT-to-DateTime conversion. Apply this same pattern for all entity types (`SqliteProviderRow`, `SqliteModelRow`, etc.).

**Example — `list_models` (joins with providers):**
```rust
async fn list_models(&self) -> Result<Vec<ModelWithProvider>, Box<dyn std::error::Error + Send + Sync>> {
    let rows = sqlx::query_as::<_, SqliteModelWithProviderRow>(
        "SELECT m.name, m.provider_id, m.billing_type, m.input_price, m.output_price, m.request_price, m.enabled, m.created_at, p.name as provider_name, p.openai_base_url, p.anthropic_base_url FROM models m JOIN providers p ON m.provider_id = p.id WHERE m.enabled = 1 AND p.enabled = 1"
    )
    .fetch_all(&self.pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.into()).collect())
}
```

**`run_migrations`:** Read `migrations/init.sql` as a string, split by `;`, execute each statement.

**`query_usage` / `query_logs`:** Build dynamic queries with `WHERE` clauses based on non-None filter fields. Use `ORDER BY created_at DESC` and `LIMIT` for pagination.

Implement all remaining methods following the same patterns. This is the largest file in the project (~300-400 lines).

- [ ] **Step 4: Update lib.rs to export sqlite module**

```rust
// crates/storage/src/lib.rs
pub mod types;
pub mod sqlite;

pub use types::*;
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo check -p llm-gateway-storage`
Expected: Compiles with no errors

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: implement Storage trait and SqliteStorage"
```

---

### Task 4: Auth crate — API key hashing and verification

**Files:**
- Modify: `crates/auth/Cargo.toml` (add dependencies)
- Modify: `crates/auth/src/lib.rs`

- [ ] **Step 1: Write failing tests**

```rust
// crates/auth/src/lib.rs
use sha2::{Digest, Sha256};

pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn verify_api_key(key: &str, hash: &str) -> bool {
    hash_api_key(key) == hash
}

pub fn generate_api_key() -> String {
    format!("sk-{}", uuid::Uuid::new_v4().to_string().replace("-", ""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_deterministic() {
        let h1 = hash_api_key("test-key");
        let h2 = hash_api_key("test-key");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_different_keys_differ() {
        let h1 = hash_api_key("key-a");
        let h2 = hash_api_key("key-b");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_verify_correct_key() {
        let hash = hash_api_key("my-secret-key");
        assert!(verify_api_key("my-secret-key", &hash));
    }

    #[test]
    fn test_verify_wrong_key() {
        let hash = hash_api_key("my-secret-key");
        assert!(!verify_api_key("wrong-key", &hash));
    }

    #[test]
    fn test_generate_key_format() {
        let key = generate_api_key();
        assert!(key.starts_with("sk-"));
        assert_eq!(key.len(), 35); // "sk-" + 32 hex chars
    }

    #[test]
    fn test_generate_keys_unique() {
        let k1 = generate_api_key();
        let k2 = generate_api_key();
        assert_ne!(k1, k2);
    }
}
```

- [ ] **Step 2: Run tests to verify they pass (implementation is inline above)**

Run: `cargo test -p llm-gateway-auth`
Expected: All 6 tests pass

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: implement API key hashing and verification"
```

---

### Task 5: Billing crate — cost calculation

**Files:**
- Modify: `crates/billing/Cargo.toml`
- Modify: `crates/billing/src/lib.rs`

- [ ] **Step 1: Write failing tests**

```rust
// crates/billing/src/lib.rs
use llm_gateway_storage::BillingType;

pub struct CostCalculation {
    pub cost: f64,
}

pub fn calculate_cost(
    billing_type: &BillingType,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    input_price: f64,   // per 1M tokens
    output_price: f64,  // per 1M tokens
    request_price: f64, // per request
) -> CostCalculation {
    let cost = match billing_type {
        BillingType::Token => {
            let input_cost = input_tokens.unwrap_or(0) as f64 / 1_000_000.0 * input_price;
            let output_cost = output_tokens.unwrap_or(0) as f64 / 1_000_000.0 * output_price;
            input_cost + output_cost
        }
        BillingType::Request => request_price,
    };
    CostCalculation { cost }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_billing() {
        let calc = calculate_cost(
            &BillingType::Token,
            Some(1_000_000),
            Some(500_000),
            3.0,  // $3/1M input
            15.0, // $15/1M output
            0.0,
        );
        assert!((calc.cost - 10.5).abs() < 0.001);
    }

    #[test]
    fn test_token_billing_zero_tokens() {
        let calc = calculate_cost(&BillingType::Token, Some(0), Some(0), 3.0, 15.0, 0.0);
        assert!((calc.cost - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_token_billing_none_tokens() {
        let calc = calculate_cost(&BillingType::Token, None, None, 3.0, 15.0, 0.0);
        assert!((calc.cost - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_request_billing() {
        let calc = calculate_cost(&BillingType::Request, None, None, 0.0, 0.0, 0.05);
        assert!((calc.cost - 0.05).abs() < 0.001);
    }

    #[test]
    fn test_request_billing_ignores_tokens() {
        let calc = calculate_cost(&BillingType::Request, Some(999), Some(999), 999.0, 999.0, 0.05);
        assert!((calc.cost - 0.05).abs() < 0.001);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p llm-gateway-billing`
Expected: All 5 tests pass

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: implement cost calculation for token and request billing"
```

---

## Phase 2: Core Proxy

### Task 6: Provider crate — upstream proxy trait + OpenAI implementation

**Files:**
- Create: `crates/provider/src/lib.rs`
- Create: `crates/provider/src/openai.rs`

- [ ] **Step 1: Define Provider trait**

```rust
// crates/provider/src/lib.rs
pub mod openai;
pub mod anthropic;

use async_trait::async_trait;
use reqwest::Client;

/// Result of a proxied request, containing response info for billing/audit.
pub struct ProxyResult {
    pub status_code: u16,
    pub response_body: String,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
}

/// Optional per-provider request/response transformation.
pub trait ProviderAdapter: Send + Sync {
    fn transform_request(&self, base_url: &str, path: &str) -> (String, Vec<(String, String)>) {
        (format!("{}{}", base_url, path), vec![])
    }
}

/// A default adapter that does no transformation.
pub struct DefaultAdapter;

impl ProviderAdapter for DefaultAdapter {}

#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn base_url(&self) -> &str;
    fn adapter(&self) -> &dyn ProviderAdapter;
    fn api_key(&self) -> &str;

    /// Proxy a non-streaming request. Returns the full response body.
    async fn proxy_request(
        &self,
        client: &Client,
        path: &str,
        request_body: String,
        extra_headers: Vec<(String, String)>,
    ) -> Result<ProxyResult, Box<dyn std::error::Error + Send + Sync>>;
}
```

- [ ] **Step 2: Implement OpenAI provider**

```rust
// crates/provider/src/openai.rs
use super::{DefaultAdapter, Provider, ProviderAdapter, ProxyResult};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

pub struct OpenAiProvider {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
}

impl ProviderAdapter for OpenAiProvider {}

#[async_trait]
impl Provider for OpenAiProvider {
    fn name(&self) -> &str { &self.name }
    fn base_url(&self) -> &str { &self.base_url }
    fn adapter(&self) -> &dyn ProviderAdapter { self }
    fn api_key(&self) -> &str { &self.api_key }

    async fn proxy_request(
        &self,
        client: &Client,
        path: &str,
        request_body: String,
        extra_headers: Vec<(String, String)>,
    ) -> Result<ProxyResult, Box<dyn std::error::Error + Send + Sync>> {
        let (url, mut adapter_headers) = self.adapter().transform_request(self.base_url(), path);
        adapter_headers.push(("Authorization".to_string(), format!("Bearer {}", self.api_key)));
        adapter_headers.push(("Content-Type".to_string(), "application/json".to_string()));
        for (k, v) in extra_headers {
            adapter_headers.push((k, v));
        }

        let mut req = client.post(&url);
        for (k, v) in &adapter_headers {
            req = req.header(k.as_str(), v.as_str());
        }
        req = req.body(request_body);

        let resp = req.send().await?;
        let status = resp.status().as_u16();
        let body = resp.text().await?;

        let (input_tokens, output_tokens) = extract_usage(&body);

        Ok(ProxyResult {
            status_code: status,
            response_body: body,
            input_tokens,
            output_tokens,
        })
    }
}

fn extract_usage(body: &str) -> (Option<i64>, Option<i64>) {
    let v: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return (None, None),
    };
    let usage = v.get("usage").and_then(|u| {
        Some((
            u.get("prompt_tokens")?.as_i64()?,
            u.get("completion_tokens")?.as_i64()?,
        ))
    });
    usage.unwrap_or((None, None))
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p llm-gateway-provider`
Expected: Compiles with no errors

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: implement Provider trait and OpenAI provider"
```

---

### Task 7: Provider crate — Anthropic implementation

**Files:**
- Create: `crates/provider/src/anthropic.rs`

- [ ] **Step 1: Implement Anthropic provider**

```rust
// crates/provider/src/anthropic.rs
use super::{DefaultAdapter, Provider, ProviderAdapter, ProxyResult};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

pub struct AnthropicProvider {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
}

impl ProviderAdapter for AnthropicProvider {}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str { &self.name }
    fn base_url(&self) -> &str { &self.base_url }
    fn adapter(&self) -> &dyn ProviderAdapter { self }
    fn api_key(&self) -> &str { &self.api_key }

    async fn proxy_request(
        &self,
        client: &Client,
        path: &str,
        request_body: String,
        extra_headers: Vec<(String, String)>,
    ) -> Result<ProxyResult, Box<dyn std::error::Error + Send + Sync>> {
        let (url, mut adapter_headers) = self.adapter().transform_request(self.base_url(), path);
        adapter_headers.push(("x-api-key".to_string(), self.api_key.clone()));
        adapter_headers.push(("anthropic-version".to_string(), "2023-06-01".to_string()));
        adapter_headers.push(("Content-Type".to_string(), "application/json".to_string()));
        for (k, v) in extra_headers {
            adapter_headers.push((k, v));
        }

        let mut req = client.post(&url);
        for (k, v) in &adapter_headers {
            req = req.header(k.as_str(), v.as_str());
        }
        req = req.body(request_body);

        let resp = req.send().await?;
        let status = resp.status().as_u16();
        let body = resp.text().await?;

        let (input_tokens, output_tokens) = extract_usage(&body);

        Ok(ProxyResult {
            status_code: status,
            response_body: body,
            input_tokens,
            output_tokens,
        })
    }
}

fn extract_usage(body: &str) -> (Option<i64>, Option<i64>) {
    let v: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return (None, None),
    };
    let usage = v.get("usage").and_then(|u| {
        Some((
            u.get("input_tokens")?.as_i64()?,
            u.get("output_tokens")?.as_i64()?,
        ))
    });
    usage.unwrap_or((None, None))
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p llm-gateway-provider`
Expected: Compiles with no errors

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: implement Anthropic provider"
```

---

### Task 8: SSE streaming support

**Files:**
- Modify: `crates/provider/src/lib.rs`
- Modify: `crates/provider/src/openai.rs`
- Modify: `crates/provider/src/anthropic.rs`

- [ ] **Step 1: Add streaming method to Provider trait**

Add to the `Provider` trait in `crates/provider/src/lib.rs`:

```rust
use tokio::sync::mpsc;

/// A single SSE event from the upstream stream.
pub struct SseEvent {
    pub data: String,
}

/// Result of a streamed request.
pub struct StreamProxyResult {
    pub status_code: u16,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
}

#[async_trait]
pub trait Provider: Send + Sync {
    // ... existing methods ...

    /// Proxy a streaming request. Returns the status code, an SSE event receiver,
    /// and extracted token counts (available after stream ends).
    async fn proxy_stream(
        &self,
        client: &Client,
        path: &str,
        request_body: String,
        extra_headers: Vec<(String, String)>,
    ) -> Result<(u16, mpsc::Receiver<Result<SseEvent, String>>), Box<dyn std::error::Error + Send + Sync>>;
}
```

- [ ] **Step 2: Implement streaming for OpenAI provider**

In `crates/provider/src/openai.rs`, implement `proxy_stream`:

```rust
async fn proxy_stream(
    &self,
    client: &Client,
    path: &str,
    request_body: String,
    extra_headers: Vec<(String, String)>,
) -> Result<(u16, mpsc::Receiver<Result<SseEvent, String>>), Box<dyn std::error::Error + Send + Sync>> {
    let (url, mut adapter_headers) = self.adapter().transform_request(self.base_url(), path);
    adapter_headers.push(("Authorization".to_string(), format!("Bearer {}", self.api_key)));
    adapter_headers.push(("Content-Type".to_string(), "application/json".to_string()));
    for (k, v) in extra_headers {
        adapter_headers.push((k, v));
    }

    let mut req = client.post(&url);
    for (k, v) in &adapter_headers {
        req = req.header(k.as_str(), v.as_str());
    }
    req = req.body(request_body);

    let resp = req.send().await?;
    let status = resp.status().as_u16();

    let (tx, rx) = mpsc::channel(256);

    tokio::spawn(async move {
        use reqwest::EventSource;
        let mut es = resp.bytes_stream().event_source();

        while let Some(event) = es.next().await {
            match event {
                Ok(e) => {
                    let _ = tx.send(Ok(SseEvent { data: e.data })).await;
                }
                Err(e) => {
                    let _ = tx.send(Err(e.to_string())).await;
                    break;
                }
            }
        }
    });

    Ok((status, rx))
}
```

Note: Enable `reqwest/stream` feature (already in workspace deps). Use `reqwest::EventSource` or manually parse SSE from the byte stream if `EventSource` is not available in reqwest 0.12. The manual approach parses `data: ...` lines from the response stream.

- [ ] **Step 3: Implement streaming for Anthropic provider**

Same pattern as OpenAI but with `x-api-key` and `anthropic-version` headers. Parse SSE events with the same manual approach. Anthropic SSE format uses `event: message_start`, `event: content_block_delta`, `event: message_delta` etc., but the `data:` field parsing is identical.

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p llm-gateway-provider`
Expected: Compiles with no errors

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: add SSE streaming support to provider proxy"
```

---

## Phase 3: Middleware & API

### Task 9: Rate limiter crate

**Files:**
- Modify: `crates/ratelimit/Cargo.toml`
- Modify: `crates/ratelimit/src/lib.rs`

- [ ] **Step 1: Write failing tests**

```rust
// crates/ratelimit/src/lib.rs
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct RateLimiter {
    counters: Arc<RwLock<HashMap<String, i64>>>,
    window_size_secs: i64,
}

impl RateLimiter {
    pub fn new(window_size_secs: i64) -> Self {
        Self {
            counters: Arc::new(RwLock::new(HashMap::new())),
            window_size_secs,
        }
    }

    /// Check if a request is allowed. Returns true if under limit.
    pub async fn check_and_increment(&self, key_id: &str, model: &str, rpm_limit: Option<i64>, tpm_limit: Option<i64>, input_tokens: Option<i64>) -> bool {
        if rpm_limit.is_none() && tpm_limit.is_none() {
            return true;
        }

        let rpm_key = format!("rpm:{}:{}", key_id, model);
        let tpm_key = format!("tpm:{}:{}", key_id, model);

        let mut counters = self.counters.write().await;

        let rpm_count = counters.entry(rpm_key.clone()).or_insert(0);
        let tpm_count = counters.entry(tpm_key.clone()).or_insert(0);

        // Check RPM
        if let Some(rpm) = rpm_limit {
            if *rpm_count >= rpm {
                return false;
            }
        }

        // Check TPM
        if let Some(tpm) = tpm_limit {
            if *tpm_count + input_tokens.unwrap_or(0) > tpm {
                return false;
            }
        }

        *rpm_count += 1;
        *tpm_count += input_tokens.unwrap_or(0);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_unlimited() {
        let limiter = RateLimiter::new(60);
        assert!(limiter.check_and_increment("key1", "model1", None, None, None).await);
    }

    #[tokio::test]
    async fn test_rpm_limit() {
        let limiter = RateLimiter::new(60);
        for _ in 0..5 {
            assert!(limiter.check_and_increment("key1", "model1", Some(5), None, None).await);
        }
        assert!(!limiter.check_and_increment("key1", "model1", Some(5), None, None).await);
    }

    #[tokio::test]
    async fn test_tpm_limit() {
        let limiter = RateLimiter::new(60);
        assert!(limiter.check_and_increment("key1", "model1", None, Some(100), Some(50)).await);
        assert!(limiter.check_and_increment("key1", "model1", None, Some(100), Some(50)).await);
        assert!(!limiter.check_and_increment("key1", "model1", None, Some(100), Some(10)).await);
    }

    #[tokio::test]
    async fn test_independent_keys() {
        let limiter = RateLimiter::new(60);
        assert!(limiter.check_and_increment("key1", "model1", Some(1), None, None).await);
        assert!(!limiter.check_and_increment("key1", "model1", Some(1), None, None).await);
        assert!(limiter.check_and_increment("key2", "model1", Some(1), None, None).await);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p llm-gateway-ratelimit`
Expected: All 4 tests pass

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: implement in-memory rate limiter"
```

---

### Task 10: Audit crate

**Files:**
- Modify: `crates/audit/Cargo.toml`
- Modify: `crates/audit/src/lib.rs`

- [ ] **Step 1: Implement audit logger**

```rust
// crates/audit/src/lib.rs
use llm_gateway_storage::{AuditLog, Protocol, Storage};
use std::sync::Arc;

pub struct AuditLogger {
    storage: Arc<dyn Storage>,
}

impl AuditLogger {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
    }

    pub async fn log_request(
        &self,
        key_id: &str,
        model_name: &str,
        provider_id: &str,
        protocol: Protocol,
        request_body: &str,
        response_body: &str,
        status_code: i32,
        latency_ms: i64,
        input_tokens: Option<i64>,
        output_tokens: Option<i64>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let log = AuditLog {
            id: uuid::Uuid::new_v4().to_string(),
            key_id: key_id.to_string(),
            model_name: model_name.to_string(),
            provider_id: provider_id.to_string(),
            protocol,
            request_body: request_body.to_string(),
            response_body: response_body.to_string(),
            status_code,
            latency_ms,
            input_tokens,
            output_tokens,
            created_at: chrono::Utc::now(),
        };
        self.storage.insert_log(&log).await
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p llm-gateway-audit`
Expected: Compiles with no errors

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: implement audit logger"
```

---

### Task 11: API crate — error types and extractors

**Files:**
- Create: `crates/api/src/error.rs`
- Create: `crates/api/src/extractors.rs`

- [ ] **Step 1: Write error types**

```rust
// crates/api/src/error.rs
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug)]
pub enum ApiError {
    Unauthorized,
    Forbidden,
    RateLimited,
    NotFound(String),
    BadRequest(String),
    UpstreamError(u16, String),
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden"),
            ApiError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded"),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.as_str()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.as_str()),
            ApiError::UpstreamError(code, msg) => {
                (StatusCode::from_u16(*code).unwrap_or(StatusCode::BAD_GATEWAY), msg.as_str())
            }
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.as_str()),
        };
        let body = json!({ "error": { "message": message, "type": status.as_u16() } });
        (status, axum::Json(body)).into_response()
    }
}
```

- [ ] **Step 2: Write extractors**

```rust
// crates/api/src/extractors.rs
use axum::http::HeaderMap;
use llm_gateway_storage::ApiKey;

/// Extract API key from Authorization header.
/// Returns the raw key string and the hashed version.
pub fn extract_bearer_token(headers: &HeaderMap) -> Result<String, crate::error::ApiError> {
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(crate::error::ApiError::Unauthorized)?;

    if !auth.starts_with("Bearer ") {
        return Err(crate::error::ApiError::Unauthorized);
    }

    Ok(auth[7..].to_string())
}

/// Verify admin token from Authorization header.
pub fn verify_admin_token(headers: &HeaderMap, expected_token: &str) -> Result<(), crate::error::ApiError> {
    let token = extract_bearer_token(headers)?;
    if token != expected_token {
        return Err(crate::error::ApiError::Unauthorized);
    }
    Ok(())
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p llm-gateway-api`
Expected: Compiles with no errors

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: implement API error types and request extractors"
```

---

### Task 12: API crate — OpenAI proxy endpoint

**Files:**
- Create: `crates/api/src/openai.rs`
- Modify: `crates/api/src/lib.rs`

- [ ] **Step 1: Implement OpenAI proxy handler**

```rust
// crates/api/src/openai.rs
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use llm_gateway_storage::{ApiKey, Model, Protocol, Storage};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;

use crate::error::ApiError;
use crate::extractors::extract_bearer_token;
use crate::AppState;

pub async fn chat_completions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Result<impl IntoResponse, ApiError> {
    let raw_key = extract_bearer_token(&headers)?;
    let key_hash = llm_gateway_auth::hash_api_key(&raw_key);
    let api_key = state.storage.get_key_by_hash(&key_hash).await
        .map_err(|_| ApiError::Internal("DB error".into()))?
        .ok_or(ApiError::Unauthorized)?;

    if !api_key.enabled {
        return Err(ApiError::Forbidden);
    }

    // Parse model from request body
    let req_json: Value = serde_json::from_str(&body)
        .map_err(|e| ApiError::BadRequest(format!("Invalid JSON: {}", e)))?;
    let model_name = req_json.get("model")
        .and_then(|m| m.as_str())
        .ok_or(ApiError::BadRequest("Missing 'model' field".into()))?
        .to_string();

    // Rate limit check
    let (rpm_limit, tpm_limit) = get_rate_limits(&state, &api_key, &model_name).await;
    if !state.rate_limiter.check_and_increment(&api_key.id, &model_name, rpm_limit, tpm_limit, None).await {
        return Err(ApiError::RateLimited);
    }

    // Find model and provider
    let model = state.storage.get_model(&model_name).await
        .map_err(|_| ApiError::Internal("DB error".into()))?
        .ok_or(ApiError::BadRequest(format!("Model '{}' not found", model_name)))?;

    let provider = state.storage.get_provider(&model.provider_id).await
        .map_err(|_| ApiError::Internal("DB error".into()))?
        .ok_or(ApiError::Internal("Provider not found".into()))?;

    let base_url = provider.openai_base_url.as_ref()
        .ok_or(ApiError::BadRequest(format!("Provider '{}' does not support OpenAI protocol", provider.name)))?;

    let is_stream = req_json.get("stream").and_then(|s| s.as_bool()).unwrap_or(false);

    if is_stream {
        proxy_stream(&state, &api_key, &model, &provider, base_url, "/chat/completions", &body, Protocol::Openai).await
    } else {
        proxy_request(&state, &api_key, &model, &provider, base_url, "/chat/completions", &body, Protocol::Openai).await
    }
}

pub async fn list_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let raw_key = extract_bearer_token(&headers)?;
    let key_hash = llm_gateway_auth::hash_api_key(&raw_key);
    let api_key = state.storage.get_key_by_hash(&key_hash).await
        .map_err(|_| ApiError::Internal("DB error".into()))?
        .ok_or(ApiError::Unauthorized)?;

    if !api_key.enabled {
        return Err(ApiError::Forbidden);
    }

    let models = state.storage.list_models().await
        .map_err(|_| ApiError::Internal("DB error".into()))?;

    let openai_models: Vec<Value> = models.iter()
        .filter(|m| m.openai_compatible)
        .map(|m| json!({
            "id": m.model.name,
            "object": "model",
            "owned_by": m.provider_name,
        }))
        .collect();

    Ok(Json(json!({
        "object": "list",
        "data": openai_models,
    })))
}

async fn proxy_request(
    state: &Arc<AppState>,
    api_key: &ApiKey,
    model: &Model,
    provider: &llm_gateway_storage::Provider,
    base_url: &str,
    path: &str,
    body: &str,
    protocol: Protocol,
) -> Result<impl IntoResponse, ApiError> {
    let start = Instant::now();
    let http_client = reqwest::Client::new();

    let upstream = llm_gateway_provider::openai::OpenAiProvider {
        name: provider.name.clone(),
        base_url: base_url.to_string(),
        api_key: provider.api_key.clone(),
    };

    let result = upstream.proxy_request(&http_client, path, body.to_string(), vec![]).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let latency_ms = start.elapsed().as_millis() as i64;

    // Post-process: audit + billing (async, non-blocking)
    let state_clone = state.clone();
    let api_key_clone = api_key.clone();
    let model_clone = model.clone();
    let provider_clone = provider.clone();
    let body_clone = body.to_string();
    let resp_clone = result.response_body.clone();
    tokio::spawn(async move {
        let _ = state_clone.audit_logger.log_request(
            &api_key_clone.id,
            &model_clone.name,
            &provider_clone.id,
            protocol,
            &body_clone,
            &resp_clone,
            result.status_code as i32,
            latency_ms,
            result.input_tokens,
            result.output_tokens,
        ).await;

        let cost = llm_gateway_billing::calculate_cost(
            &model_clone.billing_type,
            result.input_tokens,
            result.output_tokens,
            model_clone.input_price,
            model_clone.output_price,
            model_clone.request_price,
        );
        let usage = llm_gateway_storage::UsageRecord {
            id: uuid::Uuid::new_v4().to_string(),
            key_id: api_key_clone.id,
            model_name: model_clone.name,
            provider_id: provider_clone.id,
            protocol,
            input_tokens: result.input_tokens,
            output_tokens: result.output_tokens,
            cost: cost.cost,
            created_at: chrono::Utc::now(),
        };
        let _ = state_clone.storage.record_usage(&usage).await;
    });

    Ok((
        StatusCode::from_u16(result.status_code).unwrap_or(StatusCode::OK),
        result.response_body,
    ))
}

async fn proxy_stream(
    state: &Arc<AppState>,
    api_key: &ApiKey,
    model: &Model,
    provider: &llm_gateway_storage::Provider,
    base_url: &str,
    path: &str,
    body: &str,
    protocol: Protocol,
) -> Result<impl IntoResponse, ApiError> {
    // Stream proxy: forward SSE events, collect usage at end
    // Implementation builds an SSE response using axum::response::sse
    // and forwards events from the upstream channel
    todo!("SSE stream proxy implementation — uses provider.proxy_stream()")
}

async fn get_rate_limits(
    state: &Arc<AppState>,
    api_key: &ApiKey,
    model_name: &str,
) -> (Option<i64>, Option<i64>) {
    // Check per-key-per-model limits first, fall back to global key limit
    if let Ok(Some(limit)) = state.storage.get_key_model_rate_limit(&api_key.id, model_name).await {
        (Some(limit.rpm), Some(limit.tpm))
    } else {
        (api_key.rate_limit, None)
    }
}
```

- [ ] **Step 2: Update api lib.rs**

```rust
// crates/api/src/lib.rs
pub mod error;
pub mod extractors;
pub mod openai;
pub mod anthropic;
pub mod management;

use llm_gateway_audit::AuditLogger;
use llm_gateway_ratelimit::RateLimiter;
use llm_gateway_storage::Storage;
use std::sync::Arc;

pub struct AppState {
    pub storage: Arc<dyn Storage>,
    pub rate_limiter: Arc<RateLimiter>,
    pub audit_logger: Arc<AuditLogger>,
    pub admin_token: String,
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p llm-gateway-api`
Expected: Compiles with no errors (may have warnings about `todo!()`)

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: implement OpenAI proxy endpoint"
```

---

### Task 13: API crate — Anthropic proxy endpoint

**Files:**
- Create: `crates/api/src/anthropic.rs`

- [ ] **Step 1: Implement Anthropic proxy handler**

Same structure as `openai.rs` but:
- Uses `llm_gateway_provider::anthropic::AnthropicProvider` instead of `OpenAiProvider`
- Uses `provider.anthropic_base_url` instead of `openai_base_url`
- Header is `x-api-key` instead of `Authorization: Bearer`
- Extracts `input_tokens`/`output_tokens` from Anthropic's `usage` format
- Path is `/v1/messages` instead of `/v1/chat/completions`
- Model list endpoint filters by `anthropic_compatible`

The `list_models` response uses Anthropic's format:
```json
{
  "data": [{ "id": "model-name", "type": "model", "display_name": "Model Name" }],
  "object": "list"
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p llm-gateway-api`
Expected: Compiles with no errors

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: implement Anthropic proxy endpoint"
```

---

### Task 14: API crate — Management API (keys, providers, models)

**Files:**
- Create: `crates/api/src/management/mod.rs`
- Create: `crates/api/src/management/keys.rs`
- Create: `crates/api/src/management/providers.rs`
- Create: `crates/api/src/management/models.rs`
- Create: `crates/api/src/management/usage.rs`
- Create: `crates/api/src/management/logs.rs`

- [ ] **Step 1: Implement management key endpoints**

```rust
// crates/api/src/management/keys.rs
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use llm_gateway_storage::{ApiKey, CreateApiKey, UpdateApiKey};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::error::ApiError;
use crate::extractors::verify_admin_token;
use crate::AppState;

pub async fn create_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreateApiKey>,
) -> Result<Json<ApiKey>, ApiError> {
    verify_admin_token(&headers, &state.admin_token)?;
    let now = chrono::Utc::now();
    let raw_key = llm_gateway_auth::generate_api_key();
    let key = ApiKey {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name,
        key_hash: llm_gateway_auth::hash_api_key(&raw_key),
        rate_limit: input.rate_limit,
        budget_monthly: input.budget_monthly,
        enabled: true,
        created_at: now,
        updated_at: now,
    };
    let created = state.storage.create_key(&key).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    // Return the raw key only on creation
    Ok(Json(created))
}

pub async fn list_keys(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ApiKey>>, ApiError> {
    verify_admin_token(&headers, &state.admin_token)?;
    let keys = state.storage.list_keys().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(keys))
}

pub async fn get_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApiKey>, ApiError> {
    verify_admin_token(&headers, &state.admin_token)?;
    let key = state.storage.get_key(&id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound("Key not found".into()))?;
    Ok(Json(key))
}

pub async fn update_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<UpdateApiKey>,
) -> Result<Json<ApiKey>, ApiError> {
    verify_admin_token(&headers, &state.admin_token)?;
    let mut key = state.storage.get_key(&id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound("Key not found".into()))?;
    if let Some(name) = input.name { key.name = name; }
    if let Some(rl) = input.rate_limit { key.rate_limit = rl; }
    if let Some(b) = input.budget_monthly { key.budget_monthly = b; }
    if let Some(e) = input.enabled { key.enabled = e; }
    key.updated_at = chrono::Utc::now();
    let updated = state.storage.update_key(&key).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(updated))
}

pub async fn delete_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    verify_admin_token(&headers, &state.admin_token)?;
    state.storage.delete_key(&id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(json!({ "deleted": true })))
}
```

- [ ] **Step 2: Implement management provider endpoints**

Same CRUD pattern as keys, but for providers. `create_provider` accepts `CreateProvider`, returns the created `Provider`. Provider API key is stored as-is (not hashed — it's an upstream key, not a client credential).

- [ ] **Step 3: Implement management model endpoints**

Nested under providers:
- `POST /api/v1/providers/:id/models` — create model on provider
- `PATCH /api/v1/providers/:id/models/:model_name` — update model config
- `DELETE /api/v1/providers/:id/models/:model_name` — remove model

- [ ] **Step 4: Implement usage query endpoint**

```rust
// crates/api/src/management/usage.rs
pub async fn get_usage(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::Query(filter): axum::Query<UsageFilter>,
) -> Result<Json<Vec<UsageRecord>>, ApiError> {
    verify_admin_token(&headers, &state.admin_token)?;
    let records = state.storage.query_usage(&filter).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(records))
}
```

- [ ] **Step 5: Implement audit log query endpoint**

Same pattern as usage but using `query_logs` with `LogFilter`.

- [ ] **Step 6: Implement management router**

```rust
// crates/api/src/management/mod.rs
pub mod keys;
pub mod providers;
pub mod models;
pub mod usage;
pub mod logs;

use axum::routing::{get, patch, post, delete};
use axum::Router;
use std::sync::Arc;
use crate::AppState;

pub fn management_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/keys", post(keys::create_key).get(keys::list_keys))
        .route("/api/v1/keys/{id}", get(keys::get_key).patch(keys::update_key).delete(keys::delete_key))
        .route("/api/v1/providers", post(providers::create_provider).get(providers::list_providers))
        .route("/api/v1/providers/{id}", get(providers::get_provider).patch(providers::update_provider).delete(providers::delete_provider))
        .route("/api/v1/providers/{id}/models", post(models::create_model))
        .route("/api/v1/providers/{id}/models/{model_name}", patch(models::update_model).delete(models::delete_model))
        .route("/api/v1/usage", get(usage::get_usage))
        .route("/api/v1/logs", get(logs::get_logs))
}
```

- [ ] **Step 7: Verify it compiles**

Run: `cargo check -p llm-gateway-api`
Expected: Compiles with no errors

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat: implement management API for keys, providers, models, usage, logs"
```

---

## Phase 4: App Assembly & Integration

### Task 15: Gateway main — bootstrap and router assembly

**Files:**
- Modify: `crates/gateway/src/main.rs`
- Create: `config.toml`

- [ ] **Step 1: Write main.rs**

```rust
// crates/gateway/src/main.rs
use llm_gateway_api::{self as api, AppState};
use llm_gateway_audit::AuditLogger;
use llm_gateway_ratelimit::RateLimiter;
use llm_gateway_storage::{AppConfig, SqliteStorage};
use std::sync::Arc;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::init();

    // Load config
    let config_str = std::fs::read_to_string("config.toml")?;
    let config: AppConfig = toml::from_str(&config_str)?;

    // Init storage
    let storage = SqliteStorage::new(
        config.database.sqlite_path.as_deref().unwrap_or("./data/gateway.db")
    ).await?;
    storage.run_migrations().await?;

    let storage: Arc<dyn llm_gateway_storage::Storage> = Arc::new(storage);

    // Init rate limiter
    let rate_limiter = Arc::new(RateLimiter::new(config.rate_limit.window_size_secs));

    // Init audit logger
    let audit_logger = Arc::new(AuditLogger::new(storage.clone()));

    // App state
    let state = Arc::new(AppState {
        storage,
        rate_limiter,
        audit_logger,
        admin_token: config.admin.token,
    });

    // Build router
    let app = axum::Router::new()
        .route("/v1/chat/completions", axum::routing::post(api::openai::chat_completions))
        .route("/v1/models", axum::routing::get(api::openai::list_models))
        .route("/v1/messages", axum::routing::post(api::anthropic::messages))
        .merge(api::management::management_router())
        .with_state(state)
        .layer(axum::middleware::from_fn(trace_middleware));

    // Start server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    tracing::info!("Starting LLM Gateway on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn trace_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    tracing::info!(method = %method, path = %path, "request");
    let resp = next.run(req).await;
    tracing::info!(method = %method, path = %path, status = resp.status().as_u16(), "response");
    resp
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("Failed to install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
```

- [ ] **Step 2: Create default config.toml**

```toml
[server]
host = "0.0.0.0"
port = 8080

[admin]
token = "change-me-in-production"

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

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: Compiles with no errors

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: assemble gateway main with router and config loading"
```

---

### Task 16: End-to-end integration test

**Files:**
- Create: `tests/common/mod.rs`
- Create: `tests/test_management_keys.rs`

- [ ] **Step 1: Create test helpers**

```rust
// tests/common/mod.rs
use llm_gateway_storage::SqliteStorage;
use std::sync::Arc;

pub async fn setup_test_db() -> Arc<SqliteStorage> {
    let storage = SqliteStorage::new(":memory:").await.unwrap();
    storage.run_migrations().await.unwrap();
    Arc::new(storage)
}
```

- [ ] **Step 2: Write integration test for key management**

```rust
// tests/test_management_keys.rs
mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use llm_gateway_api::{AppState, management};
use llm_gateway_audit::AuditLogger;
use llm_gateway_ratelimit::RateLimiter;
use llm_gateway_storage::Storage;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

fn build_app(state: Arc<AppState>) -> axum::Router {
    management::management_router().with_state(state)
}

#[tokio::test]
async fn test_create_and_list_keys() {
    let db = common::setup_test_db().await;
    let state = Arc::new(AppState {
        storage: db.clone(),
        rate_limiter: Arc::new(RateLimiter::new(60)),
        audit_logger: Arc::new(AuditLogger::new(db)),
        admin_token: "test-token".to_string(),
    });

    let app = build_app(state);

    let resp = app
        .oneshot(Request::builder()
            .method("POST")
            .uri("/api/v1/keys")
            .header("authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(json!({"name": "test-key"}).to_string()))
            .unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(&hyper::body::to_bytes(resp.into_body()).await.unwrap()).unwrap();
    assert_eq!(body["name"], "test-key");
    assert!(body["key_hash"].is_string());
}

#[tokio::test]
async fn test_unauthorized_access() {
    let db = common::setup_test_db().await;
    let state = Arc::new(AppState {
        storage: db.clone(),
        rate_limiter: Arc::new(RateLimiter::new(60)),
        audit_logger: Arc::new(AuditLogger::new(db)),
        admin_token: "test-token".to_string(),
    });

    let app = build_app(state);

    let resp = app
        .oneshot(Request::builder()
            .method("GET")
            .uri("/api/v1/keys")
            .header("authorization", "Bearer wrong-token")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
```

- [ ] **Step 3: Add test dependencies to workspace Cargo.toml**

```toml
[workspace.dependencies]
tower = "0.5"
hyper = "1"
```

- [ ] **Step 4: Run integration tests**

Run: `cargo test --test test_management_keys`
Expected: Tests pass

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "test: add integration tests for key management API"
```

---

### Task 17: Integration tests for provider and model management

**Files:**
- Create: `tests/test_management_providers.rs`

- [ ] **Step 1: Write integration tests**

Test the full CRUD lifecycle:
1. Create a provider
2. List providers (verify it appears)
3. Get provider by ID
4. Add a model to the provider
5. Update model pricing
6. Delete model
7. Update provider
8. Delete provider

Also test:
- Creating a provider with no base URLs (valid — can be updated later)
- Deleting a provider that doesn't exist (404)
- Creating a model with invalid billing_type (400)

- [ ] **Step 2: Run integration tests**

Run: `cargo test --test test_management_providers`
Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "test: add integration tests for provider and model management"
```

---

## Phase 5: SSE Streaming & Polish

### Task 18: SSE streaming proxy implementation

**Files:**
- Modify: `crates/api/src/openai.rs` (complete `proxy_stream`)
- Modify: `crates/api/src/anthropic.rs` (complete `proxy_stream`)

- [ ] **Step 1: Implement SSE streaming for OpenAI endpoint**

Use `axum::response::sse::Sse` with a stream that:
1. Calls `provider.proxy_stream()` to get the upstream SSE channel
2. Forwards each event to the client via `axum::response::sse::Event`
3. Accumulates token counts from events that contain `usage` data
4. On stream end, spawns async task to write audit log + billing record

- [ ] **Step 2: Implement SSE streaming for Anthropic endpoint**

Same pattern. Anthropic SSE events may have different event types (`message_start`, `content_block_delta`, `message_delta`) but the `data:` field parsing is the same.

- [ ] **Step 3: Write integration test for streaming**

Use a mock HTTP server to verify SSE events are forwarded correctly. Test:
- Stream forwards events to client
- Usage is extracted from stream-final events
- Audit log and billing are written after stream ends

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: implement SSE streaming proxy for OpenAI and Anthropic"
```

---

### Task 19: Key creation returns raw key

**Files:**
- Modify: `crates/api/src/management/keys.rs`

- [ ] **Step 1: Update create_key to return raw key in response**

The raw API key should only be returned once, on creation. After that, only the hash is stored. Update the response to include the raw key:

```rust
pub struct CreateKeyResponse {
    pub id: String,
    pub name: String,
    pub key: String,  // raw key, only returned on creation
    pub rate_limit: Option<i64>,
    pub budget_monthly: Option<f64>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}
```

Return `Json(CreateKeyResponse { ..., key: raw_key })` from the `create_key` handler.

- [ ] **Step 2: Update test to verify raw key is returned**

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: return raw API key only on creation"
```

---

### Task 20: Build and verify

- [ ] **Step 1: Build release binary**

Run: `cargo build --release`
Expected: Compiles successfully, produces binary at `target/release/llm-gateway`

- [ ] **Step 2: Run all tests**

Run: `cargo test`
Expected: All unit and integration tests pass

- [ ] **Step 3: Manual smoke test**

1. Start gateway: `./target/release/llm-gateway`
2. Create a provider: `curl -X POST http://localhost:8080/api/v1/providers -H "Authorization: Bearer change-me-in-production" -H "Content-Type: application/json" -d '{"name":"test","api_key":"sk-test","openai_base_url":"https://api.openai.com/v1"}'`
3. Create an API key: `curl -X POST http://localhost:8080/api/v1/keys -H "Authorization: Bearer change-me-in-production" -H "Content-Type: application/json" -d '{"name":"test"}'`
4. List models: `curl http://localhost:8080/v1/models -H "Authorization: Bearer <key from step 3>"`
5. Verify responses are correct

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "chore: verify build and tests pass"
```

---

## Summary

| Phase | Tasks | Description |
|---|---|---|
| 1 | 1-5 | Project scaffold, domain types, storage, auth, billing |
| 2 | 6-8 | Provider trait, OpenAI/Anthropic implementations, SSE |
| 3 | 9-14 | Rate limiter, audit, error types, all API endpoints |
| 4 | 15-17 | Main bootstrap, config, integration tests |
| 5 | 18-20 | SSE streaming proxy, key creation polish, build verification |

**Frontend (React SPA) is a separate plan** — to be written after backend is functional and testable.
