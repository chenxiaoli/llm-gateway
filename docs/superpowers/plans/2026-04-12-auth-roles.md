# User Authentication & Role-Based Access Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace static admin token auth with username/password login, JWT sessions, two roles (admin/user), and role-based sidebar grouping.

**Architecture:** Extend existing `auth` crate with bcrypt password hashing and JWT. Add `users` table to storage. Replace `verify_admin_token()` with JWT extraction middleware. Frontend adds AuthContext, new login/register pages, and sidebar grouped by Console/Admin.

**Tech Stack:** Rust (axum, sqlx, bcrypt, jsonwebtoken), React 18 (Ant Design, React Query, React Router)

---

### Task 1: Add password hashing and JWT to auth crate

**Files:**
- Modify: `crates/auth/Cargo.toml`
- Modify: `crates/auth/src/lib.rs`

- [ ] **Step 1: Add bcrypt and jsonwebtoken dependencies**

Add to `Cargo.toml`:
```toml
[dependencies]
sha2 = { workspace = true }
hex = { workspace = true }
uuid = { workspace = true }
serde = { workspace = true }
bcrypt = "0.17"
jsonwebtoken = "9"
chrono = { workspace = true }
```

- [ ] **Step 2: Add password hashing functions and tests**

Add to `crates/auth/src/lib.rs` after the existing code (keep all existing API key functions):

```rust
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

// --- Password hashing (bcrypt) ---

pub fn hash_password(plain: &str) -> String {
    bcrypt::hash(plain, bcrypt::DEFAULT_COST).unwrap()
}

pub fn verify_password(plain: &str, hash: &str) -> bool {
    bcrypt::verify(plain, hash).unwrap_or(false)
}

// --- JWT ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtClaims {
    pub sub: String,   // user id
    pub role: String,  // "admin" or "user"
    pub exp: usize,    // expiration timestamp
    pub iat: usize,    // issued at
}

pub fn create_jwt(user_id: &str, role: &str, secret: &str) -> Result<String, String> {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = JwtClaims {
        sub: user_id.to_string(),
        role: role.to_string(),
        exp: now + 86400, // 24 hours
        iat: now,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| e.to_string())
}

pub fn verify_jwt(token: &str, secret: &str) -> Result<JwtClaims, String> {
    let token_data = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| e.to_string())?;
    Ok(token_data.claims)
}
```

- [ ] **Step 3: Add tests for password hashing and JWT**

Add to the existing `#[cfg(test)] mod tests` block in `crates/auth/src/lib.rs`:

```rust
#[test]
fn test_hash_and_verify_password() {
    let hash = hash_password("my-password");
    assert!(verify_password("my-password", &hash));
    assert!(!verify_password("wrong-password", &hash));
}

#[test]
fn test_create_and_verify_jwt() {
    let secret = "test-secret";
    let token = create_jwt("user-1", "admin", secret).unwrap();
    let claims = verify_jwt(&token, secret).unwrap();
    assert_eq!(claims.sub, "user-1");
    assert_eq!(claims.role, "admin");
}

#[test]
fn test_verify_jwt_wrong_secret() {
    let token = create_jwt("user-1", "admin", "secret-1").unwrap();
    assert!(verify_jwt(&token, "secret-2").is_err());
}

#[test]
fn test_verify_jwt_expired() {
    let mut claims = JwtClaims {
        sub: "user-1".to_string(),
        role: "admin".to_string(),
        exp: 0,
        iat: 0,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret("secret".as_bytes()),
    )
    .unwrap();
    assert!(verify_jwt(&token, "secret").is_err());
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p llm-gateway-auth`
Expected: All tests pass (existing + 4 new)

- [ ] **Step 5: Commit**

```bash
git add crates/auth/Cargo.toml crates/auth/src/lib.rs
git commit -m "feat(auth): add password hashing (bcrypt) and JWT functions"
```

---

### Task 2: Add users table and user CRUD to storage

**Files:**
- Modify: `crates/storage/src/types.rs`
- Modify: `crates/storage/src/lib.rs`
- Modify: `crates/storage/src/sqlite.rs`
- Modify: `crates/storage/src/migrations.rs`
- Create: `crates/storage/src/migrations/002_users.sql`

- [ ] **Step 1: Create migration file**

Create `crates/storage/src/migrations/002_users.sql`:

```sql
CREATE TABLE IF NOT EXISTS users (
    id          TEXT PRIMARY KEY,
    username    TEXT NOT NULL UNIQUE,
    password    TEXT NOT NULL,
    role        TEXT NOT NULL DEFAULT 'user' CHECK(role IN ('admin', 'user')),
    enabled     BOOLEAN NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
```

- [ ] **Step 2: Update migrations.rs to support numbered migrations**

Replace the contents of `crates/storage/src/migrations.rs`:

```rust
pub const INIT_SQL: &str = include_str!("migrations/init.sql");
pub const MIGRATION_002: &str = include_str!("migrations/002_users.sql");

pub const ALL_MIGRATIONS: &[&str] = &[INIT_SQL, MIGRATION_002];
```

- [ ] **Step 3: Add User types to types.rs**

Add to `crates/storage/src/types.rs` (before the Config section):

```rust
// --- Users ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub password: String,
    pub role: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUser {
    pub role: Option<String>,
    pub enabled: Option<bool>,
}

// --- Settings ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub allow_registration: bool,
}
```

- [ ] **Step 4: Add User and Settings methods to Storage trait**

Add to `crates/storage/src/lib.rs` inside the `Storage` trait (after the Rate Limit Counters section):

```rust
    // Users
    async fn create_user(&self, user: &User) -> Result<User, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_user(&self, id: &str) -> Result<Option<User>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_users(&self) -> Result<Vec<User>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_user(&self, user: &User) -> Result<User, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_user(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn count_admin_users(&self) -> Result<i64, Box<dyn std::error::Error + Send + Sync>>;
    async fn user_count(&self) -> Result<i64, Box<dyn std::error::Error + Send + Sync>>;

    // Settings
    async fn get_setting(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>;
    async fn set_setting(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
```

- [ ] **Step 5: Implement user and settings storage in sqlite.rs**

Add SQLite row type at the top of `crates/storage/src/sqlite.rs` (after the other row types):

```rust
#[derive(FromRow)]
struct SqliteUserRow {
    id: String,
    username: String,
    password: String,
    role: String,
    enabled: i64,
    created_at: String,
    updated_at: String,
}

impl From<SqliteUserRow> for User {
    fn from(r: SqliteUserRow) -> Self {
        User {
            id: r.id,
            username: r.username,
            password: r.password,
            role: r.role,
            enabled: r.enabled != 0,
            created_at: parse_rfc3339(&r.created_at),
            updated_at: parse_rfc3339(&r.updated_at),
        }
    }
}
```

Add the import at the top of `sqlite.rs`:

```rust
use crate::migrations::ALL_MIGRATIONS;
```

Replace `run_migrations` in the `Storage` impl:

```rust
    async fn run_migrations(&self) -> Result<(), DbErr> {
        // Create settings table if not exists (used by 002_users)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)"
        )
        .execute(&self.pool)
        .await?;

        for migration_sql in ALL_MIGRATIONS {
            for stmt in migration_sql.split(';') {
                let trimmed = stmt.trim();
                if !trimmed.is_empty() {
                    sqlx::query(trimmed).execute(&self.pool).await?;
                }
            }
        }
        Ok(())
    }
```

Add user and settings implementations at the end of the `Storage` impl (before the closing brace):

```rust
    // ---- Users ----

    async fn create_user(&self, user: &User) -> Result<User, DbErr> {
        sqlx::query(
            "INSERT INTO users (id, username, password, role, enabled, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&user.id)
        .bind(&user.username)
        .bind(&user.password)
        .bind(&user.role)
        .bind(user.enabled as i64)
        .bind(user.created_at.to_rfc3339())
        .bind(user.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(user.clone())
    }

    async fn get_user(&self, id: &str) -> Result<Option<User>, DbErr> {
        let row: Option<SqliteUserRow> = sqlx::query_as(
            "SELECT id, username, password, role, enabled, created_at, updated_at
             FROM users WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(User::from))
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, DbErr> {
        let row: Option<SqliteUserRow> = sqlx::query_as(
            "SELECT id, username, password, role, enabled, created_at, updated_at
             FROM users WHERE username = ?",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(User::from))
    }

    async fn list_users(&self) -> Result<Vec<User>, DbErr> {
        let rows: Vec<SqliteUserRow> = sqlx::query_as(
            "SELECT id, username, password, role, enabled, created_at, updated_at FROM users",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(User::from).collect())
    }

    async fn update_user(&self, user: &User) -> Result<User, DbErr> {
        sqlx::query(
            "UPDATE users SET username = ?, password = ?, role = ?, enabled = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(&user.username)
        .bind(&user.password)
        .bind(&user.role)
        .bind(user.enabled as i64)
        .bind(user.updated_at.to_rfc3339())
        .bind(&user.id)
        .execute(&self.pool)
        .await?;

        Ok(user.clone())
    }

    async fn delete_user(&self, id: &str) -> Result<(), DbErr> {
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn count_admin_users(&self) -> Result<i64, DbErr> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM users WHERE role = 'admin' AND enabled = 1",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    async fn user_count(&self) -> Result<i64, DbErr> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM users",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    // ---- Settings ----

    async fn get_setting(&self, key: &str) -> Result<Option<String>, DbErr> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT value FROM settings WHERE key = ?",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.0))
    }

    async fn set_setting(&self, key: &str, value: &str) -> Result<(), DbErr> {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES (?, ?)
             ON CONFLICT(key) DO UPDATE SET value = ?",
        )
        .bind(key)
        .bind(value)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p llm-gateway-storage`
Expected: All tests pass

- [ ] **Step 7: Commit**

```bash
git add crates/storage/
git commit -m "feat(storage): add users table, user CRUD, and settings storage"
```

---

### Task 3: Update AppState and auth middleware

**Files:**
- Modify: `crates/storage/src/types.rs` (config section)
- Modify: `crates/api/src/lib.rs`
- Modify: `crates/api/src/extractors.rs`
- Modify: `crates/api/src/error.rs`

- [ ] **Step 1: Update config types**

Replace the `AdminConfig` in `crates/storage/src/types.rs` with `AuthConfig`:

```rust
#[derive(Debug, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub allow_registration: Option<bool>,
}
```

Update `AppConfig` to use `AuthConfig` instead of `AdminConfig`:

```rust
#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub database: DatabaseConfig,
    pub rate_limit: RateLimitConfig,
    pub upstream: UpstreamConfig,
    pub audit: AuditConfig,
}
```

- [ ] **Step 2: Update AppState and add auth module**

Replace `crates/api/src/lib.rs`:

```rust
pub mod auth;
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
    pub jwt_secret: String,
}
```

- [ ] **Step 3: Create extractors module with JWT auth**

Replace `crates/api/src/extractors.rs`:

```rust
use axum::http::HeaderMap;
use crate::error::ApiError;
use llm_gateway_auth::verify_jwt;
use llm_gateway_auth::JwtClaims;

pub fn extract_bearer_token(headers: &HeaderMap) -> Result<String, ApiError> {
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;
    if !auth.starts_with("Bearer ") {
        return Err(ApiError::Unauthorized);
    }
    Ok(auth[7..].to_string())
}

pub fn require_auth(headers: &HeaderMap, jwt_secret: &str) -> Result<JwtClaims, ApiError> {
    let token = extract_bearer_token(headers)?;
    let claims = verify_jwt(&token, jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;
    Ok(claims)
}

pub fn require_admin(headers: &HeaderMap, jwt_secret: &str) -> Result<JwtClaims, ApiError> {
    let claims = require_auth(headers, jwt_secret)?;
    if claims.role != "admin" {
        return Err(ApiError::Forbidden);
    }
    Ok(claims)
}
```

- [ ] **Step 4: Update config.toml**

Replace `config.toml` at project root:

```toml
[server]
host = "0.0.0.0"
port = 8080

[auth]
jwt_secret = "change-me-in-production"
allow_registration = true

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

- [ ] **Step 5: Update main.rs**

Replace `crates/gateway/src/main.rs` AppState construction. Change:
```rust
admin_token: config.admin.token,
```
To:
```rust
jwt_secret: config.auth.jwt_secret.clone(),
```

Also add deprecation warning for old config. In `main.rs`, after loading config, add:

```rust
// Deprecation warning for old admin.token config
if config.admin.is_some() {
    tracing::warn!("The [admin] config section is deprecated. Use [auth] with jwt_secret instead.");
}
```

Note: Since `AppConfig` changed from `admin` to `auth`, the `config.admin` reference above won't compile. Instead, use a try-with approach. Actually, since we changed the struct, old configs with `[admin]` will fail to parse. This is acceptable as a breaking change. Remove the deprecation warning and just let it fail to parse.

So the main.rs change is just:
- Replace `admin_token: config.admin.token,` with `jwt_secret: config.auth.jwt_secret.clone(),`

- [ ] **Step 6: Run cargo check**

Run: `cargo check --workspace`
Expected: Compilation errors in management handlers that reference `admin_token` — these will be fixed in Tasks 4 and 5.

- [ ] **Step 7: Commit**

```bash
git add crates/storage/src/types.rs crates/api/src/lib.rs crates/api/src/extractors.rs config.toml crates/gateway/src/main.rs
git commit -m "feat: replace admin_token with JWT auth in AppState and config"
```

---

### Task 4: Add auth endpoints (login, register, me, config)

**Files:**
- Create: `crates/api/src/auth.rs`
- Modify: `crates/api/src/management/mod.rs`

- [ ] **Step 1: Create auth endpoint handlers**

Create `crates/api/src/auth.rs`:

```rust
use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use llm_gateway_auth::{hash_password, verify_password, create_jwt};
use llm_gateway_storage::User;

use crate::error::ApiError;
use crate::extractors::require_auth;
use crate::AppState;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Serialize, Clone)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub role: String,
}

#[derive(Serialize)]
pub struct MeResponse {
    pub id: String,
    pub username: String,
    pub role: String,
    pub allow_registration: bool,
}

#[derive(Serialize)]
pub struct AuthConfigResponse {
    pub allow_registration: bool,
}

impl From<&User> for UserInfo {
    fn from(u: &User) -> Self {
        UserInfo {
            id: u.id.clone(),
            username: u.username.clone(),
            role: u.role.clone(),
        }
    }
}

async fn get_allow_registration(state: &AppState) -> bool {
    state
        .storage
        .get_setting("allow_registration")
        .await
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(true) // default to true if setting not set
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(input): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let user = state
        .storage
        .get_user_by_username(&input.username)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;

    if !user.enabled {
        return Err(ApiError::Unauthorized);
    }

    if !verify_password(&input.password, &user.password) {
        return Err(ApiError::Unauthorized);
    }

    let token = create_jwt(&user.id, &user.role, &state.jwt_secret)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(AuthResponse {
        token,
        user: UserInfo::from(&user),
    }))
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(input): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let allow_reg = get_allow_registration(&state).await;

    let user_count = state
        .storage
        .user_count()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // First user always becomes admin, regardless of registration setting
    let is_first_user = user_count == 0;

    if !is_first_user && !allow_reg {
        return Err(ApiError::Forbidden);
    }

    // Check username uniqueness
    if state
        .storage
        .get_user_by_username(&input.username)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .is_some()
    {
        return Err(ApiError::BadRequest("Username already exists".to_string()));
    }

    let now = chrono::Utc::now();
    let role = if is_first_user { "admin" } else { "user" };
    let user = User {
        id: uuid::Uuid::new_v4().to_string(),
        username: input.username.clone(),
        password: hash_password(&input.password),
        role: role.to_string(),
        enabled: true,
        created_at: now,
        updated_at: now,
    };

    state
        .storage
        .create_user(&user)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let token = create_jwt(&user.id, &user.role, &state.jwt_secret)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(AuthResponse {
        token,
        user: UserInfo::from(&user),
    }))
}

pub async fn me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<MeResponse>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let user = state
        .storage
        .get_user(&claims.sub)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;

    let allow_reg = get_allow_registration(&state).await;

    Ok(Json(MeResponse {
        id: user.id,
        username: user.username,
        role: user.role,
        allow_registration: allow_reg,
    }))
}

pub async fn auth_config(
    State(state): State<Arc<AppState>>,
) -> Result<Json<AuthConfigResponse>, ApiError> {
    let allow_reg = get_allow_registration(&state).await;

    Ok(Json(AuthConfigResponse {
        allow_registration: allow_reg,
    }))
}
```

- [ ] **Step 2: Register auth routes in management/mod.rs**

Replace `crates/api/src/management/mod.rs`:

```rust
pub mod auth;
pub mod keys;
pub mod providers;
pub mod models;
pub mod usage;
pub mod logs;
pub mod users;
pub mod settings;

use axum::routing::{get, patch, post};
use axum::Router;
use std::sync::Arc;
use crate::AppState;

pub fn management_router() -> Router<Arc<AppState>> {
    Router::new()
        // Auth (public)
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/config", get(auth::auth_config))
        .route("/api/v1/auth/me", get(auth::me))
        // Keys (authenticated)
        .route("/api/v1/keys", post(keys::create_key).get(keys::list_keys))
        .route(
            "/api/v1/keys/{id}",
            get(keys::get_key).patch(keys::update_key).delete(keys::delete_key),
        )
        // Providers (admin)
        .route(
            "/api/v1/providers",
            post(providers::create_provider).get(providers::list_providers),
        )
        .route(
            "/api/v1/providers/{id}",
            get(providers::get_provider).patch(providers::update_provider).delete(providers::delete_provider),
        )
        .route(
            "/api/v1/providers/{id}/models",
            post(models::create_model),
        )
        .route(
            "/api/v1/providers/{id}/models/{model_name}",
            patch(models::update_model).delete(models::delete_model),
        )
        // Usage (authenticated)
        .route("/api/v1/usage", get(usage::get_usage))
        // Logs (admin)
        .route("/api/v1/logs", get(logs::get_logs))
        // Users (admin)
        .route("/api/v1/users", get(users::list_users))
        .route(
            "/api/v1/users/{id}",
            patch(users::update_user).delete(users::delete_user),
        )
        // Settings (admin)
        .route("/api/v1/settings", get(settings::get_settings).patch(settings::update_settings))
}
```

- [ ] **Step 3: Run cargo check**

Run: `cargo check --workspace`
Expected: Compilation errors in keys.rs, providers.rs, usage.rs, logs.rs that reference old `admin_token` — fixed in next task.

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/auth.rs crates/api/src/management/mod.rs
git commit -m "feat(api): add auth endpoints (login, register, me, config)"
```

---

### Task 5: Update management handlers to use JWT auth

**Files:**
- Modify: `crates/api/src/management/keys.rs`
- Modify: `crates/api/src/management/providers.rs`
- Modify: `crates/api/src/management/usage.rs`
- Modify: `crates/api/src/management/logs.rs`

- [ ] **Step 1: Update keys.rs — role-scoped access**

Replace `crates/api/src/management/keys.rs`:

```rust
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::Arc;

use llm_gateway_auth::{generate_api_key, hash_api_key};
use llm_gateway_storage::{ApiKey, CreateApiKey as StorageCreateApiKey, UpdateApiKey as StorageUpdateApiKey};

use crate::error::ApiError;
use crate::extractors::{require_auth, require_admin};
use crate::AppState;

#[derive(Serialize)]
pub struct CreateKeyResponse {
    pub id: String,
    pub name: String,
    pub key: String,
    pub rate_limit: Option<i64>,
    pub budget_monthly: Option<f64>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

pub async fn create_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<StorageCreateApiKey>,
) -> Result<Json<CreateKeyResponse>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let now = chrono::Utc::now();
    let raw_key = generate_api_key();
    let key = ApiKey {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name,
        key_hash: hash_api_key(&raw_key),
        rate_limit: input.rate_limit,
        budget_monthly: input.budget_monthly,
        enabled: true,
        created_at: now,
        updated_at: now,
    };

    let created = state
        .storage
        .create_key(&key)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(CreateKeyResponse {
        id: created.id,
        name: created.name,
        key: raw_key,
        rate_limit: created.rate_limit,
        budget_monthly: created.budget_monthly,
        enabled: created.enabled,
        created_at: created.created_at,
    }))
}

pub async fn list_keys(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ApiKey>>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let keys = state
        .storage
        .list_keys()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Non-admin users only see their own keys
    if claims.role != "admin" {
        let filtered: Vec<ApiKey> = keys
            .into_iter()
            .filter(|k| k.created_by.as_deref() == Some(&claims.sub))
            .collect();
        return Ok(Json(filtered));
    }

    Ok(Json(keys))
}

pub async fn get_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApiKey>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let key = state
        .storage
        .get_key(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Key '{}' not found", id)))?;

    // Non-admin can only view their own keys
    if claims.role != "admin" && key.created_by.as_deref() != Some(&claims.sub) {
        return Err(ApiError::NotFound(format!("Key '{}' not found", id)));
    }

    Ok(Json(key))
}

pub async fn update_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<StorageUpdateApiKey>,
) -> Result<Json<ApiKey>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let mut key = state
        .storage
        .get_key(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Key '{}' not found", id)))?;

    // Non-admin can only update their own keys
    if claims.role != "admin" && key.created_by.as_deref() != Some(&claims.sub) {
        return Err(ApiError::NotFound(format!("Key '{}' not found", id)));
    }

    if let Some(name) = input.name {
        key.name = name;
    }
    if let Some(rate_limit) = input.rate_limit {
        key.rate_limit = rate_limit;
    }
    if let Some(budget_monthly) = input.budget_monthly {
        key.budget_monthly = budget_monthly;
    }
    if let Some(enabled) = input.enabled {
        key.enabled = enabled;
    }
    key.updated_at = chrono::Utc::now();

    let updated = state
        .storage
        .update_key(&key)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(updated))
}

pub async fn delete_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    // Non-admin can only delete their own keys
    if claims.role != "admin" {
        let key = state
            .storage
            .get_key(&id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .ok_or(ApiError::NotFound(format!("Key '{}' not found", id)))?;

        if key.created_by.as_deref() != Some(&claims.sub) {
            return Err(ApiError::NotFound(format!("Key '{}' not found", id)));
        }
    }

    state
        .storage
        .delete_key(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
```

Note: `list_keys` filters by `created_by`, but the `ApiKey` type doesn't have a `created_by` field yet. Add `created_by: Option<String>` to the `ApiKey` struct in `crates/storage/src/types.rs`:

```rust
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub key_hash: String,
    pub rate_limit: Option<i64>,
    pub budget_monthly: Option<f64>,
    pub enabled: bool,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

Update `SqliteKeyRow` in `crates/storage/src/sqlite.rs` to include `created_by: Option<String>`, and update the `From<SqliteKeyRow>` impl:
```rust
created_by: r.created_by,
```

Update all `SqliteKeyRow` SELECT queries to include `created_by`, the INSERT to include `created_by` (bind `key.created_by`), and the UPDATE to include `created_by`.

Also update `init.sql` to add `created_by TEXT` to the `api_keys` CREATE TABLE. Remove the `ALTER TABLE api_keys ADD COLUMN created_by` from `002_users.sql` since `init.sql` now includes it. The `002_users.sql` migration should only contain:

```sql
CREATE TABLE IF NOT EXISTS users (
    id          TEXT PRIMARY KEY,
    username    TEXT NOT NULL UNIQUE,
    password    TEXT NOT NULL,
    role        TEXT NOT NULL DEFAULT 'user' CHECK(role IN ('admin', 'user')),
    enabled     BOOLEAN NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
```

Since this is a breaking change requiring a fresh database, existing databases will be recreated with the updated schema.

- [ ] **Step 2: Update providers.rs — admin only**

In `crates/api/src/management/providers.rs`, replace every `verify_admin_token(&headers, &state.admin_token)?;` with `require_admin(&headers, &state.jwt_secret)?;`.

Also remove the `use crate::extractors::verify_admin_token;` import and add `use crate::extractors::require_admin;`.

- [ ] **Step 3: Update usage.rs — authenticated, scoped**

Replace `crates/api/src/management/usage.rs`:

```rust
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;
use std::sync::Arc;

use llm_gateway_storage::{UsageFilter, UsageRecord};

use crate::error::ApiError;
use crate::extractors::require_auth;
use crate::AppState;

pub async fn get_usage(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(filter): Query<UsageFilter>,
) -> Result<Json<Vec<UsageRecord>>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    // Non-admin users can only see their own usage
    let effective_filter = if claims.role != "admin" {
        let mut scoped = filter.clone();
        // If user doesn't have their own keys, return empty
        let keys = state
            .storage
            .list_keys()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        let user_key_ids: Vec<String> = keys
            .iter()
            .filter(|k| k.created_by.as_deref() == Some(&claims.sub))
            .map(|k| k.id.clone())
            .collect();
        if user_key_ids.is_empty() {
            return Ok(Json(vec![]));
        }
        // Scope to user's keys if no key_id filter set
        if scoped.key_id.is_none() {
            // Can't filter by multiple key_ids with current API — return all and filter
        }
        scoped
    } else {
        filter
    };

    let records = state
        .storage
        .query_usage(&effective_filter)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Post-filter for non-admin: only show usage for their own keys
    let filtered = if claims.role != "admin" {
        let keys = state
            .storage
            .list_keys()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        let user_key_ids: std::collections::HashSet<String> = keys
            .iter()
            .filter(|k| k.created_by.as_deref() == Some(&claims.sub))
            .map(|k| k.id.clone())
            .collect();
        records
            .into_iter()
            .filter(|r| user_key_ids.contains(&r.key_id))
            .collect()
    } else {
        records
    };

    Ok(Json(filtered))
}
```

- [ ] **Step 4: Update logs.rs — admin only**

In `crates/api/src/management/logs.rs`, replace `verify_admin_token(&headers, &state.admin_token)?;` with `require_admin(&headers, &state.jwt_secret)?;`.

Update imports: remove `use crate::extractors::verify_admin_token;`, add `use crate::extractors::require_admin;`.

- [ ] **Step 5: Run cargo check**

Run: `cargo check --workspace`
Expected: Errors for `users` and `settings` modules that don't exist yet — fixed in Task 6.

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/management/keys.rs crates/api/src/management/providers.rs crates/api/src/management/usage.rs crates/api/src/management/logs.rs crates/storage/
git commit -m "feat(api): switch management handlers from admin_token to JWT role-based auth"
```

---

### Task 6: Add users and settings endpoints

**Files:**
- Create: `crates/api/src/management/users.rs`
- Create: `crates/api/src/management/settings.rs`

- [ ] **Step 1: Create users endpoint handlers**

Create `crates/api/src/management/users.rs`:

```rust
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Serialize;
use std::sync::Arc;

use llm_gateway_storage::{UpdateUser as StorageUpdateUser, User};

use crate::error::ApiError;
use crate::extractors::require_admin;
use crate::AppState;

#[derive(Serialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub role: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&User> for UserResponse {
    fn from(u: &User) -> Self {
        UserResponse {
            id: u.id.clone(),
            username: u.username.clone(),
            role: u.role.clone(),
            enabled: u.enabled,
            created_at: u.created_at.to_rfc3339(),
            updated_at: u.updated_at.to_rfc3339(),
        }
    }
}

pub async fn list_users(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<UserResponse>>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let users = state
        .storage
        .list_users()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(users.iter().map(UserResponse::from).collect()))
}

pub async fn update_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<StorageUpdateUser>,
) -> Result<Json<UserResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let mut user = state
        .storage
        .get_user(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("User '{}' not found", id)))?;

    // If disabling or demoting from admin, check it's not the last admin
    if let Some(false) = input.enabled {
        if user.role == "admin" {
            let admin_count = state
                .storage
                .count_admin_users()
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;
            if admin_count <= 1 {
                return Err(ApiError::BadRequest("Cannot disable the last admin user".to_string()));
            }
        }
    }

    if let Some(ref role) = input.role {
        if user.role == "admin" && role != "admin" {
            let admin_count = state
                .storage
                .count_admin_users()
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;
            if admin_count <= 1 {
                return Err(ApiError::BadRequest("Cannot demote the last admin user".to_string()));
            }
        }
        user.role = role.clone();
    }

    if let Some(enabled) = input.enabled {
        user.enabled = enabled;
    }

    user.updated_at = chrono::Utc::now();

    let updated = state
        .storage
        .update_user(&user)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(UserResponse::from(&updated)))
}

pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let user = state
        .storage
        .get_user(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("User '{}' not found", id)))?;

    if user.role == "admin" {
        let admin_count = state
            .storage
            .count_admin_users()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        if admin_count <= 1 {
            return Err(ApiError::BadRequest("Cannot delete the last admin user".to_string()));
        }
    }

    state
        .storage
        .delete_user(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
```

- [ ] **Step 2: Create settings endpoint handlers**

Create `crates/api/src/management/settings.rs`:

```rust
use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use crate::error::ApiError;
use crate::extractors::require_admin;
use crate::AppState;

#[derive(Deserialize)]
pub struct UpdateSettingsRequest {
    pub allow_registration: bool,
}

#[derive(serde::Serialize)]
pub struct SettingsResponse {
    pub allow_registration: bool,
}

pub async fn get_settings(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<SettingsResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let value = state
        .storage
        .get_setting("allow_registration")
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let allow_registration = value
        .map(|v| v == "true")
        .unwrap_or(true);

    Ok(Json(SettingsResponse { allow_registration }))
}

pub async fn update_settings(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<UpdateSettingsRequest>,
) -> Result<Json<SettingsResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    state
        .storage
        .set_setting("allow_registration", if input.allow_registration { "true" } else { "false" })
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(SettingsResponse {
        allow_registration: input.allow_registration,
    }))
}
```

- [ ] **Step 3: Run cargo check**

Run: `cargo check --workspace`
Expected: Compiles successfully

- [ ] **Step 4: Run all backend tests**

Run: `cargo test --workspace`
Expected: Existing tests fail because they use old `admin_token` in `make_state`. Fix test helpers in Task 7.

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/management/users.rs crates/api/src/management/settings.rs
git commit -m "feat(api): add user management and settings endpoints"
```

---

### Task 7: Update backend tests for JWT auth

**Files:**
- Modify: `crates/api/tests/common/mod.rs`
- Modify: `crates/api/tests/test_management_keys.rs`
- Modify: `crates/api/tests/test_management_providers.rs`

- [ ] **Step 1: Update test helpers**

Replace `crates/api/tests/common/mod.rs`:

```rust
use llm_gateway_storage::{sqlite::SqliteStorage, Storage};
use llm_gateway_auth::create_jwt;
use std::sync::Arc;

pub const TEST_JWT_SECRET: &str = "test-jwt-secret";

pub struct TestUser {
    pub id: String,
    pub username: String,
    pub role: String,
    pub token: String,
}

pub async fn setup_test_db() -> Arc<SqliteStorage> {
    let storage = SqliteStorage::new(":memory:").await.unwrap();
    storage.run_migrations().await.unwrap();
    Arc::new(storage)
}

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

- [ ] **Step 2: Update test_management_keys.rs**

Replace `crates/api/tests/test_management_keys.rs`. Key changes:
- `make_state` uses `jwt_secret` instead of `admin_token`
- All requests use `make_admin_token().token` instead of `"test-token"`
- Add test for user-scoped key access

Full file:

```rust
mod common;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use llm_gateway_api::management;
use llm_gateway_api::AppState;
use llm_gateway_audit::AuditLogger;
use llm_gateway_ratelimit::RateLimiter;
use llm_gateway_storage::Storage;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

fn build_app(state: Arc<AppState>) -> axum::Router {
    management::management_router().with_state(state)
}

fn make_state(db: Arc<llm_gateway_storage::sqlite::SqliteStorage>) -> Arc<AppState> {
    Arc::new(AppState {
        storage: db.clone() as Arc<dyn Storage>,
        rate_limiter: Arc::new(RateLimiter::new(60)),
        audit_logger: Arc::new(AuditLogger::new(db as Arc<dyn Storage>)),
        jwt_secret: common::TEST_JWT_SECRET.to_string(),
    })
}

fn auth_header(token: &str) -> (&'static str, &str) {
    ("authorization", &format!("Bearer {}", token))
}

#[tokio::test]
async fn test_create_key() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));
    let admin = common::make_admin_token();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/keys")
                .header(auth_header(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "test-key"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &to_bytes(resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(body["name"], "test-key");
    assert!(body["key"].is_string());
    assert_eq!(body["enabled"], true);
}

#[tokio::test]
async fn test_list_keys() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));
    let admin = common::make_admin_token();

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/keys")
                .header(auth_header(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "key1"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/keys")
                .header(auth_header(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &to_bytes(resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert!(body.is_array());
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_unauthorized_access() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/keys")
                .header("authorization", "Bearer invalid-jwt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_update_key() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));
    let admin = common::make_admin_token();

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/keys")
                .header(auth_header(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "original"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(
        &to_bytes(create_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    let key_id = body["id"].as_str().unwrap();

    let update_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/v1/keys/{}", key_id))
                .header(auth_header(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "updated"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(update_resp.status(), StatusCode::OK);
    let updated: Value = serde_json::from_slice(
        &to_bytes(update_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(updated["name"], "updated");
}

#[tokio::test]
async fn test_delete_key() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));
    let admin = common::make_admin_token();

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/keys")
                .header(auth_header(&admin.token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "to-delete"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(
        &to_bytes(create_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    let key_id = body["id"].as_str().unwrap();

    let delete_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/v1/keys/{}", key_id))
                .header(auth_header(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_resp.status(), StatusCode::NO_CONTENT);

    let get_resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v1/keys/{}", key_id))
                .header(auth_header(&admin.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_register_first_user_is_admin() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(json!({"username": "admin", "password": "password123"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &to_bytes(resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(body["user"]["role"], "admin");
    assert!(body["token"].is_string());
}

#[tokio::test]
async fn test_login_and_me() {
    let db = common::setup_test_db().await;
    let app = build_app(make_state(db));

    // Register
    let register_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(json!({"username": "testuser", "password": "pass123"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(
        &to_bytes(register_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    let token = body["token"].as_str().unwrap();

    // Get me
    let me_resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/me")
                .header(auth_header(token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(me_resp.status(), StatusCode::OK);
    let me_body: Value = serde_json::from_slice(
        &to_bytes(me_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(me_body["username"], "testuser");
}
```

- [ ] **Step 3: Update test_management_providers.rs**

Replace all `verify_admin_token` / `admin_token` references with `jwt_secret` and JWT tokens. Same pattern as keys test — use `common::make_admin_token()` for auth headers and `common::TEST_JWT_SECRET` in state.

- [ ] **Step 4: Run all backend tests**

Run: `cargo test --workspace`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/api/tests/
git commit -m "test: update backend tests for JWT auth"
```

---

### Task 8: Frontend — AuthContext, API client, and auth API

**Files:**
- Modify: `web/src/api/client.ts`
- Create: `web/src/api/auth.ts`
- Create: `web/src/contexts/AuthContext.tsx`
- Modify: `web/src/types/index.ts`

- [ ] **Step 1: Add User and auth types**

Add to `web/src/types/index.ts`:

```typescript
export interface User {
  id: string;
  username: string;
  role: 'admin' | 'user';
}

export interface LoginRequest {
  username: string;
  password: string;
}

export interface RegisterRequest {
  username: string;
  password: string;
}

export interface AuthResponse {
  token: string;
  user: User;
}

export interface MeResponse {
  id: string;
  username: string;
  role: 'admin' | 'user';
  allow_registration: boolean;
}

export interface AuthConfigResponse {
  allow_registration: boolean;
}

export interface UserResponse {
  id: string;
  username: string;
  role: 'admin' | 'user';
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface UpdateUserRequest {
  role?: 'admin' | 'user';
  enabled?: boolean;
}

export interface SettingsResponse {
  allow_registration: boolean;
}

export interface UpdateSettingsRequest {
  allow_registration: boolean;
}
```

- [ ] **Step 2: Create auth API functions**

Create `web/src/api/auth.ts`:

```typescript
import { apiClient } from './client';
import type { AuthResponse, LoginRequest, MeResponse, RegisterRequest, AuthConfigResponse } from '../types';

export async function login(input: LoginRequest): Promise<AuthResponse> {
  const { data } = await apiClient.post<AuthResponse>('/auth/login', input);
  return data;
}

export async function register(input: RegisterRequest): Promise<AuthResponse> {
  const { data } = await apiClient.post<AuthResponse>('/auth/register', input);
  return data;
}

export async function getMe(): Promise<MeResponse> {
  const { data } = await apiClient.get<MeResponse>('/auth/me');
  return data;
}

export async function getAuthConfig(): Promise<AuthConfigResponse> {
  const { data } = await apiClient.get<AuthConfigResponse>('/auth/config');
  return data;
}
```

- [ ] **Step 3: Create users and settings API functions**

Create `web/src/api/users.ts`:

```typescript
import { apiClient } from './client';
import type { UserResponse, UpdateUserRequest } from '../types';

export async function listUsers(): Promise<UserResponse[]> {
  const { data } = await apiClient.get<UserResponse[]>('/users');
  return data;
}

export async function updateUser(id: string, input: UpdateUserRequest): Promise<UserResponse> {
  const { data } = await apiClient.patch<UserResponse>(`/users/${id}`, input);
  return data;
}

export async function deleteUser(id: string): Promise<void> {
  await apiClient.delete(`/users/${id}`);
}
```

Create `web/src/api/settings.ts`:

```typescript
import { apiClient } from './client';
import type { SettingsResponse, UpdateSettingsRequest } from '../types';

export async function getSettings(): Promise<SettingsResponse> {
  const { data } = await apiClient.get<SettingsResponse>('/settings');
  return data;
}

export async function updateSettings(input: UpdateSettingsRequest): Promise<SettingsResponse> {
  const { data } = await apiClient.patch<SettingsResponse>('/settings', input);
  return data;
}
```

- [ ] **Step 4: Create AuthContext**

Create `web/src/contexts/AuthContext.tsx`:

```tsx
import { createContext, useContext, useEffect } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getMe, login as apiLogin, register as apiRegister } from '../api/auth';
import { getToken, setToken, clearToken } from '../api/client';
import type { User, LoginRequest, RegisterRequest, AuthResponse } from '../types';

interface AuthContextValue {
  user: User | undefined;
  isLoading: boolean;
  login: (input: LoginRequest) => Promise<AuthResponse>;
  register: (input: RegisterRequest) => Promise<AuthResponse>;
  logout: () => void;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const queryClient = useQueryClient();

  const { data: me, isLoading } = useQuery({
    queryKey: ['me'],
    queryFn: getMe,
    retry: false,
    enabled: !!getToken(),
  });

  const login = async (input: LoginRequest) => {
    const resp = await apiLogin(input);
    setToken(resp.token);
    queryClient.invalidateQueries({ queryKey: ['me'] });
    return resp;
  };

  const register = async (input: RegisterRequest) => {
    const resp = await apiRegister(input);
    setToken(resp.token);
    queryClient.invalidateQueries({ queryKey: ['me'] });
    return resp;
  };

  const logout = () => {
    clearToken();
    queryClient.clear();
  };

  const user: User | undefined = me
    ? { id: me.id, username: me.username, role: me.role }
    : undefined;

  return (
    <AuthContext.Provider value={{ user, isLoading, login, register, logout }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error('useAuth must be used within AuthProvider');
  return ctx;
}
```

- [ ] **Step 5: Update App.tsx with AuthProvider and new routes**

Replace `web/src/App.tsx`:

```tsx
import { BrowserRouter, Routes, Route, Navigate, Outlet } from 'react-router-dom';
import { getToken } from './api/client';
import { AuthProvider, useAuth } from './contexts/AuthContext';
import Layout from './components/Layout';
import Home from './pages/Home';
import Login from './pages/Login';
import Register from './pages/Register';
import Dashboard from './pages/Dashboard';
import Keys from './pages/Keys';
import KeyDetail from './pages/KeyDetail';
import Providers from './pages/Providers';
import ProviderDetail from './pages/ProviderDetail';
import Users from './pages/Users';
import Settings from './pages/Settings';
import Usage from './pages/Usage';
import Logs from './pages/Logs';

function RequireAuth() {
  const { user, isLoading } = useAuth();
  if (isLoading) return null;
  if (!user) return <Navigate to="/admin/login" replace />;
  return <Outlet />;
}

function RequireAdmin() {
  const { user, isLoading } = useAuth();
  if (isLoading) return null;
  if (!user || user.role !== 'admin') return <Navigate to="/admin/dashboard" replace />;
  return <Outlet />;
}

function App() {
  return (
    <BrowserRouter>
      <AuthProvider>
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/admin/login" element={<Login />} />
          <Route path="/admin/register" element={<Register />} />
          <Route path="/admin" element={<Layout />}>
            <Route element={<RequireAuth />}>
              <Route path="dashboard" element={<Dashboard />} />
              <Route path="keys" element={<Keys />} />
              <Route path="keys/:id" element={<KeyDetail />} />
              <Route path="usage" element={<Usage />} />
            </Route>
            <Route element={<RequireAdmin />}>
              <Route path="providers" element={<Providers />} />
              <Route path="providers/:id" element={<ProviderDetail />} />
              <Route path="users" element={<Users />} />
              <Route path="settings" element={<Settings />} />
              <Route path="logs" element={<Logs />} />
            </Route>
            <Route index element={<Navigate to="/admin/dashboard" replace />} />
          </Route>
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </AuthProvider>
    </BrowserRouter>
  );
}

export default App;
```

- [ ] **Step 6: Run frontend build**

Run: `cd web && npm run build`
Expected: Build succeeds (new pages will be created in next task, so this may fail — create placeholder files first)

Create placeholder files:
- `web/src/pages/Register.tsx` — `export default function Register() { return null; }`
- `web/src/pages/Users.tsx` — `export default function Users() { return null; }`
- `web/src/pages/Settings.tsx` — `export default function Settings() { return null; }`

Then build again.

- [ ] **Step 7: Commit**

```bash
git add web/src/api/auth.ts web/src/api/users.ts web/src/api/settings.ts web/src/contexts/AuthContext.tsx web/src/types/index.ts web/src/App.tsx web/src/pages/Register.tsx web/src/pages/Users.tsx web/src/pages/Settings.tsx
git commit -m "feat(web): add AuthContext, auth API, and route structure with role-based access"
```

---

### Task 9: Frontend — Update Layout sidebar and Login page

**Files:**
- Modify: `web/src/components/Layout.tsx`
- Modify: `web/src/pages/Login.tsx`
- Create: `web/src/pages/Register.tsx` (replace placeholder)

- [ ] **Step 1: Update Layout sidebar with grouped menu**

Replace `web/src/components/Layout.tsx`:

```tsx
import { useState } from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import { Layout, Menu, theme, Typography, Space } from 'antd';
import {
  DashboardOutlined,
  KeyOutlined,
  CloudServerOutlined,
  BarChartOutlined,
  FileSearchOutlined,
  LogoutOutlined,
  UserOutlined,
  SettingOutlined,
  TeamOutlined,
} from '@ant-design/icons';
import { useAuth } from '../contexts/AuthContext';

const { Header, Sider, Content } = Layout;

const consoleItems = [
  { key: '/admin/dashboard', icon: <DashboardOutlined />, label: 'Dashboard' },
  { key: '/admin/keys', icon: <KeyOutlined />, label: 'API Keys' },
  { key: '/admin/usage', icon: <BarChartOutlined />, label: 'Usage' },
];

const adminItems = [
  { key: '/admin/providers', icon: <CloudServerOutlined />, label: 'Providers' },
  { key: '/admin/users', icon: <TeamOutlined />, label: 'Users' },
  { key: '/admin/settings', icon: <SettingOutlined />, label: 'Settings' },
  { key: '/admin/logs', icon: <FileSearchOutlined />, label: 'Logs' },
];

export default function AppLayout() {
  const [collapsed, setCollapsed] = useState(false);
  const navigate = useNavigate();
  const location = useLocation();
  const { user, logout } = useAuth();
  const { token: { colorBgContainer, borderRadiusLG } } = theme.useToken();

  const isAdmin = user?.role === 'admin';

  const menuItems = [
    { key: 'console', label: 'Console', type: 'group' as const, children: consoleItems },
    ...(isAdmin ? [{ key: 'admin', label: 'Admin', type: 'group' as const, children: adminItems }] : []),
  ];

  return (
    <Layout style={{ minHeight: '100vh' }}>
      <Sider collapsible collapsed={collapsed} onCollapse={setCollapsed}>
        <div style={{ height: 32, margin: 16, textAlign: 'center', color: '#fff', fontSize: collapsed ? 14 : 16, fontWeight: 'bold' }}>
          {collapsed ? 'GW' : 'LLM Gateway'}
        </div>
        <Menu
          theme="dark"
          mode="inline"
          selectedKeys={[location.pathname]}
          items={menuItems}
          onClick={({ key }) => navigate(key)}
        />
      </Sider>
      <Layout>
        <Header style={{ padding: '0 16px', background: colorBgContainer, display: 'flex', justifyContent: 'flex-end', alignItems: 'center' }}>
          <Space size="middle">
            <span><UserOutlined /> {user?.username}</span>
            <a onClick={logout} style={{ cursor: 'pointer' }}>
              <LogoutOutlined /> Logout
            </a>
          </Space>
        </Header>
        <Content style={{ margin: 16 }}>
          <div style={{ padding: 24, minHeight: 360, background: colorBgContainer, borderRadius: borderRadiusLG }}>
            <Outlet />
          </div>
        </Content>
      </Layout>
    </Layout>
  );
}
```

- [ ] **Step 2: Update Login page with username/password**

Replace `web/src/pages/Login.tsx`:

```tsx
import { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import { Form, Input, Button, Card, Typography, message, Space } from 'antd';
import { useAuth } from '../contexts/AuthContext';
import { useQuery } from '@tanstack/react-query';
import { getAuthConfig } from '../api/auth';

const { Title, Text } = Typography;

export default function Login() {
  const [loading, setLoading] = useState(false);
  const navigate = useNavigate();
  const { login } = useAuth();

  const { data: authConfig } = useQuery({
    queryKey: ['authConfig'],
    queryFn: getAuthConfig,
    retry: false,
  });

  const onFinish = async (values: { username: string; password: string }) => {
    setLoading(true);
    try {
      await login(values);
      navigate('/admin/dashboard');
    } catch {
      message.error('Invalid username or password');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', minHeight: '100vh' }}>
      <Card style={{ width: 400 }}>
        <Title level={3} style={{ textAlign: 'center' }}>LLM Gateway</Title>
        <Form onFinish={onFinish} layout="vertical">
          <Form.Item name="username" label="Username" rules={[{ required: true, message: 'Enter your username' }]}>
            <Input placeholder="Username" />
          </Form.Item>
          <Form.Item name="password" label="Password" rules={[{ required: true, message: 'Enter your password' }]}>
            <Input.Password placeholder="Password" />
          </Form.Item>
          <Form.Item>
            <Button type="primary" htmlType="submit" loading={loading} block>
              Login
            </Button>
          </Form.Item>
        </Form>
        {authConfig?.allow_registration && (
          <Text style={{ display: 'block', textAlign: 'center' }}>
            <Link to="/admin/register">Create an account</Link>
          </Text>
        )}
      </Card>
    </div>
  );
}
```

- [ ] **Step 3: Create Register page**

Replace `web/src/pages/Register.tsx`:

```tsx
import { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import { Form, Input, Button, Card, Typography, message, Alert } from 'antd';
import { useAuth } from '../contexts/AuthContext';
import { useQuery } from '@tanstack/react-query';
import { getAuthConfig } from '../api/auth';

const { Title, Text } = Typography;

export default function Register() {
  const [loading, setLoading] = useState(false);
  const navigate = useNavigate();
  const { register } = useAuth();

  const { data: authConfig } = useQuery({
    queryKey: ['authConfig'],
    queryFn: getAuthConfig,
    retry: false,
  });

  const registrationDisabled = authConfig !== undefined && !authConfig.allow_registration;

  const onFinish = async (values: { username: string; password: string }) => {
    setLoading(true);
    try {
      await register(values);
      navigate('/admin/dashboard');
    } catch (err: unknown) {
      const error = err as { response?: { data?: { error?: { message?: string } } } };
      const msg = error.response?.data?.error?.message || 'Registration failed';
      message.error(msg);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', minHeight: '100vh' }}>
      <Card style={{ width: 400 }}>
        <Title level={3} style={{ textAlign: 'center' }}>Create Account</Title>
        {registrationDisabled && (
          <Alert message="Registration is currently disabled" type="warning" style={{ marginBottom: 16 }} />
        )}
        <Form onFinish={onFinish} layout="vertical">
          <Form.Item name="username" label="Username" rules={[
            { required: true, message: 'Enter a username' },
            { min: 3, message: 'Username must be at least 3 characters' },
          ]}>
            <Input placeholder="Username" disabled={registrationDisabled} />
          </Form.Item>
          <Form.Item name="password" label="Password" rules={[
            { required: true, message: 'Enter a password' },
            { min: 6, message: 'Password must be at least 6 characters' },
          ]}>
            <Input.Password placeholder="Password" disabled={registrationDisabled} />
          </Form.Item>
          <Form.Item name="confirm" label="Confirm Password" dependencies={['password']} rules={[
            { required: true, message: 'Confirm your password' },
            ({ getFieldValue }) => ({
              validator(_, value) {
                if (!value || getFieldValue('password') === value) return Promise.resolve();
                return Promise.reject(new Error('Passwords do not match'));
              },
            }),
          ]}>
            <Input.Password placeholder="Confirm password" disabled={registrationDisabled} />
          </Form.Item>
          <Form.Item>
            <Button type="primary" htmlType="submit" loading={loading} block disabled={registrationDisabled}>
              Register
            </Button>
          </Form.Item>
        </Form>
        <Text style={{ display: 'block', textAlign: 'center' }}>
          <Link to="/admin/login">Already have an account? Login</Link>
        </Text>
      </Card>
    </div>
  );
}
```

- [ ] **Step 4: Update MSW server for auth endpoints**

Add to `web/src/test/server.ts`:

```typescript
import { setupServer } from 'msw/node';
import { http, HttpResponse } from 'msw';

export const server = setupServer(
  http.get('*/api/v1/auth/config', () => {
    return HttpResponse.json({ allow_registration: true });
  }),
  http.get('*/api/v1/auth/me', () => {
    return HttpResponse.json({
      id: 'user-1',
      username: 'admin',
      role: 'admin',
      allow_registration: true,
    });
  }),
  http.post('*/api/v1/auth/login', () => {
    return HttpResponse.json({
      token: 'test-jwt-token',
      user: { id: 'user-1', username: 'admin', role: 'admin' },
    });
  }),
  http.post('*/api/v1/auth/register', () => {
    return HttpResponse.json({
      token: 'test-jwt-token',
      user: { id: 'user-1', username: 'admin', role: 'admin' },
    });
  }),
  http.get('*/api/v1/keys', () => {
    return HttpResponse.json([]);
  }),
  http.get('*/api/v1/providers', () => {
    return HttpResponse.json([]);
  }),
  http.get('*/api/v1/users', () => {
    return HttpResponse.json([]);
  }),
  http.get('*/api/v1/settings', () => {
    return HttpResponse.json({ allow_registration: true });
  }),
  http.get('*/api/v1/usage', () => {
    return HttpResponse.json([]);
  }),
  http.get('*/api/v1/logs', () => {
    return HttpResponse.json([]);
  }),
);
```

- [ ] **Step 5: Run frontend tests**

Run: `cd web && npm test`
Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add web/src/components/Layout.tsx web/src/pages/Login.tsx web/src/pages/Register.tsx web/src/test/server.ts
git commit -m "feat(web): update sidebar groups, login page, and add register page"
```

---

### Task 10: Frontend — Users and Settings pages

**Files:**
- Create: `web/src/pages/Users.tsx` (replace placeholder)
- Create: `web/src/pages/Settings.tsx` (replace placeholder)
- Create: `web/src/hooks/useUsers.ts`
- Create: `web/src/hooks/useSettings.ts`

- [ ] **Step 1: Create useUsers hook**

Create `web/src/hooks/useUsers.ts`:

```typescript
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listUsers, updateUser, deleteUser } from '../api/users';
import type { UpdateUserRequest } from '../types';
import { message } from 'antd';

export function useUsers() {
  return useQuery({ queryKey: ['users'], queryFn: listUsers });
}

export function useUpdateUser() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateUserRequest }) => updateUser(id, input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['users'] }); message.success('User updated'); },
    onError: () => { message.error('Failed to update user'); },
  });
}

export function useDeleteUser() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteUser(id),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['users'] }); message.success('User deleted'); },
    onError: () => { message.error('Failed to delete user'); },
  });
}
```

- [ ] **Step 2: Create useSettings hook**

Create `web/src/hooks/useSettings.ts`:

```typescript
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { getSettings, updateSettings } from '../api/settings';
import type { UpdateSettingsRequest } from '../types';
import { message } from 'antd';

export function useSettings() {
  return useQuery({ queryKey: ['settings'], queryFn: getSettings });
}

export function useUpdateSettings() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: UpdateSettingsRequest) => updateSettings(input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['settings'] }); message.success('Settings updated'); },
    onError: () => { message.error('Failed to update settings'); },
  });
}
```

- [ ] **Step 3: Create Users page**

Replace `web/src/pages/Users.tsx`:

```tsx
import { Table, Tag, Button, Popconfirm, Select, Typography, Space } from 'antd';
import { useUsers, useUpdateUser, useDeleteUser } from '../hooks/useUsers';
import type { UserResponse } from '../types';

const { Title } = Typography;

export default function Users() {
  const { data: users, isLoading } = useUsers();
  const updateMutation = useUpdateUser();
  const deleteMutation = useDeleteUser();

  const columns = [
    {
      title: 'Username',
      dataIndex: 'username',
      key: 'username',
    },
    {
      title: 'Role',
      dataIndex: 'role',
      key: 'role',
      render: (role: string, record: UserResponse) => (
        <Select
          value={role}
          size="small"
          style={{ width: 100 }}
          onChange={(value) => updateMutation.mutate({ id: record.id, input: { role: value } })}
          options={[
            { value: 'admin', label: 'Admin' },
            { value: 'user', label: 'User' },
          ]}
        />
      ),
    },
    {
      title: 'Status',
      dataIndex: 'enabled',
      key: 'enabled',
      render: (enabled: boolean, record: UserResponse) => (
        <Tag
          color={enabled ? 'green' : 'red'}
          style={{ cursor: 'pointer' }}
          onClick={() => updateMutation.mutate({ id: record.id, input: { enabled: !enabled } })}
        >
          {enabled ? 'Enabled' : 'Disabled'}
        </Tag>
      ),
    },
    {
      title: 'Created',
      dataIndex: 'created_at',
      key: 'created_at',
      render: (v: string) => new Date(v).toLocaleDateString(),
    },
    {
      title: 'Actions',
      key: 'actions',
      render: (_: unknown, record: UserResponse) => (
        <Popconfirm
          title="Delete this user?"
          onConfirm={() => deleteMutation.mutate(record.id)}
          okText="Delete"
          cancelText="Cancel"
        >
          <Button danger size="small">Delete</Button>
        </Popconfirm>
      ),
    },
  ];

  return (
    <div>
      <Title level={4}>Users</Title>
      <Table dataSource={users} columns={columns} rowKey="id" loading={isLoading} />
    </div>
  );
}
```

- [ ] **Step 4: Create Settings page**

Replace `web/src/pages/Settings.tsx`:

```tsx
import { Typography, Switch, Card } from 'antd';
import { useSettings, useUpdateSettings } from '../hooks/useSettings';

const { Title } = Typography;

export default function Settings() {
  const { data: settings, isLoading } = useSettings();
  const updateMutation = useUpdateSettings();

  return (
    <div>
      <Title level={4}>Settings</Title>
      <Card loading={isLoading}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', maxWidth: 400 }}>
          <span>Allow Registration</span>
          <Switch
            checked={settings?.allow_registration ?? false}
            onChange={(checked) => updateMutation.mutate({ allow_registration: checked })}
          />
        </div>
      </Card>
    </div>
  );
}
```

- [ ] **Step 5: Run frontend tests**

Run: `cd web && npm test`
Expected: All tests pass

- [ ] **Step 6: Run frontend build**

Run: `cd web && npm run build`
Expected: Build succeeds

- [ ] **Step 7: Commit**

```bash
git add web/src/pages/Users.tsx web/src/pages/Settings.tsx web/src/hooks/useUsers.ts web/src/hooks/useSettings.ts
git commit -m "feat(web): add Users and Settings pages with hooks"
```

---

### Task 11: Update existing frontend tests

**Files:**
- Modify: `web/src/pages/Login.test.tsx`
- Modify: `web/src/test/render.tsx`
- Modify: `web/src/api/keys.test.ts`
- Modify: `web/src/api/providers.test.ts`

- [ ] **Step 1: Update render.tsx to include AuthProvider**

Replace `web/src/test/render.tsx`:

```tsx
import { render } from '@testing-library/react';
import type { ReactElement, ReactNode } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import { AuthProvider } from '../contexts/AuthContext';

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
  });
}

export function renderWithProviders(
  ui: ReactElement,
  options?: { route?: string; queryClient?: QueryClient },
) {
  const queryClient = options?.queryClient ?? createTestQueryClient();
  const route = options?.route ?? '/';

  function Wrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        <AuthProvider>
          <MemoryRouter initialEntries={[route]}>
            {children}
          </MemoryRouter>
        </AuthProvider>
      </QueryClientProvider>
    );
  }

  return { ...render(ui, { wrapper: Wrapper }), queryClient };
}

export { render };
```

- [ ] **Step 2: Update Login tests for username/password form**

Replace `web/src/pages/Login.test.tsx`:

```tsx
Replace `web/src/pages/Login.test.tsx`:

```tsx
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { http, HttpResponse } from 'msw';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import Login from './Login';

describe('Login', () => {
  test('renders login form with username and password fields', () => {
    renderWithProviders(<Login />);
    expect(screen.getByLabelText('Username')).toBeInTheDocument();
    expect(screen.getByLabelText('Password')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Login' })).toBeInTheDocument();
  });

  test('shows error on invalid credentials', async () => {
    server.use(
      http.post('*/api/v1/auth/login', () => {
        return HttpResponse.json(
          { error: { message: 'Invalid credentials' } },
          { status: 401 },
        );
      }),
    );
    const user = userEvent.setup();
    renderWithProviders(<Login />);
    await user.type(screen.getByLabelText('Username'), 'wrong');
    await user.type(screen.getByLabelText('Password'), 'wrong');
    await user.click(screen.getByRole('button', { name: 'Login' }));
    await waitFor(() => {
      expect(screen.getByText('Invalid username or password')).toBeInTheDocument();
    });
  });

  test('valid login redirects to dashboard', async () => {
    const user = userEvent.setup();
    renderWithProviders(<Login />, { route: '/admin/login' });
    await user.type(screen.getByLabelText('Username'), 'admin');
    await user.type(screen.getByLabelText('Password'), 'password');
    await user.click(screen.getByRole('button', { name: 'Login' }));
    await waitFor(() => {
      expect(screen.getByText('admin')).toBeInTheDocument();
    });
  });
});
```

Note: The navigation test uses `screen.getByText('admin')` to check the Layout header shows the username after login. The MemoryRouter won't actually change the URL, but the AuthContext will have the user data.

- [ ] **Step 3: Run all frontend tests**

Run: `cd web && npm test`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add web/src/test/render.tsx web/src/pages/Login.test.tsx
git commit -m "test(web): update Login tests for username/password auth"
```

---

### Task 12: Update CI and final verification

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Update CI workflow**

No changes needed — the existing CI already runs `npm test` and `npm run build` for frontend, and `cargo test --workspace` for backend. The new tests and builds are automatically included.

- [ ] **Step 2: Run full backend test suite**

Run: `cargo test --workspace`
Expected: All tests pass

- [ ] **Step 3: Run full frontend test suite**

Run: `cd web && npm test`
Expected: All tests pass

- [ ] **Step 4: Run frontend build**

Run: `cd web && npm run build`
Expected: Build succeeds

- [ ] **Step 5: Commit and verify**

```bash
git add -A
git status
```

If there are any uncommitted changes, commit them. Otherwise, the implementation is complete.
