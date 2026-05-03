# Drop SQLite Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove all SQLite code, migrations, dependencies, and references from the codebase. PostgreSQL becomes the only supported database driver.

**Architecture:** Purely mechanical deletion and substitution. Delete `sqlite.rs`, 20 migration files, and SQLite feature flags. Rewrite the test helper to connect to PostgreSQL via `DATABASE_URL` env var. Update all 5 test files to use `PostgresStorage` instead of `SqliteStorage`.

**Tech Stack:** Rust, sqlx (PostgreSQL only), tokio

---

### Task 1: Delete SQLite source and migration files

**Files:**
- Delete: `crates/storage/src/sqlite.rs`
- Delete: `crates/storage/migrations/sqlite/` (20 SQL files)

- [ ] **Step 1: Delete sqlite.rs**

```bash
rm crates/storage/src/sqlite.rs
```

- [ ] **Step 2: Delete SQLite migration directory**

```bash
rm -rf crates/storage/migrations/sqlite/
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "chore: delete SQLite storage implementation and migrations"
```

---

### Task 2: Remove SQLite feature flags and sqlx feature from Cargo.toml files

**Files:**
- Modify: `crates/storage/Cargo.toml`
- Modify: `Cargo.toml` (workspace root)
- Modify: `crates/gateway/Cargo.toml`

- [ ] **Step 1: Update `crates/storage/Cargo.toml`**

Remove the `[features]` section entirely (both `default = ["sqlite"]` and `sqlite = []` and `postgres = []`). The crate no longer needs feature flags since only PostgreSQL exists.

The file should go from:

```toml
[package]
name = "llm-gateway-storage"
version = "1.0.0"
edition = "2021"

[features]
default = ["sqlite"]
sqlite = []
postgres = []

[dependencies]
...
```

To:

```toml
[package]
name = "llm-gateway-storage"
version = "1.0.0"
edition = "2021"

[dependencies]
...
```

- [ ] **Step 2: Update workspace `Cargo.toml`**

Remove `"sqlite"` from the sqlx features line in `[workspace.dependencies]`:

Change:
```toml
sqlx = { version = "0.8", features = ["runtime-tokio-native-tls", "sqlite", "postgres", "migrate", "derive", "chrono"] }
```

To:
```toml
sqlx = { version = "0.8", features = ["runtime-tokio-native-tls", "postgres", "migrate", "derive", "chrono"] }
```

- [ ] **Step 3: Update `crates/gateway/Cargo.toml`**

Change:
```toml
llm-gateway-storage = { path = "../storage", features = ["sqlite", "postgres"] }
```

To:
```toml
llm-gateway-storage = { path = "../storage" }
```

- [ ] **Step 4: Verify build compiles**

Run: `cargo check --workspace 2>&1 | head -30`

Expected: Compilation errors only about the removed sqlite module in `lib.rs` (fixed in Task 3). No other new errors.

- [ ] **Step 5: Commit**

```bash
git add crates/storage/Cargo.toml Cargo.toml crates/gateway/Cargo.toml
git commit -m "chore: remove SQLite feature flags and sqlx sqlite feature"
```

---

### Task 3: Remove SQLite module declaration and update build.rs

**Files:**
- Modify: `crates/storage/src/lib.rs` (lines 4-5)
- Modify: `crates/storage/build.rs`

- [ ] **Step 1: Update `crates/storage/src/lib.rs`**

Remove the sqlite module declaration and its cfg gate. Change lines 4-6 from:

```rust
#[cfg(feature = "sqlite")]
pub mod sqlite;
#[cfg(feature = "postgres")]
pub mod postgres;
```

To:

```rust
pub mod postgres;
```

- [ ] **Step 2: Update `crates/storage/build.rs`**

Remove the SQLite rerun-if-changed line. Change:

```rust
fn main() {
    println!("cargo:rerun-if-changed=migrations/*");
    println!("cargo:rerun-if-changed=migrations/postgres/*");
    println!("cargo:rerun-if-changed=migrations/sqlite/*");
}
```

To:

```rust
fn main() {
    println!("cargo:rerun-if-changed=migrations/*");
    println!("cargo:rerun-if-changed=migrations/postgres/*");
}
```

- [ ] **Step 3: Verify storage crate compiles**

Run: `cargo check -p llm-gateway-storage 2>&1 | tail -5`

Expected: Success (no errors).

- [ ] **Step 4: Commit**

```bash
git add crates/storage/src/lib.rs crates/storage/build.rs
git commit -m "chore: remove SQLite module declaration and build trigger"
```

---

### Task 4: Remove SQLite from gateway main.rs

**Files:**
- Modify: `crates/gateway/src/main.rs`

- [ ] **Step 1: Remove SqliteStorage import**

On line 9, delete:
```rust
use llm_gateway_storage::sqlite::SqliteStorage;
```

Keep the `PostgresStorage` import on line 10.

- [ ] **Step 2: Replace driver match block with postgres-only logic**

Change the storage init block (lines 33-53) from:

```rust
    let storage: Arc<dyn Storage> = match config.database.driver.as_str() {
        "postgres" => {
            let url = config.database.url.as_deref().ok_or("database.url is required for postgres")?;
            tracing::info!("Using PostgreSQL: {}", url.split('@').last().unwrap_or("***"));
            let db = PostgresStorage::new(url).await?;
            db.run_migrations().await?;
            db.seed_data().await?;
            Arc::new(db)
        }
        "sqlite" => {
            let db_path = config.database.url.as_deref().unwrap_or("./data/gateway.db");
            tracing::info!("Using SQLite: {}", db_path);
            let db = SqliteStorage::new(db_path).await?;
            db.run_migrations().await?;
            db.seed_data().await?;
            Arc::new(db)
        }
        other => {
            return Err(format!("Unknown database driver '{}'. Supported: 'sqlite', 'postgres'", other).into());
        }
    };
```

To:

```rust
    let storage: Arc<dyn Storage> = {
        if config.database.driver.as_str() != "postgres" {
            return Err(format!("Unsupported database driver '{}'. Only 'postgres' is supported", config.database.driver).into());
        }
        let url = config.database.url.as_deref().ok_or("database.url is required")?;
        tracing::info!("Using PostgreSQL: {}", url.split('@').last().unwrap_or("***"));
        let db = PostgresStorage::new(url).await?;
        db.run_migrations().await?;
        db.seed_data().await?;
        Arc::new(db)
    };
```

- [ ] **Step 3: Update bootstrap default config to use postgres**

In the `bootstrap()` function (around line 194), change the `[database]` section in the default config template from:

```toml
[database]
driver = "sqlite"
url = "./data/gateway.db"
```

To:

```toml
[database]
driver = "postgres"
url = "postgresql://user:password@localhost/llm_gateway"
```

- [ ] **Step 4: Verify gateway crate compiles**

Run: `cargo check -p llm-gateway 2>&1 | tail -5`

Expected: Success (no errors).

- [ ] **Step 5: Commit**

```bash
git add crates/gateway/src/main.rs
git commit -m "chore: remove SQLite driver from gateway, postgres only"
```

---

### Task 5: Migrate test helper to PostgreSQL

**Files:**
- Modify: `crates/api/tests/common/mod.rs`

This task changes the test helper from `SqliteStorage::new_in_memory()` to `PostgresStorage::new()` using the `DATABASE_URL` env var. Tests will share a single PostgreSQL connection (created once via `once_cell::sync::Lazy` or similar) with table truncation between tests.

- [ ] **Step 1: Rewrite `crates/api/tests/common/mod.rs`**

Replace the entire file with:

```rust
use llm_gateway_storage::{postgres::PostgresStorage, Storage};
use llm_gateway_auth::create_jwt;
use llm_gateway_api::{ChannelRegistry, ResolvedChannel};
use std::sync::Arc;

pub struct MockChannelRegistry;

#[async_trait::async_trait]
impl ChannelRegistry for MockChannelRegistry {
    async fn resolve(&self, _channel_id: &str) -> Option<ResolvedChannel> {
        None
    }
    async fn resolve_by_model(&self, _model: &str) -> Vec<ResolvedChannel> {
        Vec::new()
    }
    async fn reload(&self) {}
}

pub const TEST_JWT_SECRET: &str = "test-jwt-secret";

#[allow(dead_code)]
pub struct TestUser {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub username: String,
    #[allow(dead_code)]
    pub role: String,
    pub token: String,
}

/// Set up a test database connection.
/// Requires DATABASE_URL env var to point at a PostgreSQL instance.
/// Truncates all tables to ensure test isolation.
pub async fn setup_test_db() -> Arc<PostgresStorage> {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL env var must be set for tests (e.g. postgresql://test:test@localhost/llm_gateway_test)");
    let storage = PostgresStorage::new(&database_url)
        .await
        .expect("Failed to connect to test PostgreSQL");
    storage.run_migrations().await.expect("Failed to run migrations");

    // Truncate all tables for test isolation
    let tables = [
        "transactions", "accounts", "usage_records", "audit_logs",
        "rate_limit_counters", "channel_models", "channels",
        "provider_models", "provider_models_pricing", "models",
        "providers", "api_keys", "users", "settings",
        "pricing_policies", "model_fallbacks",
    ];
    for table in &tables {
        sqlx::query(&format!("TRUNCATE TABLE {} CASCADE", table))
            .execute(storage.pool())
            .await
            .expect("Failed to truncate table");
    }

    Arc::new(storage)
}

#[allow(dead_code)]
pub fn make_admin_token() -> TestUser {
    let id = "admin-1".to_string();
    let token = create_jwt(&id, "admin", TEST_JWT_SECRET).unwrap();
    TestUser {
        id,
        username: "admin".to_string(),
        role: "admin".to_string(),
        token,
    }
}

#[allow(dead_code)]
pub fn make_user_token(user_id: &str) -> TestUser {
    let token = create_jwt(user_id, "user", TEST_JWT_SECRET).unwrap();
    TestUser {
        id: user_id.to_string(),
        username: "testuser".to_string(),
        role: "user".to_string(),
        token,
    }
}
```

- [ ] **Step 2: Verify test helper compiles**

Run: `cargo check -p llm-gateway-api --tests 2>&1 | tail -10`

Expected: Compilation errors only about `SqliteStorage` type mismatches in the 5 test files (fixed in Task 6). No errors from common/mod.rs itself.

- [ ] **Step 3: Commit**

```bash
git add crates/api/tests/common/mod.rs
git commit -m "chore: migrate test helper from SQLite to PostgreSQL"
```

---

### Task 6: Update all test files to use PostgresStorage

**Files:**
- Modify: `crates/api/tests/test_auth.rs`
- Modify: `crates/api/tests/test_management_keys.rs`
- Modify: `crates/api/tests/test_management_providers.rs`
- Modify: `crates/api/tests/test_settings.rs`
- Modify: `crates/api/tests/test_users.rs`

All 5 test files have the same pattern to fix. Each has a `make_state` function with:
1. Parameter type `Arc<llm_gateway_storage::sqlite::SqliteStorage>` → `Arc<llm_gateway_storage::postgres::PostgresStorage>`
2. `database_driver: "sqlite".to_string()` → `"postgres".to_string()`

- [ ] **Step 1: Update `test_auth.rs`**

Change the `make_state` function signature and body:

From:
```rust
fn make_state(db: Arc<llm_gateway_storage::sqlite::SqliteStorage>) -> Arc<AppState> {
```
To:
```rust
fn make_state(db: Arc<llm_gateway_storage::postgres::PostgresStorage>) -> Arc<AppState> {
```

Change `database_driver` value:
```rust
            database_driver: "postgres".to_string(),
```

- [ ] **Step 2: Update `test_management_keys.rs`**

Same changes as Step 1 — update `make_state` parameter type and `database_driver` value.

- [ ] **Step 3: Update `test_management_providers.rs`**

Same changes as Step 1 — update `make_state` parameter type and `database_driver` value.

- [ ] **Step 4: Update `test_settings.rs`**

Same changes as Step 1 — update `make_state` parameter type and `database_driver` value.

- [ ] **Step 5: Update `test_users.rs`**

Same changes as Step 1 — update `make_state` parameter type and `database_driver` value.

- [ ] **Step 6: Verify all tests compile**

Run: `cargo check -p llm-gateway-api --tests 2>&1 | tail -5`

Expected: Success (no errors).

- [ ] **Step 7: Commit**

```bash
git add crates/api/tests/test_auth.rs crates/api/tests/test_management_keys.rs crates/api/tests/test_management_providers.rs crates/api/tests/test_settings.rs crates/api/tests/test_users.rs
git commit -m "chore: update test files to use PostgresStorage"
```

---

### Task 7: Update CI workflow

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Replace SQLite setup with PostgreSQL service**

In the `backend-test` job, replace the SQLite database creation step with a PostgreSQL service container.

Replace the `backend-test` job's steps. Change from the current (lines 52-77):

```yaml
  backend-test:
    needs: frontend-build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: actions/download-artifact@v4
        with:
          name: llm-gateway-frontend
          path: web/dist

      - uses: dtolnay/rust-toolchain@stable

      - uses: Swatinem/rust-cache@v2

      - name: Install cargo-sqlx
        run: cargo install sqlx-cli --locked

      - name: Create database for sqlx
        run: mkdir -p data && touch data/gateway.db

      - name: Run sqlx prepare
        run: DATABASE_URL=sqlite:data/gateway.db cargo sqlx prepare --workspace

      - name: Run tests
        run: cargo test --workspace
```

To:

```yaml
  backend-test:
    needs: frontend-build
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:18
        env:
          POSTGRES_USER: test
          POSTGRES_PASSWORD: test
          POSTGRES_DB: llm_gateway_test
        ports:
          - 5432:5432
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
    env:
      DATABASE_URL: postgresql://test:test@localhost/llm_gateway_test
    steps:
      - uses: actions/checkout@v4

      - uses: actions/download-artifact@v4
        with:
          name: llm-gateway-frontend
          path: web/dist

      - uses: dtolnay/rust-toolchain@stable

      - uses: Swatinem/rust-cache@v2

      - name: Install cargo-sqlx
        run: cargo install sqlx-cli --no-default-features --features postgres --locked

      - name: Run sqlx prepare
        run: DATABASE_URL=postgresql://test:test@localhost/llm_gateway_test cargo sqlx prepare --workspace

      - name: Run tests
        run: cargo test --workspace
```

Note: `sqlx-cli` is installed with `--no-default-features --features postgres` to avoid requiring libsqlite3-sys on CI. The `DATABASE_URL` env var is set at job level so both `sqlx prepare` and `cargo test` use it.

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: use PostgreSQL service container instead of SQLite"
```

---

### Task 8: Update documentation and config files

**Files:**
- Modify: `README.md`
- Modify: `CLAUDE.md`
- Modify: `web/src/test/server.ts`
- Modify: `docs/dev-deploy.md`

- [ ] **Step 1: Update `README.md`**

In the architecture section, change the storage description from SQLite/PostgreSQL to PostgreSQL only. Change the config example from `driver = "sqlite"` to `driver = "postgres"`. Remove any SQLite-specific config examples.

Find all references to `sqlite` or `SQLite` and update them to reference PostgreSQL only.

- [ ] **Step 2: Update `CLAUDE.md`**

On line 39, change:
```
├── storage/      # SQLite/PostgreSQL storage trait + migrations
```
To:
```
├── storage/      # PostgreSQL storage trait + migrations
```

- [ ] **Step 3: Update `web/src/test/server.ts`**

Change both occurrences of `database_driver: 'sqlite'` to `database_driver: 'postgres'`.

On line 90 and line 101, change:
```typescript
      database_driver: 'sqlite',
```
To:
```typescript
      database_driver: 'postgres',
```

- [ ] **Step 4: Update `docs/dev-deploy.md`**

Change the config example database section from:
```toml
[database]
driver = "sqlite"
url = "./data/gateway.db"
```
To:
```toml
[database]
driver = "postgres"
url = "postgresql://user:password@localhost/llm_gateway"
```

- [ ] **Step 5: Commit**

```bash
git add README.md CLAUDE.md web/src/test/server.ts docs/dev-deploy.md
git commit -m "docs: remove SQLite references, PostgreSQL only"
```

---

### Task 9: Verify full build and clean up Cargo.lock

**Files:**
- Modified automatically: `Cargo.lock`

- [ ] **Step 1: Full clean build**

```bash
cargo clean
cargo build --workspace 2>&1 | tail -20
```

Expected: Successful build. No SQLite-related errors.

- [ ] **Step 2: Verify libsqlite3-sys is gone from Cargo.lock**

```bash
grep -c "libsqlite3-sys" Cargo.lock
```

Expected: `0` (no matches).

- [ ] **Step 3: Run cargo test (requires DATABASE_URL)**

```bash
DATABASE_URL="postgresql://llm_gateway:Xabc12345@10.0.17.3/llm_gateway_test" cargo test --workspace 2>&1 | tail -30
```

Expected: All tests pass. (A test database must exist; if it doesn't, create it first with `createdb -h 10.0.17.3 -U llm_gateway llm_gateway_test`.)

- [ ] **Step 4: Commit Cargo.lock changes**

```bash
git add Cargo.lock
git commit -m "chore: update Cargo.lock after SQLite removal"
```
