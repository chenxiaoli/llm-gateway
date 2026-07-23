# `model=auto` Capability-Aware Routing — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `model=auto` routing mode that lets clients omit a specific model name and have the gateway pick one based on the request's required capabilities (`vision`, `tools`) from a per-key admin-defined candidate pool.

**Architecture:** Two new migrations add capability columns to `models` and a new `auto_route_configs` platform-level table mirrored on `model_fallbacks`. Per-key binding via `api_keys.auto_route_id`. At request time, `model == "auto"` triggers capability detection from the request body, filters the config's model pool by capability, and feeds the resulting multi-model candidate set into the existing channel-priority + weighted-routing machinery.

**Tech Stack:** Rust (sqlx + axum + async-trait), React + TypeScript + Vite + Vitest + React Query.

**Spec:** `docs/superpowers/specs/2026-07-22-model-auto-routing-design.md`

**Branch:** `develop` (work directly per user instruction).

---

## File Structure

**Backend (Rust):**

| File | Responsibility |
|---|---|
| `crates/storage/migrations/postgres/20260803000001_models_capabilities.sql` (new) | Add `supports_vision`, `supports_tools` BOOLEAN columns to `models` |
| `crates/storage/migrations/postgres/20260803000002_auto_route_configs.sql` (new) | Create `auto_route_configs` table + `api_keys.auto_route_id` FK |
| `crates/storage/src/types.rs` (modify) | Extend `Model`; add `AutoRouteConfig*` types; extend `ApiKey` |
| `crates/storage/src/lib.rs` (modify) | Add 6 `Storage` trait methods |
| `crates/storage/src/postgres.rs` (modify) | Implement new methods; update `Model` row mapping + INSERT/UPDATE sites |
| `crates/api/src/auto_route.rs` (new) | `CapabilitySet`, `detect_required_capabilities` |
| `crates/api/src/error.rs` (modify) | 4 new `ApiError` variants |
| `crates/api/src/management/auto_routes.rs` (new) | 5 CRUD handlers |
| `crates/api/src/management/mod.rs` (modify) | Register new module + routes |
| `crates/api/src/management/models.rs` (modify) | Reject name `auto`; extend `UpdateModel` for capability flags |
| `crates/api/src/management/api_keys.rs` (modify) | `auto_route_id` in DTOs + persistence |
| `crates/api/src/lib.rs` (modify) | `pub mod auto_route;` |
| `crates/api/src/proxy.rs` (modify) | `model == "auto"` branch; `resolve_by_pool` |
| `crates/api/src/channel_registry.rs` or `proxy.rs` (modify) | `resolve_by_pool` trait method |
| `crates/api/tests/test_auto_route.rs` (new) | Integration tests |

**Frontend (React):**

| File | Responsibility |
|---|---|
| `web/src/types/index.ts` (modify) | Extend `Model`, `ApiKey`, DTOs; add `AutoRouteConfig` types |
| `web/src/api/autoRoutes.ts` (new) | CRUD client |
| `web/src/hooks/useAutoRouteConfigs.ts` (new) | React Query hooks |
| `web/src/pages/AutoRoutes.tsx` (new) | Admin CRUD page |
| `web/src/pages/Models.tsx` (modify) | Two capability checkboxes per row |
| `web/src/pages/Keys.tsx` (modify) | Auto-route selector |
| `web/src/components/Layout.tsx` (modify) | Sidebar entry under Admin group |
| `web/src/App.tsx` (modify) | Route registration |
| `web/src/i18n/en.json` + `zh.json` (modify) | New strings |

**Top-level:**

| File | Responsibility |
|---|---|
| `CHANGELOG.md` (modify) | Feature entry under `### Added` |

---

## Task 1: Storage migrations

**Files:**
- Create: `crates/storage/migrations/postgres/20260803000001_models_capabilities.sql`
- Create: `crates/storage/migrations/postgres/20260803000002_auto_route_configs.sql`

- [ ] **Step 1: Write the first migration**

`crates/storage/migrations/postgres/20260803000001_models_capabilities.sql`:

```sql
ALTER TABLE models
  ADD COLUMN IF NOT EXISTS supports_vision BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN IF NOT EXISTS supports_tools  BOOLEAN NOT NULL DEFAULT FALSE;
```

- [ ] **Step 2: Write the second migration**

`crates/storage/migrations/postgres/20260803000002_auto_route_configs.sql`:

```sql
CREATE TABLE IF NOT EXISTS auto_route_configs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    config TEXT NOT NULL,
    created_by TEXT,
    created_at TIMESTAMPTZ NOT NULL
);

ALTER TABLE api_keys ADD COLUMN IF NOT EXISTS auto_route_id TEXT REFERENCES auto_route_configs(id);
```

- [ ] **Step 3: Verify migrations run cleanly**

Run: `cargo test --workspace -p llm-gateway-storage -- --nocapture 2>&1 | tail -40`

Expected: all existing storage tests still pass (migrations apply on the test DB). No new test yet — Task 7 covers it.

- [ ] **Step 4: Commit**

```bash
git add crates/storage/migrations/postgres/20260803000001_models_capabilities.sql \
        crates/storage/migrations/postgres/20260803000002_auto_route_configs.sql
git commit -m "feat(storage): migrations for model capabilities + auto_route_configs"
```

---

## Task 2: Storage types

**Files:**
- Modify: `crates/storage/src/types.rs` — extend `Model` (around line 513), add `AutoRouteConfig*` types near `ModelFallbackConfig` (around line 1305), extend `ApiKey` struct.

- [ ] **Step 1: Extend `Model` struct**

In `crates/storage/src/types.rs`, find the `Model` struct (currently lines 512-520) and add the two new fields:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Model {
    pub id: String,           // primary key
    pub owner_org_id: Option<String>,
    pub name: String,          // display name
    pub model_type: Option<String>,
    pub pricing_policy_id: Option<String>,
    pub supports_vision: bool,
    pub supports_tools: bool,
    pub created_at: DateTime<Utc>,
}
```

- [ ] **Step 2: Add `supports_vision` / `supports_tools` to `UpdateModel`**

The existing `UpdateModel` struct at `crates/storage/src/types.rs:724-727` is currently:

```rust
#[derive(Debug, Deserialize)]
pub struct UpdateModel {
    pub pricing_policy_id: Option<Option<String>>,  // None=keep, Some(None)=clear
}
```

Extend it to:

```rust
#[derive(Debug, Deserialize)]
pub struct UpdateModel {
    pub pricing_policy_id: Option<Option<String>>,  // None=keep, Some(None)=clear
    pub supports_vision: Option<bool>,
    pub supports_tools: Option<bool>,
}
```

- [ ] **Step 3: Extend `ApiKey` struct**

Find `pub struct ApiKey` (search the file). Add `auto_route_id: Option<String>` next to the existing `model_fallback_id`:

```rust
pub struct ApiKey {
    // ...existing fields...
    pub model_fallback_id: Option<String>,
    pub auto_route_id: Option<String>,
    // ...existing fields...
}
```

- [ ] **Step 4: Add `AutoRouteConfig*` types near `ModelFallbackConfig`**

After the `ModelFallbackConfig` / `UpdateModelFallback` block (around line 1324), add:

```rust
// --- Auto Route Config ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoRouteConfigData {
    pub model_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoRouteConfig {
    pub id: String,
    pub name: String,
    pub config: AutoRouteConfigData,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAutoRouteConfig {
    pub name: String,
    pub config: AutoRouteConfigData,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAutoRouteConfig {
    pub name: Option<String>,
    pub config: Option<AutoRouteConfigData>,
}
```

Note: `AutoRouteConfigData` is a separate struct (not a bare `Vec<String>`) because the config JSON is an object `{"model_names": [...]}` — keeping it as a struct makes serialization unambiguous and leaves room for future fields without a schema break.

- [ ] **Step 5: Build to catch type errors**

Run: `cargo build -p llm-gateway-storage 2>&1 | tail -30`

Expected: COMPILE FAILURES at every site that constructs `Model { ... }` or `ApiKey { ... }` literally (they're missing the new fields). Do not fix these yet — Task 4 fixes the model-construction sites; api_keys construction sites will be fixed in their respective tasks. Note the list of failure sites for later.

- [ ] **Step 6: Commit (work in progress — code won't compile yet)**

We commit at the end of Task 4 once everything compiles. Skip the commit step here.

---

## Task 3: Storage trait methods

**Files:**
- Modify: `crates/storage/src/lib.rs` — add 6 trait methods after the existing `// Model Fallbacks` block.

- [ ] **Step 1: Add trait methods**

In `crates/storage/src/lib.rs`, find the existing `// Model Fallbacks` block in the `Storage` trait. After it (before `// Settings` or the next section), add:

```rust
    // Auto Route Configs
    async fn get_auto_route_config(
        &self,
        id: &str,
    ) -> Result<Option<AutoRouteConfig>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_auto_route_configs(
        &self,
    ) -> Result<Vec<AutoRouteConfig>, Box<dyn std::error::Error + Send + Sync>>;
    async fn create_auto_route_config(
        &self,
        config: &AutoRouteConfig,
    ) -> Result<AutoRouteConfig, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_auto_route_config(
        &self,
        config: &AutoRouteConfig,
    ) -> Result<AutoRouteConfig, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_auto_route_config(
        &self,
        id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    /// Returns models in `candidate_names` that belong to `org_id` and satisfy
    /// the required capabilities. When `require_vision` is false, the
    /// `supports_vision` filter is NOT applied (so text-only models stay
    /// eligible). Same for `require_tools`.
    async fn list_models_with_capabilities(
        &self,
        org_id: &str,
        require_vision: bool,
        require_tools: bool,
        candidate_names: &[String],
    ) -> Result<Vec<Model>, Box<dyn std::error::Error + Send + Sync>>;
```

- [ ] **Step 2: Re-export new types from the storage crate root**

In `crates/storage/src/lib.rs`, the existing `pub use types::{ *, ... }` glob already covers the new types because they live in `types.rs`. Confirm by checking the glob is intact.

- [ ] **Step 3: Build the storage crate**

Run: `cargo build -p llm-gateway-storage 2>&1 | tail -30`

Expected: trait-method-not-satisfied errors at `PostgresStorage` (the impl doesn't have these methods yet — Task 4-6 add them). Combined with Task 2's literal-constructor failures, the crate is in a deliberately broken state.

- [ ] **Step 4: Skip commit — code still WIP**

---

## Task 4: Postgres impl — model CRUD round-trips new columns

**Files:**
- Modify: `crates/storage/src/postgres.rs` — `PgModelRow`, `From<PgModelRow> for Model`, `create_model`, `update_model`, every SELECT site that reads models.

- [ ] **Step 1: Update `PgModelRow` and its `From<...>` impl**

Search for `PgModelRow` in `crates/storage/src/postgres.rs`. Add the two new columns to the struct and to the `From` impl. The struct will look like:

```rust
#[derive(FromRow)]
struct PgModelRow {
    id: String,
    owner_org_id: Option<String>,
    name: String,
    model_type: Option<String>,
    pricing_policy_id: Option<String>,
    supports_vision: bool,
    supports_tools: bool,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<PgModelRow> for Model {
    fn from(r: PgModelRow) -> Self {
        Model {
            id: r.id,
            owner_org_id: r.owner_org_id,
            name: r.name,
            model_type: r.model_type,
            pricing_policy_id: r.pricing_policy_id,
            supports_vision: r.supports_vision,
            supports_tools: r.supports_tools,
            created_at: r.created_at,
        }
    }
}
```

- [ ] **Step 2: Update every `SELECT` against `models`**

Grep for `SELECT` queries touching the `models` table. Use: `grep -n "SELECT" crates/storage/src/postgres.rs | head -40` then narrow to model queries. There are roughly 4 such sites (in `get_model`, `get_model_by_id`, `list_models`, `list_models_by_provider`, plus the `list_models_with_provider` JOIN). For each, change `m.*` or explicit column lists to include `supports_vision, supports_tools`.

Pattern for `m.*` is fine — sqlx maps columns by name. If a site uses explicit columns, add the two new ones.

- [ ] **Step 3: Update `create_model` INSERT**

Find `async fn create_model` in postgres.rs. Update the INSERT statement to include the two new columns. Example pattern (match the existing style):

```rust
async fn create_model(
    &self,
    viewer_org_id: &str,
    model: &Model,
) -> Result<Model, Box<dyn std::error::Error + Send + Sync>> {
    let row = sqlx::query_as::<_, PgModelRow>(
        r#"INSERT INTO models (id, owner_org_id, name, model_type, pricing_policy_id, supports_vision, supports_tools, created_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           RETURNING id, owner_org_id, name, model_type, pricing_policy_id, supports_vision, supports_tools, created_at"#,
    )
    .bind(&model.id)
    .bind(model.owner_org_id.as_deref())
    .bind(&model.name)
    .bind(model.model_type.as_deref())
    .bind(model.pricing_policy_id.as_deref())
    .bind(model.supports_vision)
    .bind(model.supports_tools)
    .bind(model.created_at)
    .fetch_one(&self.pool)
    .await?;
    Ok(row.into())
}
```

(Adjust to match the existing code style — preserve parameter numbering, error handling, etc.)

- [ ] **Step 4: Update `update_model` UPDATE**

Find `async fn update_model`. Add the two new columns to the SET clause. Since they're `Option<bool>` in `UpdateModel`, only update when `Some`:

```rust
async fn update_model(
    &self,
    viewer_org_id: &str,
    model: &Model,
) -> Result<Model, Box<dyn std::error::Error + Send + Sync>> {
    // The handler in management/models.rs has already applied partial-update
    // semantics onto `model` before calling this — so this function just
    // persists the full row.
    let row = sqlx::query_as::<_, PgModelRow>(
        r#"UPDATE models
           SET pricing_policy_id = $3,
               supports_vision = $4,
               supports_tools = $5
           WHERE id = $1 AND owner_org_id IS NOT DISTINCT FROM $2
           RETURNING id, owner_org_id, name, model_type, pricing_policy_id, supports_vision, supports_tools, created_at"#,
    )
    .bind(&model.id)
    .bind(model.owner_org_id.as_deref())
    .bind(model.pricing_policy_id.as_deref())
    .bind(model.supports_vision)
    .bind(model.supports_tools)
    .fetch_one(&self.pool)
    .await?;
    Ok(row.into())
}
```

Verify against the existing function — it may already use `WHERE id = $1 AND owner_org_id IS NOT DISTINCT FROM $2`. Preserve existing structure.

- [ ] **Step 5: Update every site that constructs `Model { ... }` literally**

The compile errors from Task 2 Step 5 list these sites. They're typically in `seed.rs`, test helpers, and possibly elsewhere. Add `supports_vision: false, supports_tools: false,` to each.

- [ ] **Step 6: Build the storage crate**

Run: `cargo build -p llm-gateway-storage 2>&1 | tail -30`

Expected: trait-method errors only (from Task 3) — no more `Model`/`ApiKey` constructor errors.

- [ ] **Step 7: Commit**

```bash
git add crates/storage/src/types.rs crates/storage/src/lib.rs crates/storage/src/postgres.rs crates/storage/src/seed.rs
git commit -m "feat(storage): model capability columns + AutoRouteConfig types"
```

---

## Task 5: Postgres impl — `auto_route_configs` CRUD

**Files:**
- Modify: `crates/storage/src/postgres.rs`

- [ ] **Step 1: Add the row type and `From` impl**

Near the existing `PgModelFallbackRow` (search the file), add:

```rust
#[derive(FromRow)]
struct PgAutoRouteConfigRow {
    id: String,
    name: String,
    config: String,
    created_by: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<PgAutoRouteConfigRow> for AutoRouteConfig {
    fn from(r: PgAutoRouteConfigRow) -> Self {
        let config: AutoRouteConfigData = serde_json::from_str(&r.config).unwrap_or(AutoRouteConfigData {
            model_names: Vec::new(),
        });
        AutoRouteConfig {
            id: r.id,
            name: r.name,
            config,
            created_by: r.created_by,
            created_at: r.created_at,
        }
    }
}
```

- [ ] **Step 2: Implement the 5 CRUD methods**

Add to the `impl Storage for PostgresStorage` block (mirror `model_fallbacks` patterns exactly):

```rust
async fn get_auto_route_config(
    &self,
    id: &str,
) -> Result<Option<AutoRouteConfig>, Box<dyn std::error::Error + Send + Sync>> {
    let row = sqlx::query_as::<_, PgAutoRouteConfigRow>(
        r#"SELECT id, name, config, created_by, created_at
           FROM auto_route_configs WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(&self.pool)
    .await?;
    Ok(row.map(Into::into))
}

async fn list_auto_route_configs(
    &self,
) -> Result<Vec<AutoRouteConfig>, Box<dyn std::error::Error + Send + Sync>> {
    let rows = sqlx::query_as::<_, PgAutoRouteConfigRow>(
        r#"SELECT id, name, config, created_by, created_at
           FROM auto_route_configs ORDER BY created_at DESC"#,
    )
    .fetch_all(&self.pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

async fn create_auto_route_config(
    &self,
    config: &AutoRouteConfig,
) -> Result<AutoRouteConfig, Box<dyn std::error::Error + Send + Sync>> {
    let config_json = serde_json::to_string(&config.config)?;
    let row = sqlx::query_as::<_, PgAutoRouteConfigRow>(
        r#"INSERT INTO auto_route_configs (id, name, config, created_by, created_at)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id, name, config, created_by, created_at"#,
    )
    .bind(&config.id)
    .bind(&config.name)
    .bind(&config_json)
    .bind(config.created_by.as_deref())
    .bind(config.created_at)
    .fetch_one(&self.pool)
    .await?;
    Ok(row.into())
}

async fn update_auto_route_config(
    &self,
    config: &AutoRouteConfig,
) -> Result<AutoRouteConfig, Box<dyn std::error::Error + Send + Sync>> {
    let config_json = serde_json::to_string(&config.config)?;
    let row = sqlx::query_as::<_, PgAutoRouteConfigRow>(
        r#"UPDATE auto_route_configs
           SET name = $2, config = $3
           WHERE id = $1
           RETURNING id, name, config, created_by, created_at"#,
    )
    .bind(&config.id)
    .bind(&config.name)
    .bind(&config_json)
    .fetch_one(&self.pool)
    .await?;
    Ok(row.into())
}

async fn delete_auto_route_config(
    &self,
    id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    sqlx::query("DELETE FROM auto_route_configs WHERE id = $1")
        .bind(id)
        .execute(&self.pool)
        .await?;
    Ok(())
}
```

- [ ] **Step 3: Build to confirm trait impl is complete for these 5 methods**

Run: `cargo build -p llm-gateway-storage 2>&1 | tail -30`

Expected: only `list_models_with_capabilities` trait error remains.

- [ ] **Step 4: Skip commit — Task 6 also touches postgres.rs**

---

## Task 6: Postgres impl — `list_models_with_capabilities`

**Files:**
- Modify: `crates/storage/src/postgres.rs`

- [ ] **Step 1: Write the failing test first (TDD)**

Create `crates/storage/tests/auto_route_query.rs`:

```rust
use llm_gateway_storage::{PostgresStorage, Storage, Model, AutoRouteConfig};
use sqlx::PgPool;

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn list_models_with_capabilities_filters_correctly(pool: PgPool) {
    let storage = PostgresStorage::from_pool(pool);

    // Seed three models in org_default: text-only, vision-capable, both.
    let now = chrono::Utc::now();
    for (name, vision, tools) in [
        ("text-only", false, false),
        ("vision-model", true, false),
        ("full-capable", true, true),
    ] {
        let m = Model {
            id: name.to_string(),
            owner_org_id: Some("org_default".to_string()),
            name: name.to_string(),
            model_type: None,
            pricing_policy_id: None,
            supports_vision: vision,
            supports_tools: tools,
            created_at: now,
        };
        storage.create_model("org_default", &m).await.unwrap();
    }

    let candidates: Vec<String> = vec![
        "text-only".into(),
        "vision-model".into(),
        "full-capable".into(),
    ];

    // No capabilities required → all 3 eligible.
    let all = storage.list_models_with_capabilities("org_default", false, false, &candidates).await.unwrap();
    assert_eq!(all.len(), 3);

    // Vision required → 2 eligible.
    let vis = storage.list_models_with_capabilities("org_default", true, false, &candidates).await.unwrap();
    assert_eq!(vis.len(), 2);
    assert!(vis.iter().all(|m| m.supports_vision));

    // Both required → 1 eligible.
    let both = storage.list_models_with_capabilities("org_default", true, true, &candidates).await.unwrap();
    assert_eq!(both.len(), 1);
    assert_eq!(both[0].name, "full-capable");

    // Candidate pool restriction honored.
    let small_pool = storage.list_models_with_capabilities("org_default", false, false, &["text-only".to_string()]).await.unwrap();
    assert_eq!(small_pool.len(), 1);
    assert_eq!(small_pool[0].name, "text-only");
}
```

- [ ] **Step 2: Run the test — should fail to compile**

Run: `cargo test --workspace -p llm-gateway-storage --test auto_route_query 2>&1 | tail -20`

Expected: COMPILE ERROR — `list_models_with_capabilities` not yet implemented.

- [ ] **Step 3: Implement the method**

Add to `impl Storage for PostgresStorage`:

```rust
async fn list_models_with_capabilities(
    &self,
    org_id: &str,
    require_vision: bool,
    require_tools: bool,
    candidate_names: &[String],
) -> Result<Vec<Model>, Box<dyn std::error::Error + Send + Sync>> {
    let rows = sqlx::query_as::<_, PgModelRow>(
        r#"SELECT id, owner_org_id, name, model_type, pricing_policy_id,
                  supports_vision, supports_tools, created_at
           FROM models
           WHERE owner_org_id = $1
             AND (NOT $2 OR supports_vision)
             AND (NOT $3 OR supports_tools)
             AND name = ANY($4::text[])"#,
    )
    .bind(org_id)
    .bind(require_vision)
    .bind(require_tools)
    .bind(candidate_names)
    .fetch_all(&self.pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}
```

Note: `owner_org_id = $1` — the existing models table also has platform-level rows (`owner_org_id IS NULL`). Auto-routing is org-scoped per the spec (the pool comes from a per-key config), so platform-level models are NOT eligible unless explicitly added to the org. If the spec later wants platform-level inclusion, change to `(owner_org_id = $1 OR owner_org_id IS NULL)`.

- [ ] **Step 4: Run the test — should pass**

Run: `cargo test --workspace -p llm-gateway-storage --test auto_route_query 2>&1 | tail -20`

Expected: 1 test passes.

- [ ] **Step 5: Build the whole storage crate**

Run: `cargo build -p llm-gateway-storage 2>&1 | tail -10`

Expected: clean compile.

- [ ] **Step 6: Commit**

```bash
git add crates/storage/src/postgres.rs crates/storage/tests/auto_route_query.rs
git commit -m "feat(storage): auto_route_configs CRUD + list_models_with_capabilities"
```

---

## Task 7: API errors

**Files:**
- Modify: `crates/api/src/error.rs`

- [ ] **Step 1: Add 4 new variants to the `ApiError` enum**

In `crates/api/src/error.rs`, extend the enum (after `InvalidNickname`):

```rust
    // --- model=auto routing ---
    AutoNotConfigured,                                          // 400 auto_not_configured
    AutoNoMatchingModel { required_vision: bool, required_tools: bool },  // 400 auto_no_matching_model
    AutoAllCandidatesFailed,                                    // 502 auto_all_candidates_failed
    ModelNameReserved,                                          // 400 model_name_reserved
```

- [ ] **Step 2: Add 4 new match arms to `IntoResponse`**

In the `(status, message, code) = match &self` block (before the `RateLimited`/`BudgetExceeded` unreachable arms), add:

```rust
            ApiError::AutoNotConfigured => (
                StatusCode::BAD_REQUEST,
                "This API key has no auto_route_config assigned",
                Some("auto_not_configured"),
            ),
            ApiError::AutoNoMatchingModel { required_vision, required_tools } => {
                let mut caps = Vec::new();
                if *required_vision { caps.push("vision"); }
                if *required_tools { caps.push("tools"); }
                let caps_str = if caps.is_empty() { "none".to_string() } else { caps.join(",") };
                (
                    StatusCode::BAD_REQUEST,
                    // Leaked via the message; full structured body emitted below.
                    Box::leak(format!("No model in the auto-route pool satisfies the required capabilities ({})", caps_str).into_boxed_str()),
                    Some("auto_no_matching_model"),
                )
            }
            ApiError::AutoAllCandidatesFailed => (
                StatusCode::BAD_GATEWAY,
                "All candidate models failed for this auto-routed request",
                Some("auto_all_candidates_failed"),
            ),
            ApiError::ModelNameReserved => (
                StatusCode::BAD_REQUEST,
                "The model name 'auto' is reserved",
                Some("model_name_reserved"),
            ),
```

The `Box::leak` trick is ugly. Better: handle `AutoNoMatchingModel` as an early-return like `BudgetExceeded`, emitting a structured body. Replace the match arm with a non-leaky version by handling it before the flat-match:

Before the `if let ApiError::RateLimited ...` block, add:

```rust
        if let ApiError::AutoNoMatchingModel { required_vision, required_tools } = &self {
            let mut required = Vec::new();
            if *required_vision { required.push("vision"); }
            if *required_tools { required.push("tools"); }
            let body = json!({
                "error": {
                    "message": "No model in the auto-route pool satisfies the required capabilities",
                    "type": StatusCode::BAD_REQUEST.as_u16(),
                    "code": "auto_no_matching_model",
                    "required_capabilities": required,
                }
            });
            return (StatusCode::BAD_REQUEST, axum::Json(body)).into_response();
        }
```

Then in the flat match, use the simpler form:

```rust
            ApiError::AutoNoMatchingModel { .. } => unreachable!("handled above"),
```

- [ ] **Step 3: Build the api crate**

Run: `cargo build -p llm-gateway-api 2>&1 | tail -20`

Expected: clean compile.

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/error.rs
git commit -m "feat(api): error variants for model=auto routing"
```

---

## Task 8: `auto_route.rs` — capability detection

**Files:**
- Create: `crates/api/src/auto_route.rs`
- Modify: `crates/api/src/lib.rs` — register module

- [ ] **Step 1: Write the failing unit tests first (TDD)**

Create `crates/api/src/auto_route.rs` with just the test module and type stubs at the top:

```rust
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CapabilitySet {
    pub vision: bool,
    pub tools: bool,
}

/// Walk the request body to detect required capabilities. Returns the
/// empty set on any malformation — we never fail the request from this
/// function; an empty set just means "any model in the pool is eligible".
pub fn detect_required_capabilities(body: &Value) -> CapabilitySet {
    CapabilitySet::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn empty_body() {
        let caps = detect_required_capabilities(&json!({}));
        assert!(!caps.vision && !caps.tools);
    }

    #[test]
    fn openai_text_only() {
        let body = json!({
            "messages": [{"role": "user", "content": "hello"}]
        });
        let caps = detect_required_capabilities(&body);
        assert!(!caps.vision && !caps.tools);
    }

    #[test]
    fn openai_with_image_url() {
        let body = json!({
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": "what's this?"},
                    {"type": "image_url", "image_url": {"url": "https://x/y.png"}}
                ]
            }]
        });
        let caps = detect_required_capabilities(&body);
        assert!(caps.vision);
        assert!(!caps.tools);
    }

    #[test]
    fn openai_with_tools() {
        let body = json!({
            "messages": [{"role": "user", "content": "use the tool"}],
            "tools": [{"type": "function", "function": {"name": "do_it"}}]
        });
        let caps = detect_required_capabilities(&body);
        assert!(!caps.vision);
        assert!(caps.tools);
    }

    #[test]
    fn openai_with_both() {
        let body = json!({
            "messages": [{
                "role": "user",
                "content": [{"type": "image_url", "image_url": {"url": "x"}}]
            }],
            "tools": [{"type": "function", "function": {"name": "f"}}]
        });
        let caps = detect_required_capabilities(&body);
        assert!(caps.vision && caps.tools);
    }

    #[test]
    fn anthropic_with_image_block() {
        let body = json!({
            "messages": [{
                "role": "user",
                "content": [{"type": "image", "source": {"type": "base64"}}]
            }]
        });
        let caps = detect_required_capabilities(&body);
        assert!(caps.vision);
    }

    #[test]
    fn anthropic_with_tools() {
        let body = json!({
            "messages": [{"role": "user", "content": "hi"}],
            "tools": [{"name": "t", "input_schema": {}}]
        });
        let caps = detect_required_capabilities(&body);
        assert!(caps.tools);
    }

    #[test]
    fn malformed_content_string_does_not_panic() {
        let body = json!({
            "messages": [{"role": "user", "content": "just a string"}]
        });
        let _ = detect_required_capabilities(&body);
    }

    #[test]
    fn malformed_content_missing_type_does_not_panic() {
        let body = json!({
            "messages": [{"role": "user", "content": [{"no_type": true}]}]
        });
        let _ = detect_required_capabilities(&body);
    }

    #[test]
    fn empty_tools_array_does_not_trigger_tools() {
        let body = json!({
            "messages": [{"role": "user", "content": "hi"}],
            "tools": []
        });
        let caps = detect_required_capabilities(&body);
        assert!(!caps.tools);
    }
}
```

- [ ] **Step 2: Register the module**

In `crates/api/src/lib.rs`, add `pub mod auto_route;` near the existing module declarations.

- [ ] **Step 3: Run the tests — should fail (stubs return empty)**

Run: `cargo test --workspace -p llm-gateway-api auto_route::tests 2>&1 | tail -30`

Expected: 5 failures (image_url, tools, both, anthropic image, anthropic tools, empty_tools).

- [ ] **Step 4: Implement `detect_required_capabilities`**

Replace the stub:

```rust
pub fn detect_required_capabilities(body: &Value) -> CapabilitySet {
    let mut caps = CapabilitySet::default();

    // Tools: any non-empty `tools` array.
    if let Some(tools) = body.get("tools").and_then(|t| t.as_array()) {
        if !tools.is_empty() {
            caps.tools = true;
        }
    }

    // Vision: walk messages[].content looking for image blocks.
    // OpenAI uses `{"type": "image_url", ...}`; Anthropic uses `{"type": "image", ...}`.
    if let Some(messages) = body.get("messages").and_then(|m| m.as_array()) {
        for msg in messages {
            let content = match msg.get("content") {
                Some(c) => c,
                None => continue,
            };
            if let Some(arr) = content.as_array() {
                for block in arr {
                    if let Some(t) = block.get("type").and_then(|v| v.as_str()) {
                        if t == "image_url" || t == "image" {
                            caps.vision = true;
                            break;
                        }
                    }
                }
            }
            if caps.vision { break; }
        }
    }

    caps
}
```

- [ ] **Step 5: Run the tests — all should pass**

Run: `cargo test --workspace -p llm-gateway-api auto_route::tests 2>&1 | tail -20`

Expected: 10 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/auto_route.rs crates/api/src/lib.rs
git commit -m "feat(api): detect_required_capabilities for model=auto routing"
```

---

## Task 9: Reject model name `auto`

**Files:**
- Modify: `crates/api/src/management/models.rs` — `create_model_global` (line ~31) and `update_model` (line ~71).

- [ ] **Step 1: Write the failing integration test first (TDD)**

Add a new test file `crates/api/tests/test_model_name_reserved.rs`. Mirror the setup pattern from `crates/api/tests/test_management_keys.rs` (or `crates/api/tests/common/mod.rs` for shared helpers):

```rust
// Pattern (adapt to whatever the existing test helpers provide):
//   - Use the shared test-harness helper (likely in tests/common/mod.rs)
//     to spin up AppState + router with a seeded platform-admin user.
//   - Build an axum::Router::Oneshot request.
//   - Use the platform-admin's auth token in the Authorization header.
```

- [ ] **Step 2: Run the test — should fail**

Run: `cargo test --workspace -p llm-gateway-api --test test_model_name_reserved 2>&1 | tail -20`

Expected: FAIL — currently creating a model named `auto` returns 200.

- [ ] **Step 3: Add the guard to `create_model_global`**

In `crates/api/src/management/models.rs`, at the top of `create_model_global` (before the ownership logic):

```rust
pub async fn create_model_global(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Json(input): Json<CreateModelRequest>,
) -> Result<Json<Model>, ApiError> {
    if input.name.eq_ignore_ascii_case("auto") {
        return Err(ApiError::ModelNameReserved);
    }
    // ... existing body ...
```

- [ ] **Step 4: Add the same guard to `update_model`**

In `update_model`, after fetching the existing model, also reject if the incoming name (if any) is `auto`. Since `UpdateModel` doesn't currently include `name`, the guard isn't strictly needed — but we add it defensively in case the struct later gains a `name` field. For V1, the create-time check is sufficient; skip this step if `UpdateModel` has no `name` field.

- [ ] **Step 5: Run the test — should pass**

Run: `cargo test --workspace -p llm-gateway-api --test test_model_name_reserved 2>&1 | tail -20`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/management/models.rs crates/api/tests/test_model_name_reserved.rs
git commit -m "feat(api): reject model name 'auto' as reserved"
```

---

## Task 10: `auto_route_configs` CRUD endpoints

**Files:**
- Create: `crates/api/src/management/auto_routes.rs`
- Modify: `crates/api/src/management/mod.rs` — register module + routes

- [ ] **Step 1: Create the handler module**

`crates/api/src/management/auto_routes.rs` — mirror `model_fallbacks.rs` exactly. Substitute `AutoRouteConfig` types and call the new storage methods:

```rust
use axum::extract::{Path, State};
use axum::Json;
use std::sync::Arc;

use llm_gateway_org::OrgContext;
use llm_gateway_storage::{AutoRouteConfig, AutoRouteConfigData, CreateAutoRouteConfig, UpdateAutoRouteConfig};

use crate::error::ApiError;
use crate::AppState;

pub async fn create_auto_route_config(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Json(input): Json<CreateAutoRouteConfig>,
) -> Result<Json<AutoRouteConfig>, ApiError> {
    if !ctx.is_platform_admin() {
        return Err(ApiError::Forbidden);
    }

    let config = AutoRouteConfig {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name,
        config: input.config,
        created_by: Some(ctx.user_id.clone()),
        created_at: chrono::Utc::now(),
    };

    let created = state
        .storage
        .create_auto_route_config(&config)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(created))
}

pub async fn list_auto_route_configs(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
) -> Result<Json<Vec<AutoRouteConfig>>, ApiError> {
    let all = state
        .storage
        .list_auto_route_configs()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let filtered = if ctx.is_platform_admin() {
        all
    } else {
        all.into_iter()
            .filter(|c| c.created_by.as_deref() == Some(ctx.user_id.as_str()))
            .collect()
    };

    Ok(Json(filtered))
}

pub async fn get_auto_route_config(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path((_org_slug, id)): Path<(String, String)>,
) -> Result<Json<AutoRouteConfig>, ApiError> {
    let config = state
        .storage
        .get_auto_route_config(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Auto-route config '{}' not found", id)))?;

    if !ctx.is_platform_admin() && config.created_by.as_deref() != Some(ctx.user_id.as_str()) {
        return Err(ApiError::NotFound(format!("Auto-route config '{}' not found", id)));
    }

    Ok(Json(config))
}

pub async fn update_auto_route_config(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path((_org_slug, id)): Path<(String, String)>,
    Json(input): Json<UpdateAutoRouteConfig>,
) -> Result<Json<AutoRouteConfig>, ApiError> {
    let mut config = state
        .storage
        .get_auto_route_config(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Auto-route config '{}' not found", id)))?;

    if !ctx.is_platform_admin() && config.created_by.as_deref() != Some(ctx.user_id.as_str()) {
        return Err(ApiError::NotFound(format!("Auto-route config '{}' not found", id)));
    }

    if let Some(name) = input.name {
        config.name = name;
    }
    if let Some(new_config) = input.config {
        config.config = new_config;
    }

    let updated = state
        .storage
        .update_auto_route_config(&config)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(updated))
}

pub async fn delete_auto_route_config(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path((_org_slug, id)): Path<(String, String)>,
) -> Result<axum::http::StatusCode, ApiError> {
    let config = state
        .storage
        .get_auto_route_config(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Auto-route config '{}' not found", id)))?;

    if !ctx.is_platform_admin() && config.created_by.as_deref() != Some(ctx.user_id.as_str()) {
        return Err(ApiError::NotFound(format!("Auto-route config '{}' not found", id)));
    }

    state
        .storage
        .delete_auto_route_config(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
```

- [ ] **Step 2: Register the module**

In `crates/api/src/management/mod.rs`, add `pub mod auto_routes;` next to `pub mod model_fallbacks;` (around line 7).

- [ ] **Step 3: Register the routes**

In the same file, find the `// Model Fallbacks (authenticated)` block (around line 206-214). After it, add a matching `// Auto Route Configs` block:

```rust
        // Auto Route Configs (authenticated)
        .route(
            "/auto-route-configs",
            post(auto_routes::create_auto_route_config).get(auto_routes::list_auto_route_configs),
        )
        .route(
            "/auto-route-configs/{id}",
            get(auto_routes::get_auto_route_config)
                .patch(auto_routes::update_auto_route_config)
                .delete(auto_routes::delete_auto_route_config),
        )
```

- [ ] **Step 4: Build the api crate**

Run: `cargo build -p llm-gateway-api 2>&1 | tail -20`

Expected: clean compile.

- [ ] **Step 5: Add management integration test**

Create `crates/api/tests/test_auto_route_configs_crud.rs`. Mirror the setup pattern from `crates/api/tests/test_management_keys.rs` (and use any helpers from `crates/api/tests/common/mod.rs`). Cases:

- `platform_admin_can_crud_auto_route_config` — full POST/GET/PATCH/DELETE lifecycle
- `non_admin_create_returns_403` — regular member POST gets 403
- `non_admin_get_returns_404_for_others_configs` — non-creator non-admin GET returns 404

- [ ] **Step 6: Run the tests**

Run: `cargo test --workspace -p llm-gateway-api --test test_auto_route_configs_crud 2>&1 | tail -20`

Expected: all PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/api/src/management/auto_routes.rs \
        crates/api/src/management/mod.rs \
        crates/api/tests/test_auto_route_configs_crud.rs
git commit -m "feat(api): auto_route_configs CRUD endpoints"
```

---

## Task 11: `api_keys.auto_route_id` field plumbing

**Files:**
- Modify: `crates/api/src/management/keys.rs` — DTOs (`CreateKeyRequest`, `UpdateKeyRequest`, `CreateKeyResponse`, `KeyResponse` + their `From<ApiKey>` impls)
- Modify: `crates/storage/src/postgres.rs` — update `PgKeyRow`, INSERT, UPDATE, SELECT for `api_keys` to include `auto_route_id`

- [ ] **Step 1: Update `PgKeyRow` and impls**

In `crates/storage/src/postgres.rs`, find `PgKeyRow` (around line 42) and the matching `PgKeyWithMtdRow`. Add `auto_route_id: Option<String>` to each. Update the corresponding `From<...> for ApiKey` (and `ApiKeyWithMtd`) impls to copy the field through.

- [ ] **Step 2: Update INSERT in `create_key`**

Find `async fn create_key` and add `auto_route_id` to the INSERT column list and `.bind(...)`:

```rust
.bind(key.auto_route_id.as_deref())
```

Match the existing bind order. The `RETURNING` clause must also include `auto_route_id`.

- [ ] **Step 3: Update UPDATE in `update_key`**

Same pattern — add to SET clause and bind.

- [ ] **Step 4: Update every `api_keys` SELECT**

Grep for `SELECT` queries against `api_keys`. Add `auto_route_id` to the column list (or rely on `k.*` if the existing queries use it — sqlx maps by name).

- [ ] **Step 5: Extend the API key DTOs**

In `crates/api/src/management/keys.rs`, the existing DTOs are at lines 19-65:
- `CreateKeyRequest` (line 19) — add `pub auto_route_id: Option<String>`
- `UpdateKeyRequest` (line 27) — add `pub auto_route_id: Option<Option<String>>` (None=keep, Some(None)=clear)
- `CreateKeyResponse` (line 38) — does NOT need the field (it's the create-ack; the id is on the key but not surfaced on creation)
- `KeyResponse` (line 49) — add `pub auto_route_id: Option<String>` and populate it in both `From<ApiKey>` (line 67) and `From<ApiKeyWithMtd>` (line 85) impls.

- [ ] **Step 6: Plumb through to storage**

In the create/update handlers, copy the DTO field onto the `ApiKey` struct before calling storage.

- [ ] **Step 7: Build and run existing api_keys tests**

Run: `cargo build -p llm-gateway-api 2>&1 | tail -20 && cargo test --workspace -p llm-gateway-api --test test_management_keys 2>&1 | tail -20`

Existing tests must still pass — the new field is optional.

- [ ] **Step 8: Commit**

```bash
git add crates/storage/src/postgres.rs crates/api/src/management/keys.rs
git commit -m "feat(api,storage): auto_route_id on api_keys"
```

---

## Task 12: `ChannelRegistry::resolve_by_pool`

**Files:**
- Modify: `crates/api/src/proxy.rs` (or wherever the `ChannelRegistry` trait lives — currently around line 85).

- [ ] **Step 1: Add the trait method**

Find `#[async_trait::async_trait] pub trait ChannelRegistry` (around line 85). Add:

```rust
/// Resolve channels across multiple model names, returning `(model_name, ResolvedChannel)` pairs.
/// Each pair indicates "this channel can serve this model". A channel may
/// appear multiple times if it serves multiple pool models — that's correct;
/// the caller deduplicates per attempt by `(model_name, channel_id)`.
async fn resolve_by_pool(&self, model_names: &[String]) -> Vec<(String, ResolvedChannel)>;
```

- [ ] **Step 2: Implement on `InMemoryChannelRegistry`**

In the `impl ChannelRegistry for InMemoryChannelRegistry` block (around line 346), add:

```rust
async fn resolve_by_pool(&self, model_names: &[String]) -> Vec<(String, ResolvedChannel)> {
    let mut out = Vec::new();
    for name in model_names {
        for rc in self.resolve_by_model(name).await {
            out.push((name.clone(), rc));
        }
    }
    out
}
```

- [ ] **Step 3: Update any test stub implementations**

Search for `impl ChannelRegistry for` in the codebase (test mocks). The trait addition will require implementing the new method on each. Add a stub:

```rust
async fn resolve_by_pool(&self, _model_names: &[String]) -> Vec<(String, ResolvedChannel)> {
    Vec::new()
}
```

- [ ] **Step 4: Build**

Run: `cargo build -p llm-gateway-api 2>&1 | tail -20`

Expected: clean compile.

- [ ] **Step 5: Skip commit — proxy integration in Task 13 will commit together**

---

## Task 13: `proxy.rs` — `model == "auto"` branch

**Files:**
- Modify: `crates/api/src/proxy.rs` — `proxy_route_and_forward` (around line 1068).

This is the most invasive task. The current flow assumes a single `model_entry` resolved up-front; the auto path needs to defer that and instead assemble a multi-model candidate pool.

- [ ] **Step 1: Read the existing proxy flow thoroughly**

Read `crates/api/src/proxy.rs` lines 1068-1400. Note where `model_entry` is resolved (around line 1140), where `resolved_channels` is fetched (line 1146), and where candidates are assembled into `routing_candidates` (lines 1148-1212 cache-hit path, 1213-1300 DB-fallback path).

- [ ] **Step 2: Add the auto-routing branch**

In `proxy_route_and_forward`, after `client_requested_model` is captured (around line 1033-1037) and BEFORE the existing model lookup, add:

```rust
let is_auto = client_requested_model.eq_ignore_ascii_case("auto");

// Auto-routing setup. If is_auto, resolve the candidate model pool now;
// otherwise, leave the existing single-model flow intact.
let auto_pool: Vec<llm_gateway_storage::Model> = if is_auto {
    let config_id = api_key.auto_route_id.as_deref()
        .ok_or(ApiError::AutoNotConfigured)?;
    let config = state.storage.get_auto_route_config(config_id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::AutoNotConfigured)?;
    let required = crate::auto_route::detect_required_capabilities(&body);
    state.storage.list_models_with_capabilities(
        &api_key.org_id,
        required.vision,
        required.tools,
        &config.config.model_names,
    ).await
        .map_err(|e| ApiError::Internal(e.to_string()))?
} else {
    Vec::new()
};

if is_auto && auto_pool.is_empty() {
    let required = crate::auto_route::detect_required_capabilities(&body);
    return Err(ApiError::AutoNoMatchingModel {
        required_vision: required.vision,
        required_tools: required.tools,
    });
}
```

- [ ] **Step 3: Branch the candidate-gathering step**

The existing single-model flow at lines 1146-1212 builds `routing_candidates` by:
1. Calling `state.registry.resolve_by_model(&model_name)`
2. For each `ResolvedChannel`, looking up `model_overrides[model_key]` and building a `(ResolvedChannel, ChannelModel)` tuple

Wrap this in an `if is_auto { ... } else { <existing code> }`. In the auto branch, iterate over the pool and call `resolve_by_pool`:

```rust
let mut routing_candidates: Vec<(ResolvedChannel, llm_gateway_storage::ChannelModel)> = if is_auto {
    let pool_names: Vec<String> = auto_pool.iter().map(|m| m.name.clone()).collect();
    let pool_index: std::collections::HashMap<String, &llm_gateway_storage::Model> =
        auto_pool.iter().map(|m| (m.name.to_lowercase(), m)).collect();

    let resolved_pairs = state.registry.resolve_by_pool(&pool_names).await;
    let mut candidates = Vec::new();
    for (model_name_lc, rc) in resolved_pairs {
        let model_key = model_name_lc.to_lowercase();
        if let Some(enriched) = rc.model_overrides.get(&model_key) {
            let model_entry = match pool_index.get(&model_key) {
                Some(m) => m,
                None => continue,
            };
            let cm = llm_gateway_storage::ChannelModel {
                id: Uuid::new_v4().to_string(),
                org_id: api_key.org_id.clone(),
                channel_id: rc.channel_id.to_string(),
                model_id: model_entry.id.clone(),
                enabled: true,
                upstream_model_name: enriched.upstream_model_name.clone(),
                pricing_policy_id: enriched.pricing_policy_id.clone(),
                markup_ratio: enriched.markup_ratio,
                priority_override: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            candidates.push((rc, cm));
        }
    }
    if candidates.is_empty() {
        return Err(ApiError::AutoNoMatchingModel {
            required_vision: false,  // already have pool — this is "no channels", not "no models"
            required_tools: false,
        });
    }
    // Apply user-group filter (same as single-model path).
    if let Some(ref user_id) = request_user_id {
        if !request_is_admin {
            match state.storage.get_user_group_id(user_id, &api_key.org_id).await {
                Ok(Some(allowed_group_id)) => {
                    candidates.retain(|(rc, _)| {
                        rc.group_id.is_none() || rc.group_id.as_deref() == Some(&allowed_group_id)
                    });
                }
                _ => {}
            }
        }
    }
    candidates
} else {
    // ... existing single-model flow ...
};
```

- [ ] **Step 4: Update the per-candidate model_name tracking**

The failover loop logs `routes.push(RouteAttempt { model: model_name.clone(), ... })`. In the auto path, the model name varies per candidate. Track it alongside each candidate by changing `routing_candidates` to `Vec<(String, ResolvedChannel, ChannelModel)>` (adding the model name as the first element), OR by re-deriving from `cm.model_id` → model lookup. The first option is cleaner.

Update the failover loop to use `candidate_model_name` (the new tuple field) when constructing `RouteAttempt`.

- [ ] **Step 5: Update terminal failure in auto mode**

When all candidates are exhausted, the existing code falls through to `try_model_fallback` then `ApiError::NotFound`. In auto mode, `try_model_fallback` must NOT be called (auto and fallback are independent per the spec). And the terminal error becomes `AutoAllCandidatesFailed`:

```rust
if is_auto {
    return Err(ApiError::AutoAllCandidatesFailed);
}
// ... existing try_model_fallback + NotFound path ...
```

- [ ] **Step 6: Audit-log compatibility**

The existing code records `client_requested_model` (= "auto") in the audit log. The spec accepts this (line 39): "Audit-log changes beyond what the existing `client_requested_model` + `routes[].model` columns already capture." No additional audit work needed.

- [ ] **Step 7: Build and run existing proxy tests**

Run: `cargo build -p llm-gateway-api 2>&1 | tail -20 && cargo test --workspace -p llm-gateway-api 2>&1 | tail -30`

Expected: clean build, all existing tests still pass. The auto path is unreachable in existing tests because none send `model=auto`.

- [ ] **Step 8: Commit**

```bash
git add crates/api/src/proxy.rs
git commit -m "feat(api): model=auto routing path in proxy"
```

---

## Task 14: Proxy integration tests for `model=auto`

**Files:**
- Create: `crates/api/tests/test_auto_route.rs`

- [ ] **Step 1: Write the failing integration tests**

Create `crates/api/tests/test_auto_route.rs`. The proxy end-to-end test harness already exists in this codebase — `crates/api/tests/phase5_enforcement.rs`, `phase6_enforcement.rs`, and `phase7_budget_status.rs` all drive requests through the full gateway (auth → routing → upstream → response). Read `phase5_enforcement.rs` first to learn how it:

- Builds AppState with a real `PostgresStorage` (via `#[sqlx::test]`)
- Seeds providers, channels, channel_models, models, api_keys
- Mocks the upstream HTTP endpoint (look for `wiremock`, `httpmock`, or a custom mock — use whatever phase5 uses)
- Invokes the router via `oneshot` against `/v1/chat/completions` or `/v1/messages`

Use the EXACT same harness — don't invent a new one.

Test cases (write each as a separate `#[sqlx::test]` function):

```rust
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn auto_without_config_returns_400(pool: sqlx::PgPool) {
    // Key has auto_route_id = NULL.
    // Send POST /v1/chat/completions with model=auto.
    // Assert 400 + body code "auto_not_configured".
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn auto_with_vision_routes_to_vision_capable_model(pool: sqlx::PgPool) {
    // Pool has 2 models: text-only + vision-capable. Both have channels.
    // Body contains an image_url block.
    // Upstream mock records the model name it received.
    // Assert only the vision-capable model was hit.
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn auto_with_tools_routes_to_tools_capable_model(pool: sqlx::PgPool) { /* analogous */ }

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn auto_with_no_capabilities_routes_to_any_pool_model(pool: sqlx::PgPool) {
    // Pool has 2 models, body is text-only. Either is acceptable.
    // Assert upstream was hit exactly once (no double-routing) and the
    // chosen model is in the pool.
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn auto_with_unsatisfiable_capabilities_returns_400(pool: sqlx::PgPool) {
    // Pool has only text-only model. Body requires vision.
    // Assert 400 + code "auto_no_matching_model" + required_capabilities=["vision"].
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn auto_failover_across_models_when_first_model_channels_fail(pool: sqlx::PgPool) {
    // Pool has model_A (channel returns 500) + model_B (channel returns 200).
    // Assert: model_B was eventually hit; response status 200; routes[] has 2 entries.
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn auto_all_candidates_failed_returns_502(pool: sqlx::PgPool) {
    // Pool has 2 models, both channels return 500.
    // Assert 502 + code "auto_all_candidates_failed".
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn auto_ignores_models_outside_pool_even_if_capable(pool: sqlx::PgPool) {
    // Org has 3 vision-capable models; pool includes only 1.
    // Assert only the pooled model is ever hit.
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test --workspace -p llm-gateway-api --test test_auto_route 2>&1 | tail -40`

Expected: all PASS. If any fail, debug systematically (Phase 1 of systematic-debugging skill): read the error, reproduce consistently, identify root cause before fixing.

- [ ] **Step 3: Commit**

```bash
git add crates/api/tests/test_auto_route.rs
git commit -m "test(api): integration tests for model=auto routing"
```

---

## Task 15: Frontend types

**Files:**
- Modify: `web/src/types/index.ts`

- [ ] **Step 1: Extend `Model`**

```typescript
export interface Model {
  id: string;
  owner_org_id: string | null;
  name: string;
  model_type?: string | null;
  pricing_policy_id?: string | null;
  supports_vision: boolean;
  supports_tools: boolean;
  created_at: string;
}
```

- [ ] **Step 2: Extend `CreateKeyRequest` + `UpdateKeyRequest`**

```typescript
export interface CreateKeyRequest {
  name: string;
  rate_limit?: number | null;
  budget_monthly?: number | null;
  model_fallback_id?: string | null;
  auto_route_id?: string | null;
}

export interface UpdateKeyRequest {
  name?: string;
  rate_limit?: number | null;
  budget_monthly?: number | null;
  enabled?: boolean;
  model_fallback_id?: string | null;
  auto_route_id?: string | null;
}
```

- [ ] **Step 3: Extend `ApiKey`**

```typescript
export interface ApiKey {
  // ...existing...
  model_fallback_id: string | null;
  auto_route_id: string | null;
  // ...
}
```

- [ ] **Step 4: Add `AutoRouteConfig*` types**

Near the existing `ModelFallbackConfig` block (around line 507):

```typescript
// ── Auto Route Config Types ───────────────────────────────────────────────

export interface AutoRouteConfigData {
  model_names: string[];
}

export interface AutoRouteConfig {
  id: string;
  name: string;
  config: AutoRouteConfigData;
  created_by: string | null;
  created_at: string;
}

export interface CreateAutoRouteConfigRequest {
  name: string;
  config: AutoRouteConfigData;
}

export interface UpdateAutoRouteConfigRequest {
  name?: string;
  config?: AutoRouteConfigData;
}
```

- [ ] **Step 5: Build**

Run: `source ~/.nvm/nvm.sh && cd web && npm run build 2>&1 | tail -20`

Expected: TypeScript errors at sites that index `model.supports_vision` etc., but no errors in `types/index.ts` itself. Continue — fixes come in Tasks 17-19.

- [ ] **Step 6: Skip commit — Tasks 16-19 will batch with their respective UI work**

---

## Task 16: Frontend API client + hooks

**Files:**
- Create: `web/src/api/autoRoutes.ts`
- Create: `web/src/hooks/useAutoRouteConfigs.ts`

- [ ] **Step 1: Create `web/src/api/autoRoutes.ts`**

Mirror `modelFallbacks.ts`:

```typescript
import { apiClient, orgPrefix } from './client';
import type {
  AutoRouteConfig,
  CreateAutoRouteConfigRequest,
  UpdateAutoRouteConfigRequest,
} from '../types';

export async function listAutoRouteConfigs(): Promise<AutoRouteConfig[]> {
  const { data } = await apiClient.get<AutoRouteConfig[]>(`${orgPrefix()}/auto-route-configs`);
  return data;
}

export async function getAutoRouteConfig(id: string): Promise<AutoRouteConfig> {
  const { data } = await apiClient.get<AutoRouteConfig>(`${orgPrefix()}/auto-route-configs/${id}`);
  return data;
}

export async function createAutoRouteConfig(input: CreateAutoRouteConfigRequest): Promise<AutoRouteConfig> {
  const { data } = await apiClient.post<AutoRouteConfig>(`${orgPrefix()}/auto-route-configs`, input);
  return data;
}

export async function updateAutoRouteConfig(id: string, input: UpdateAutoRouteConfigRequest): Promise<AutoRouteConfig> {
  const { data } = await apiClient.patch<AutoRouteConfig>(`${orgPrefix()}/auto-route-configs/${id}`, input);
  return data;
}

export async function deleteAutoRouteConfig(id: string): Promise<void> {
  await apiClient.delete(`${orgPrefix()}/auto-route-configs/${id}`);
}
```

- [ ] **Step 2: Create `web/src/hooks/useAutoRouteConfigs.ts`**

Mirror `useModelFallbacks.ts`:

```typescript
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  listAutoRouteConfigs,
  createAutoRouteConfig,
  updateAutoRouteConfig,
  deleteAutoRouteConfig,
} from '../api/autoRoutes';
import type { CreateAutoRouteConfigRequest, UpdateAutoRouteConfigRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';
import i18n from '../i18n';
import { useAuthStore } from '../stores/authStore';

export function useAutoRouteConfigs() {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({
    queryKey: [slug, 'auto-route-configs'],
    queryFn: listAutoRouteConfigs,
    enabled: !!slug,
  });
}

export function useCreateAutoRouteConfig() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: (input: CreateAutoRouteConfigRequest) => createAutoRouteConfig(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [slug, 'auto-route-configs'] });
      toast.success(i18n.t('toasts.autoRouteCreated'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.autoRouteCreateFailed'))); },
  });
}

export function useUpdateAutoRouteConfig() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateAutoRouteConfigRequest }) =>
      updateAutoRouteConfig(id, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [slug, 'auto-route-configs'] });
      toast.success(i18n.t('toasts.autoRouteUpdated'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.autoRouteUpdateFailed'))); },
  });
}

export function useDeleteAutoRouteConfig() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: (id: string) => deleteAutoRouteConfig(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [slug, 'auto-route-configs'] });
      toast.success(i18n.t('toasts.autoRouteDeleted'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.autoRouteDeleteFailed'))); },
  });
}
```

- [ ] **Step 3: Build (types may still be broken from Task 15 — that's OK)**

Run: `source ~/.nvm/nvm.sh && cd web && npm run build 2>&1 | tail -20`

Expected: same errors as Task 15 (UI hasn't consumed the new types yet).

- [ ] **Step 4: Skip commit — Task 17 batches the frontend commits**

---

## Task 17: AutoRoutes page + Layout + i18n

**Files:**
- Create: `web/src/pages/AutoRoutes.tsx`
- Modify: `web/src/components/Layout.tsx` — add sidebar entry
- Modify: `web/src/App.tsx` — register route
- Modify: `web/src/i18n/en.json` + `zh.json` — new strings

- [ ] **Step 1: Create `web/src/pages/AutoRoutes.tsx`**

Mirror `ModelFallbacks.tsx` layout (table + create/edit drawer). Fields: name, multi-select for `model_names` (use the org's models list — `useModels()` hook already exists).

```tsx
import { useState } from 'react';
import { Plus, Trash2, X } from 'lucide-react';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  useAutoRouteConfigs,
  useCreateAutoRouteConfig,
  useUpdateAutoRouteConfig,
  useDeleteAutoRouteConfig,
} from '../hooks/useAutoRouteConfigs';
import { useModels } from '../hooks/useModels';
import { useReducedMotion } from '../hooks/useReducedMotion';
import { Button } from '../components/ui/Button';
import { Modal } from '../components/ui/Modal';

const EASE = [0.16, 1, 0.3, 1] as const;

export default function AutoRoutes() {
  const { t } = useTranslation();
  const { data: configs, isLoading } = useAutoRouteConfigs();
  const { data: models } = useModels();
  const createMutation = useCreateAutoRouteConfig();
  const updateMutation = useUpdateAutoRouteConfig();
  const deleteMutation = useDeleteAutoRouteConfig();
  const reducedMotion = useReducedMotion();

  const [createOpen, setCreateOpen] = useState(false);
  const [editId, setEditId] = useState<string | null>(null);
  const [name, setName] = useState('');
  const [selectedModels, setSelectedModels] = useState<string[]>([]);
  const [deleteId, setDeleteId] = useState<string | null>(null);
  const [validationError, setValidationError] = useState('');

  const resetForm = () => {
    setName('');
    setSelectedModels([]);
    setEditId(null);
    setValidationError('');
  };
  const openCreate = () => { resetForm(); setCreateOpen(true); };
  const openEdit = (id: string) => {
    const c = configs?.find((c) => c.id === id);
    if (!c) return;
    setEditId(id);
    setName(c.name);
    setSelectedModels(c.config.model_names);
    setValidationError('');
    setCreateOpen(true);
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    setValidationError('');
    if (!name.trim()) {
      setValidationError(t('autoRoutes.editModal.nameRequired'));
      return;
    }
    if (selectedModels.length === 0) {
      setValidationError(t('autoRoutes.editModal.atLeastOneModel'));
      return;
    }
    if (editId) {
      await updateMutation.mutateAsync({
        id: editId,
        input: { name, config: { model_names: selectedModels } },
      });
    } else {
      await createMutation.mutateAsync({
        name,
        config: { model_names: selectedModels },
      });
    }
    setCreateOpen(false);
    resetForm();
  };

  const toggleModel = (modelName: string) => {
    setSelectedModels((prev) =>
      prev.includes(modelName) ? prev.filter((m) => m !== modelName) : [...prev, modelName],
    );
  };

  return (
    <div className="p-6">
      {/* header + create button + table + drawer + delete-confirm modal */}
      {/* Mirror ModelFallbacks.tsx layout. The table columns: Name, Models (comma-joined), Created By, Actions (Edit/Delete). */}
      {/* The create/edit drawer has: name input, multi-select (checkboxes from models list), Save/Cancel. */}
    </div>
  );
}
```

(The full JSX is ~200 lines — mirror `ModelFallbacks.tsx` for the exact structure: header with create button, table with rows, drawer/modal for create-edit, delete confirmation modal.)

- [ ] **Step 2: Add the sidebar entry**

In `web/src/components/Layout.tsx`, find the `adminItems` array (around line 74-85). Add a new entry after the model-fallbacks-related items (or wherever fits the IA — likely near Models):

```tsx
{ key: `/${slug}/admin/auto-routes`, icon: <SomeIcon>, label: t('sidebar.autoRoutes') },
```

Pick an icon from `lucide-react` — `Compass` or `Route` or `Shuffle` fit "auto routing". Import it at the top.

- [ ] **Step 3: Register the route in App.tsx**

In `web/src/App.tsx`, inside the `RequireAdmin` block (where other admin pages are registered), add:

```tsx
<Route path="admin/auto-routes" element={<AutoRoutes />} />
```

And import `AutoRoutes` at the top.

- [ ] **Step 4: Add i18n keys**

In `web/src/i18n/en.json`:

```json
"sidebar": {
  ...,
  "autoRoutes": "Auto Routes"
},
"autoRoutes": {
  "title": "Auto Routes",
  "subtitle": "Per-key capability-aware model pools for model=auto",
  "createBtn": "New Auto Route",
  "table": {
    "name": "Name",
    "models": "Models",
    "createdBy": "Created By",
    "actions": "Actions"
  },
  "editModal": {
    "createTitle": "Create Auto Route",
    "editTitle": "Edit Auto Route",
    "name": "Name",
    "nameRequired": "Name is required",
    "models": "Models in pool",
    "atLeastOneModel": "Select at least one model",
    "save": "Save",
    "cancel": "Cancel"
  },
  "deleteConfirm": {
    "title": "Delete auto route?",
    "message": "This will remove the config. Keys referencing it will return auto_not_configured until reassigned.",
    "confirm": "Delete"
  },
  "empty": "No auto routes configured."
},
"toasts": {
  ...,
  "autoRouteCreated": "Auto route created",
  "autoRouteCreateFailed": "Failed to create auto route",
  "autoRouteUpdated": "Auto route updated",
  "autoRouteUpdateFailed": "Failed to update auto route",
  "autoRouteDeleted": "Auto route deleted",
  "autoRouteDeleteFailed": "Failed to delete auto route"
}
```

Mirror in `web/src/i18n/zh.json` with Chinese translations:
- `"autoRoutes": "自动路由"`
- `"title": "自动路由配置"`
- etc.

- [ ] **Step 5: Build**

Run: `source ~/.nvm/nvm.sh && cd web && npm run build 2>&1 | tail -20`

Expected: clean compile.

- [ ] **Step 6: Commit**

```bash
git add web/src/pages/AutoRoutes.tsx \
        web/src/components/Layout.tsx \
        web/src/App.tsx \
        web/src/i18n/en.json \
        web/src/i18n/zh.json \
        web/src/types/index.ts \
        web/src/api/autoRoutes.ts \
        web/src/hooks/useAutoRouteConfigs.ts
git commit -m "feat(web): Auto Routes admin page + sidebar entry"
```

---

## Task 18: Models page capability checkboxes

**Files:**
- Modify: `web/src/pages/Models.tsx`

- [ ] **Step 1: Read the current Models page**

Identify the row-rendering code and the existing edit flow (likely an inline form or modal). The two new checkboxes should appear in the same edit UI.

- [ ] **Step 2: Add capability checkboxes to the edit form**

In the edit modal/row, add:

```tsx
<label className="label cursor-pointer justify-start gap-3">
  <input
    type="checkbox"
    className="checkbox checkbox-sm"
    checked={supportsVision}
    onChange={(e) => setSupportsVision(e.target.checked)}
  />
  <span className="label-text">{t('models.supportsVision')}</span>
</label>
<label className="label cursor-pointer justify-start gap-3">
  <input
    type="checkbox"
    className="checkbox checkbox-sm"
    checked={supportsTools}
    onChange={(e) => setSupportsTools(e.target.checked)}
  />
  <span className="label-text">{t('models.supportsTools')}</span>
</label>
```

Wire `supportsVision` / `supportsTools` into the edit-form state, initialized from the model being edited, and include them in the PATCH request body.

- [ ] **Step 3: Display the flags as read-only badges in the row**

Add two small badges next to the model name (or in a "Capabilities" column):

```tsx
{model.supports_vision && <span className="badge badge-xs badge-ghost">Vision</span>}
{model.supports_tools && <span className="badge badge-xs badge-ghost">Tools</span>}
```

- [ ] **Step 4: Add i18n keys**

In `en.json`: `"models": { ..., "supportsVision": "Vision", "supportsTools": "Tools" }`.
In `zh.json`: `"supportsVision": "视觉"`, `"supportsTools": "工具调用"`.

- [ ] **Step 5: Build**

Run: `source ~/.nvm/nvm.sh && cd web && npm run build 2>&1 | tail -20`

- [ ] **Step 6: Commit**

```bash
git add web/src/pages/Models.tsx web/src/i18n/en.json web/src/i18n/zh.json
git commit -m "feat(web): capability checkboxes on Models page"
```

---

## Task 19: Keys page auto-route selector

**Files:**
- Modify: `web/src/pages/Keys.tsx`

- [ ] **Step 1: Add the selector to the create + edit forms**

Mirror the existing `modelFallbackId` select (around lines 272-276 of Keys.tsx). Below it, add:

```tsx
<div className="form-control">
  <label className="label">
    <span className="label-text font-medium">{t('keys.form.autoRouteConfig')}</span>
  </label>
  <select
    value={autoRouteId}
    onChange={(e) => setAutoRouteId(e.target.value)}
    className="select select-bordered w-full"
  >
    <option value="">{t('keys.form.noneOption')}</option>
    {autoRouteConfigs?.map((c) => (
      <option key={c.id} value={c.id}>{c.name}</option>
    ))}
  </select>
</div>
```

- [ ] **Step 2: Wire state + mutation**

Add at the top of the component:

```tsx
const { data: autoRouteConfigs } = useAutoRouteConfigs();
const [autoRouteId, setAutoRouteId] = useState<string>('');
```

Initialize `autoRouteId` from the existing key when editing. Include `auto_route_id: autoRouteId || null` in the create + update mutation payloads.

- [ ] **Step 3: Add i18n keys**

`en.json`: `"keys": { ..., "form": { ..., "autoRouteConfig": "Auto Route Config" } }`.
`zh.json`: `"autoRouteConfig": "自动路由配置"`.

- [ ] **Step 4: Build**

Run: `source ~/.nvm/nvm.sh && cd web && npm run build 2>&1 | tail -20`

- [ ] **Step 5: Commit**

```bash
git add web/src/pages/Keys.tsx web/src/i18n/en.json web/src/i18n/zh.json
git commit -m "feat(web): auto-route selector on Keys page"
```

---

## Task 20: Frontend tests

**Files:**
- Create: `web/src/pages/AutoRoutes.test.tsx`
- Modify: `web/src/pages/Models.test.tsx` if it exists, else create
- Modify: `web/src/pages/Keys.test.tsx` if it exists, else create

- [ ] **Step 1: Write AutoRoutes page tests**

Mirror existing page tests (look for `web/src/pages/*.test.tsx` files). Cases:

```typescript
describe('AutoRoutes page', () => {
  it('renders empty state when no configs');
  it('opens create modal on button click');
  it('validates name is required');
  it('validates at least one model is selected');
  it('submits create mutation with correct payload');
  it('opens edit modal pre-populated');
  it('submits update mutation');
  it('confirms before delete');
});
```

- [ ] **Step 2: Write Models page test for capability checkboxes**

```typescript
describe('Models page capability checkboxes', () => {
  it('displays capability badges on rows');
  it('checkboxes toggle and persist on save');
});
```

- [ ] **Step 3: Write Keys page test for auto-route selector**

```typescript
describe('Keys page auto-route selector', () => {
  it('lists auto-route configs in the select');
  it('submits auto_route_id on create');
  it('submits auto_route_id on update');
});
```

- [ ] **Step 4: Run all frontend tests**

Run: `source ~/.nvm/nvm.sh && cd web && npm test -- --run 2>&1 | tail -40`

Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add web/src/pages/AutoRoutes.test.tsx web/src/pages/Models.test.tsx web/src/pages/Keys.test.tsx
git commit -m "test(web): auto-route UI tests"
```

---

## Task 21: CHANGELOG entry

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Add entry under `### Added`**

Find the topmost `## [Unreleased]` section (or create one). Under `### Added`:

```markdown
### Added

- **`model=auto` capability-aware routing**: clients can now send `model=auto` and the gateway picks a model from a per-key admin-defined pool based on the request's required capabilities (vision, tools).
- New `auto_route_configs` platform-level table + `api_keys.auto_route_id` FK for binding configs to keys.
- New `supports_vision` / `supports_tools` columns on `models` (admin-managed via the Models page).
- New management endpoints `/api/v1/{slug}/auto-route-configs` (CRUD, platform-admin-gated).
- Reserved the model name `auto` — creation is rejected with `model_name_reserved`.
```

- [ ] **Step 2: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs: changelog entry for model=auto routing"
```

---

## Verification (final)

After all tasks complete:

1. **Backend workspace check:**
   ```bash
   cargo test --workspace 2>&1 | tail -30
   ```
   All tests pass, including the new `test_auto_route.rs`, `test_auto_route_configs_crud.rs`, `test_model_name_reserved.rs`, and `auto_route_query.rs`.

2. **Frontend check:**
   ```bash
   source ~/.nvm/nvm.sh && cd web && npm run build && npm test -- --run
   ```
   TypeScript clean, all tests pass.

3. **Manual smoke test (API on :8080, web on :5173):**
   - As platform-admin: create a model with `supports_vision=true`, create an `auto_route_config` with that model in its pool, create an API key with `auto_route_id` pointing at the config.
   - Send a chat completion request with `model=auto` and an image block in the body → response 200, audit log shows the chosen model name and `client_requested_model=auto`.
   - Send `model=auto` with text-only body → routes to any model in the pool.
   - Unset the key's `auto_route_id`, send `model=auto` → 400 `auto_not_configured`.
   - Send `model=auto` with a tools array but pool only has non-tools models → 400 `auto_no_matching_model` with `required_capabilities=["tools"]`.
   - Try to create a model named `auto` → 400 `model_name_reserved`.

4. **Run the finishing-a-development-branch skill** to merge / PR.
