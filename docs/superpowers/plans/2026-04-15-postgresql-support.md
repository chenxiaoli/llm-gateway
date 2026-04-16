# PostgreSQL Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add PostgreSQL support to llm-gateway by creating PostgresStorage implementation alongside existing SqliteStorage.

**Architecture:** Refactor storage crate to support both SQLite and PostgreSQL via feature flags. Create Storage trait implementation for PostgreSQL using sqlx with postgres feature.

**Tech Stack:** Rust (sqlx, tokio, async-trait)

---

### Task 1: Update Cargo.toml - Add postgres feature

**Files:**
- Modify: `crates/storage/Cargo.toml`

- [ ] **Step 1: Add postgres feature to Cargo.toml**

```toml
[package]
name = "llm-gateway-storage"
version = "0.1.0"
edition = "2021"

[features]
default = ["sqlite"]
sqlite = []
postgres = []

[dependencies]
sqlx = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true }
async-trait = { workspace = true }

[dependencies.sqlx]
features = ["runtime-tokio-rustls", "sqlite", "postgres"]
optional = true

[target.'cfg(feature = "postgres")'.dependencies]
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "postgres"] }
```

- [ ] **Step 2: Commit**

```bash
git add crates/storage/Cargo.toml
git commit -m "feat: add postgres feature flag to storage crate"
```

---

### Task 2: Create PostgresStorage implementation

**Files:**
- Create: `crates/storage/src/postgres.rs`

- [ ] **Step 1: Create postgres.rs with Storage trait implementation**

Copy the structure from sqlite.rs and adapt for PostgreSQL:

```rust
// crates/storage/src/postgres.rs
use async_trait::async_trait;
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};
use sqlx::Row;

use crate::types::*;
use crate::Storage;

pub struct PostgresStorage {
    pool: PgPool,
}

impl PostgresStorage {
    pub async fn new(connection_string: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(connection_string)
            .await?;
        Ok(PostgresStorage { pool })
    }
}

#[async_trait]
impl Storage for PostgresStorage {
    async fn run_migrations(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Use sqlx migrate for postgres migrations
        let migrator = sqlx::migrate!("./migrations/postgres");
        migrator.run(&self.pool).await.map_err(|e| Box::new(e) as _)
    }

    // Implement all Storage trait methods for PostgreSQL
    // ... (copy from sqlite.rs and adapt queries)
}
```

- [ ] **Step 2: Implement all Storage trait methods**

Each method needs to be adapted for PostgreSQL syntax:
- Replace `?` parameter placeholders with `$1, $2, ...`
- Use `gen_ulid()` instead of UUID for ID generation
- Use PostgreSQL types (BOOLEAN instead of INTEGER for bool)
- Use `TIMESTAMP WITH TIME ZONE` for dates

Example for create_key:
```rust
async fn create_key(&self, key: &ApiKey) -> Result<ApiKey, Box<dyn std::error::Error + Send + Sync>> {
    sqlx::query(
        "INSERT INTO api_keys (id, name, key_hash, rate_limit, budget_monthly, enabled, created_by, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
    )
    .bind(&key.id)
    .bind(&key.name)
    .bind(&key.key_hash)
    .bind(&key.rate_limit)
    .bind(&key.budget_monthly)
    .bind(key.enabled)
    .bind(&key.created_by)
    .bind(key.created_at.to_rfc3339())
    .bind(key.updated_at.to_rfc3339())
    .execute(&self.pool)
    .await?;
    Ok(key.clone())
}
```

- [ ] **Step 3: Run cargo check to verify**

```bash
cd /workspace && cargo check --package llm-gateway-storage --features postgres
```

Expected: No errors (may need iterations to fix)

- [ ] **Step 4: Commit**

```bash
git add crates/storage/src/postgres.rs
git commit -m "feat: add PostgresStorage implementation"
```

---

### Task 3: Update lib.rs to conditionally export storage

**Files:**
- Modify: `crates/storage/src/lib.rs`

- [ ] **Step 1: Add conditional module declarations**

```rust
pub mod types;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;

pub use types::*;

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteStorage;

#[cfg(feature = "postgres")]
pub use postgres::PostgresStorage;
```

- [ ] **Step 2: Add create_storage factory function**

```rust
#[cfg(feature = "sqlite")]
pub async fn create_sqlite_storage(path: &str) -> Result<SqliteStorage, Box<dyn std::error::Error + Send + Sync>> {
    SqliteStorage::new(path).await
}

#[cfg(feature = "postgres")]
pub async fn create_postgres_storage(connection_string: &str) -> Result<PostgresStorage, Box<dyn std::error::Error + Send + Sync>> {
    PostgresStorage::new(connection_string).await
}
```

- [ ] **Step 3: Run cargo check**

```bash
cd /workspace && cargo check --package llm-gateway-storage --all-features
```

Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add crates/storage/src/lib.rs
git commit -m "feat: add storage factory functions for sqlite and postgres"
```

---

### Task 4: Create PostgreSQL migrations

**Files:**
- Create: `crates/storage/migrations/postgres/*.sql`

- [ ] **Step 1: Create postgres migrations directory**

```bash
mkdir -p crates/storage/migrations/postgres
```

- [ ] **Step 2: Create init migration (20260415000000_init.sql)**

```sql
-- PostgreSQL init migration
-- Use gen_ulid() for ID generation

-- API Keys
CREATE TABLE IF NOT EXISTS api_keys (
    id            TEXT PRIMARY KEY DEFAULT gen_ulid(),
    name          TEXT NOT NULL,
    key_hash      TEXT NOT NULL UNIQUE,
    rate_limit    INTEGER,
    budget_monthly REAL,
    enabled       BOOLEAN NOT NULL DEFAULT true,
    created_by    TEXT REFERENCES users(id),
    created_at    TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Providers
CREATE TABLE IF NOT EXISTS providers (
    id          TEXT PRIMARY KEY DEFAULT gen_ulid(),
    name        TEXT NOT NULL,
    base_url    TEXT,
    api_key     TEXT,
    enabled     BOOLEAN NOT NULL DEFAULT true,
    created_at  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Channels
CREATE TABLE IF NOT EXISTS channels (
    id           TEXT PRIMARY KEY DEFAULT gen_ulid(),
    provider_id  TEXT NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    base_url    TEXT,
    api_key     TEXT,
    rpm_limit   INTEGER,
    tpm_limit   INTEGER,
    enabled     BOOLEAN NOT NULL DEFAULT true,
    created_at  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Models
CREATE TABLE IF NOT EXISTS models (
    name              TEXT PRIMARY KEY,
    provider_id       TEXT NOT NULL REFERENCES providers(id),
    model_type        TEXT,
    pricing_policy_id TEXT REFERENCES pricing_policies(id),
    billing_type      TEXT NOT NULL DEFAULT 'per_token',
    input_price       REAL NOT NULL DEFAULT 0,
    output_price      REAL NOT NULL DEFAULT 0,
    request_price     REAL NOT NULL DEFAULT 0,
    enabled           BOOLEAN NOT NULL DEFAULT true,
    created_at        TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- ... (continue with other tables)
```

- [ ] **Step 3: Create remaining migrations**

Create all migrations matching SQLite schema:
- 20260401000001_users.sql
- 20260401000002_refresh_tokens.sql
- 20260401000003_channels.sql
- 20260401000004_models_model_type.sql
- 20260414000000_add_channel_fields.sql
- 20260414000001_add_model_id.sql
- 20260414000002_create_channel_models.sql
- 20260415000000_provider_endpoints.sql
- 20260415000001_pricing_policies.sql
- 20260415000002_add_model_billing_type.sql
- 20260415000003_add_channel_model_billing_fields.sql

Each with PostgreSQL-specific syntax (gen_ulid(), BOOLEAN, TIMESTAMP WITH TIME ZONE).

- [ ] **Step 4: Commit**

```bash
git add crates/storage/migrations/postgres/
git commit -m "db: add PostgreSQL migrations"
```

---

### Task 5: Update main.rs or app initialization

**Files:**
- Modify: `crates/api/src/main.rs` or initialization code

- [ ] **Step 1: Update to use new storage factory**

Based on config, create appropriate storage:

```rust
let storage: Box<dyn Storage> = match config.database.database_type {
    DatabaseType::Sqlite => {
        let sqlite = llm_gateway_storage::create_sqlite_storage(&config.database.path).await?;
        Box::new(sqlite)
    }
    DatabaseType::Postgres => {
        let pg = llm_gateway_storage::create_postgres_storage(&config.database.connection_string()).await?;
        Box::new(pg)
    }
};
```

- [ ] **Step 2: Commit**

```bash
git add crates/api/src/main.rs
git commit -m "feat: support both sqlite and postgres storage"
```

---

### Task 6: Final Verification

- [ ] **Step 1: Run full build with both features**

```bash
cd /workspace && cargo build --all-features 2>&1 | tail -30
```

Expected: Build successful

- [ ] **Step 2: Commit**

```bash
git add .
git commit -m "feat: add PostgreSQL support to llm-gateway"
```