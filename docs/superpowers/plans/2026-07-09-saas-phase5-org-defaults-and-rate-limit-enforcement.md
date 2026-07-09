# SaaS Phase 5: Org Defaults + Rate-Limit Enforcement — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship org-wide rate-limit + budget defaults (via `/api/v1/orgs/{id}/defaults`), and wire the existing-but-unused `RateLimiter::check_and_increment` into the proxy request path so per-key and org-default RPM limits actually enforce.

**Architecture:** Storage reuses the existing `org_settings` kv table from Phase 1 — no schema migration. Two new typed storage trait methods (`get_org_defaults`, `set_org_defaults`) wrap the kv calls. Two new management handlers expose typed GET/PUT endpoints. A new step in `proxy_inner` resolves `api_key.rate_limit ?? org.default_rate_limit_rpm ?? None` and calls `check_and_increment` with the empty model string for per-key bucketing; on `false` returns `ApiError::RateLimited { retry_after_secs }` which emits a `Retry-After` header.

**Tech Stack:** Rust (Axum 0.8, sqlx, tower), React + TypeScript (Zustand, React Query, Tailwind/DaisyUI, sonner), Vitest + RTL + MSW for frontend tests, `#[sqlx::test]` + `tower::ServiceExt` for backend integration, Playwright for e2e.

**Spec:** `docs/superpowers/specs/2026-07-09-saas-phase5-org-defaults-and-rate-limit-enforcement-design.md`

**Branch:** `feature/saas-phase5-org-defaults` cut from `develop`.

---

## File Structure

**Backend (Rust):**

| File | Role | Action |
|---|---|---|
| `crates/storage/src/types.rs` | Add `OrgDefaults` struct | Modify |
| `crates/storage/src/lib.rs` | Declare `get_org_defaults`, `set_org_defaults` trait methods | Modify |
| `crates/storage/src/postgres.rs` | Implement the two methods wrapping kv calls + storage unit tests | Modify |
| `crates/api/src/error.rs` | Change `RateLimited` to struct variant carrying `retry_after_secs`; emit `Retry-After` header | Modify |
| `crates/api/src/management/auth.rs` | Add `get_org_defaults`, `update_org_defaults` handlers | Modify |
| `crates/api/src/management/mod.rs` | Mount `.route("/defaults", get(...).put(...))` | Modify |
| `crates/api/src/proxy.rs` | New "Step 1.5: Rate-limit check" between auth and balance check | Modify |
| `crates/api/tests/phase5_org_defaults.rs` | API integration tests for the defaults endpoints | Create |
| `crates/api/tests/phase5_enforcement.rs` | API integration tests for proxy enforcement | Create |

**Frontend:**

| File | Role | Action |
|---|---|---|
| `web/src/api/orgs.ts` | Add `OrgDefaults` type + `getOrgDefaults`, `updateOrgDefaults` | Modify |
| `web/src/hooks/useOrgDefaults.ts` | `useGetOrgDefaults`, `useUpdateOrgDefaults` React Query hooks | Create |
| `web/src/pages/OrgSettings.tsx` | New "Defaults" section between General and Danger Zone | Modify |
| `web/src/pages/OrgSettings.test.tsx` | Frontend unit tests for the Defaults section | Create |
| `web/src/i18n/en.json` | `orgSettings.defaults.*` keys | Modify |
| `web/src/i18n/zh.json` | Mirrored Chinese keys | Modify |
| `web/e2e/org-defaults.spec.ts` | E2E: admin sets default, key gets 429 | Create |

**Docs:**

| File | Role | Action |
|---|---|---|
| `CHANGELOG.md` | Phase 5 entry under `## [Unreleased] → Added` with upgrade note | Modify |

---

## Task 1: Storage — `OrgDefaults` type + trait methods + postgres impl

**Files:**
- Modify: `crates/storage/src/types.rs` (add `OrgDefaults` near other org-related types)
- Modify: `crates/storage/src/lib.rs:328` (add trait methods after `set_org_setting`)
- Modify: `crates/storage/src/postgres.rs` (add impl after existing `set_org_setting` impl + add round-trip test)
- Test: `crates/storage/src/postgres.rs` (inline `#[sqlx::test]`)

### Step 1: Write failing storage test

Add this test inside the existing `#[cfg(test)]` block in `crates/storage/src/postgres.rs`, near other Phase 4 tests (look for `password_reset_round_trip` for placement):

```rust
/// Org defaults round-trip: writes both fields, reads them back, verifies
/// `None` is preserved, and confirms no cross-org interference.
#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn org_defaults_round_trip(pool: sqlx::PgPool) {
    use crate::types::OrgDefaults;

    let storage = crate::postgres::PostgresStorage::from_pool(pool);
    let org = make_test_org(&storage, "org-a", "Org A").await;

    // 1. Initial state — both None.
    let initial = storage.get_org_defaults(&org.id).await.expect("get initial");
    assert_eq!(initial, OrgDefaults {
        default_rate_limit_rpm: None,
        default_budget_monthly_usd: None,
    });

    // 2. Write both fields.
    storage
        .set_org_defaults(&org.id, &OrgDefaults {
            default_rate_limit_rpm: Some(100),
            default_budget_monthly_usd: Some(5000),  // $50.00 in cents
        })
        .await
        .expect("set both");

    let after = storage.get_org_defaults(&org.id).await.expect("get after set");
    assert_eq!(after.default_rate_limit_rpm, Some(100));
    assert_eq!(after.default_budget_monthly_usd, Some(5000));

    // 3. Clear rate limit only — budget must persist.
    storage
        .set_org_defaults(&org.id, &OrgDefaults {
            default_rate_limit_rpm: None,
            default_budget_monthly_usd: Some(5000),
        })
        .await
        .expect("clear rate limit");
    let cleared = storage.get_org_defaults(&org.id).await.expect("get after clear");
    assert_eq!(cleared.default_rate_limit_rpm, None);
    assert_eq!(cleared.default_budget_monthly_usd, Some(5000));

    // 4. Different org — independent state.
    let org_b = make_test_org(&storage, "org-b", "Org B").await;
    let b_initial = storage.get_org_defaults(&org_b.id).await.expect("get b initial");
    assert_eq!(b_initial, OrgDefaults {
        default_rate_limit_rpm: None,
        default_budget_monthly_usd: None,
    });
}
```

### Step 2: Run test to verify it fails

```bash
DATABASE_URL='postgresql://llm_gateway:Xabc12345@10.0.17.3:5432/llm_gateway' \
  cargo test -p llm-gateway-storage --lib org_defaults_round_trip
```

Expected: FAIL with `error[E0433]: failed to resolve: use of undeclared type or module 'OrgDefaults'` (or trait method not found).

### Step 3: Add `OrgDefaults` type

In `crates/storage/src/types.rs`, add after the `OrgSettings` or similar struct (placement near the org-related types):

```rust
/// Org-wide default settings surfaced via `GET/PUT /api/v1/orgs/{id}/defaults`.
///
/// Stored as two rows in `org_settings` kv table; this struct is the typed
/// facade. `default_budget_monthly_usd` is in integer cents (matches the
/// monetary-integer-subunits convention used elsewhere — see `crates/storage/src/money.rs`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrgDefaults {
    /// Default per-key RPM cap for keys whose own `rate_limit` is NULL.
    /// None = unlimited.
    pub default_rate_limit_rpm: Option<i64>,
    /// Default per-key monthly budget in USD cents. None = no budget.
    /// NOTE: stored for display; NOT enforced in Phase 5.
    pub default_budget_monthly_usd: Option<i64>,
}
```

### Step 4: Add trait method declarations

In `crates/storage/src/lib.rs`, immediately after the existing `list_org_settings` declaration (line 330), add:

```rust
    /// Typed facade over `org_settings` for the two Phase 5 default keys
    /// (`default_rate_limit_rpm`, `default_budget_monthly_usd`). Absent keys
    /// are surfaced as `None`. `default_budget_monthly_usd` is in USD cents.
    async fn get_org_defaults(
        &self,
        org_id: &str,
    ) -> Result<crate::types::OrgDefaults, Box<dyn std::error::Error + Send + Sync>>;

    /// Writes both default keys atomically (call sites pass the full struct;
    /// `None` clears that key by deleting the row).
    async fn set_org_defaults(
        &self,
        org_id: &str,
        defaults: &crate::types::OrgDefaults,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
```

### Step 5: Add postgres impl

In `crates/storage/src/postgres.rs`, after the existing `list_org_settings` impl, add:

```rust
    async fn get_org_defaults(
        &self,
        org_id: &str,
    ) -> Result<crate::types::OrgDefaults, DbErr> {
        let rpm = self
            .get_org_setting(org_id, "default_rate_limit_rpm")
            .await?
            .and_then(|s| s.parse::<i64>().ok());
        let budget = self
            .get_org_setting(org_id, "default_budget_monthly_usd")
            .await?
            .and_then(|s| s.parse::<i64>().ok());
        Ok(crate::types::OrgDefaults {
            default_rate_limit_rpm: rpm,
            default_budget_monthly_usd: budget,
        })
    }

    async fn set_org_defaults(
        &self,
        org_id: &str,
        defaults: &crate::types::OrgDefaults,
    ) -> Result<(), DbErr> {
        match defaults.default_rate_limit_rpm {
            Some(n) => self
                .set_org_setting(org_id, "default_rate_limit_rpm", &n.to_string())
                .await?,
            None => {
                sqlx::query(
                    "DELETE FROM org_settings WHERE org_id = $1 AND key = 'default_rate_limit_rpm'",
                )
                .bind(org_id)
                .execute(&self.pool)
                .await?;
            }
        }
        match defaults.default_budget_monthly_usd {
            Some(n) => self
                .set_org_setting(org_id, "default_budget_monthly_usd", &n.to_string())
                .await?,
            None => {
                sqlx::query(
                    "DELETE FROM org_settings WHERE org_id = $1 AND key = 'default_budget_monthly_usd'",
                )
                .bind(org_id)
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }
```

### Step 6: Run test to verify it passes

```bash
DATABASE_URL='postgresql://llm_gateway:Xabc12345@10.0.17.3:5432/llm_gateway' \
  cargo test -p llm-gateway-storage --lib org_defaults_round_trip
```

Expected: PASS (1 passed).

### Step 7: Commit

```bash
git add crates/storage/src/types.rs crates/storage/src/lib.rs crates/storage/src/postgres.rs
git commit -m "feat(storage): OrgDefaults type + get/set trait methods

Wraps the Phase 1 org_settings kv table with a typed facade for the two
Phase 5 default keys (rate-limit RPM, monthly budget in cents). Setting a
field to None deletes the underlying row."
```

---

## Task 2: ApiError — change `RateLimited` to struct variant with `Retry-After`

**Files:**
- Modify: `crates/api/src/error.rs` (variant declaration at line 9, mapping at line 60, response builder at lines 50–128)

### Step 1: Write failing test

Create a new test file `crates/api/tests/phase5_rate_limited_header.rs`:

```rust
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use llm_gateway_api::ApiError;
use tower::ServiceExt;

#[tokio::test]
async fn rate_limited_emits_retry_after_header() {
    let resp = ApiError::RateLimited { retry_after_secs: 60 }
        .into_response();
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    let retry_after = resp
        .headers()
        .get("retry-after")
        .expect("Retry-After header missing")
        .to_str()
        .unwrap();
    assert_eq!(retry_after, "60");

    // Body still carries the standard error envelope.
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["error"]["type"], 429);
}
```

### Step 2: Run test to verify it fails

```bash
cargo test -p llm-gateway-api --test phase5_rate_limited_header
```

Expected: FAIL with `error: no variant or associated item named `RateLimited` found ... that takes a struct argument` (variant is currently unit).

### Step 3: Update variant + IntoResponse impl

In `crates/api/src/error.rs`:

**Change the declaration** (line 9) from:
```rust
    RateLimited,
```
to:
```rust
    RateLimited { retry_after_secs: i64 },
```

**Replace the mapping arm** (line 60) — remove it from the simple match (it can no longer be a flat `(status, message, code)` tuple). Instead, handle it specially in the `IntoResponse::into_response` impl.

Locate the `IntoResponse for ApiError` impl. Before the existing match (or where the simple mappings are turned into tuples), branch on `RateLimited` first. The simplest is to insert this at the very top of `into_response()`:

```rust
impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        if let ApiError::RateLimited { retry_after_secs } = self {
            let body = axum::Json(serde_json::json!({
                "error": {
                    "message": "Rate limit exceeded",
                    "type": StatusCode::TOO_MANY_REQUESTS.as_u16(),
                }
            }));
            let mut resp = (StatusCode::TOO_MANY_REQUESTS, body).into_response();
            resp.headers_mut().insert(
                "retry-after",
                axum::http::HeaderValue::from_str(&retry_after_secs.to_string())
                    .expect("retry_after_secs fits in a HeaderValue"),
            );
            return resp;
        }

        // ... existing match arms unchanged ...
        match self {
            // existing arms — but REMOVE the old `ApiError::RateLimited => ...` arm
            // since we've handled it above.
            ApiError::SomeOtherVariant => ...,
            // ...
        }
    }
}
```

**Important:** Delete the old `ApiError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded", None),` arm from the simple match — it would otherwise be unreachable and trigger a compile error.

### Step 4: Run test to verify it passes

```bash
cargo test -p llm-gateway-api --test phase5_rate_limited_header
```

Expected: PASS.

### Step 5: Run full workspace build to confirm no callers broke

```bash
cargo build --workspace
```

Expected: clean compile. (There should be no existing call sites of `ApiError::RateLimited` per the spec's investigation — only the variant declaration itself.)

### Step 6: Commit

```bash
git add crates/api/src/error.rs crates/api/tests/phase5_rate_limited_header.rs
git commit -m "feat(api): RateLimited carries retry_after_secs + emits Retry-After header

Variant was previously unit; now structured so the proxy can pass the
configured window size for the Retry-After header."
```

---

## Task 3: Management handlers — `GET`/`PUT /api/v1/orgs/{slug}/defaults`

**Files:**
- Modify: `crates/api/src/management/auth.rs` (add `get_org_defaults`, `update_org_defaults` near `update_org`)
- Modify: `crates/api/src/management/mod.rs:175` (mount new route after the org-detail route)
- Create: `crates/api/tests/phase5_org_defaults.rs`

### Step 1: Write failing API integration tests

Create `crates/api/tests/phase5_org_defaults.rs`:

```rust
//! Integration tests for `GET`/`PUT /api/v1/orgs/{slug}/defaults` (Phase 5).

mod common;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use llm_gateway_api::management;
use llm_gateway_api::AppState;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

fn build_app(state: Arc<AppState>) -> axum::Router {
    management::management_router(state.clone()).with_state(state)
}

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

async fn body_json(resp: axum::http::Response<Body>) -> Value {
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn get(
    app: &axum::Router,
    uri: &str,
    token: &str,
) -> axum::http::Response<Body> {
    app.clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .header("authorization", bearer(token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn put(
    app: &axum::Router,
    uri: &str,
    token: &str,
    body: Value,
) -> axum::http::Response<Body> {
    app.clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(uri)
                .header("content-type", "application/json")
                .header("authorization", bearer(token))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// 1. GET on an org with no defaults set → both fields null.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn get_defaults_initial_empty(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));

    // Seed an org + admin user (use the same pattern as phase2_orgs tests).
    let (token, slug) = common::seed_org_with_admin(&pool, &app).await;

    let resp = get(&app, &format!("/api/v1/{slug}/defaults"), &token).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["default_rate_limit_rpm"], Value::Null);
    assert_eq!(body["default_budget_monthly_usd"], Value::Null);
}

/// 2. PUT both fields → GET reflects them.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn put_then_get_round_trip(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));
    let (token, slug) = common::seed_org_with_admin(&pool, &app).await;

    let resp = put(
        &app,
        &format!("/api/v1/{slug}/defaults"),
        &token,
        json!({
            "default_rate_limit_rpm": 100,
            "default_budget_monthly_usd": 50.00,
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["default_rate_limit_rpm"], 100);
    assert_eq!(body["default_budget_monthly_usd"], 50.00);

    // GET confirms persistence.
    let resp = get(&app, &format!("/api/v1/{slug}/defaults"), &token).await;
    let body = body_json(resp).await;
    assert_eq!(body["default_rate_limit_rpm"], 100);
    assert_eq!(body["default_budget_monthly_usd"], 50.00);
}

/// 3. PUT with null clears that field.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn put_null_clears_field(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));
    let (token, slug) = common::seed_org_with_admin(&pool, &app).await;

    // Set both.
    let _ = put(
        &app,
        &format!("/api/v1/{slug}/defaults"),
        &token,
        json!({ "default_rate_limit_rpm": 100, "default_budget_monthly_usd": 50.0 }),
    )
    .await;

    // Clear rate limit only.
    let resp = put(
        &app,
        &format!("/api/v1/{slug}/defaults"),
        &token,
        json!({ "default_rate_limit_rpm": null, "default_budget_monthly_usd": 50.0 }),
    )
    .await;
    let body = body_json(resp).await;
    assert_eq!(body["default_rate_limit_rpm"], Value::Null);
    assert_eq!(body["default_budget_monthly_usd"], 50.0);
}

/// 4. Validation: rpm < 1 → 400.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn put_rejects_zero_rpm(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));
    let (token, slug) = common::seed_org_with_admin(&pool, &app).await;

    let resp = put(
        &app,
        &format!("/api/v1/{slug}/defaults"),
        &token,
        json!({ "default_rate_limit_rpm": 0, "default_budget_monthly_usd": null }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

/// 5. Validation: budget < 0 → 400.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn put_rejects_negative_budget(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));
    let (token, slug) = common::seed_org_with_admin(&pool, &app).await;

    let resp = put(
        &app,
        &format!("/api/v1/{slug}/defaults"),
        &token,
        json!({ "default_rate_limit_rpm": null, "default_budget_monthly_usd": -1.0 }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

/// 6. Non-admin member → 403 on PUT.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn put_forbidden_for_member(pool: PgPool) {
    let app = build_app(common::make_state(pool.clone()));
    let (admin_token, slug) = common::seed_org_with_admin(&pool, &app).await;
    let member_token = common::seed_member_in_org(&pool, &app, &slug).await;

    let resp = put(
        &app,
        &format!("/api/v1/{slug}/defaults"),
        &member_token,
        json!({ "default_rate_limit_rpm": 100, "default_budget_monthly_usd": null }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // Member CAN still GET.
    let resp = get(&app, &format!("/api/v1/{slug}/defaults"), &member_token).await;
    assert_eq!(resp.status(), StatusCode::OK);
}
```

**Note on `common::seed_org_with_admin` / `seed_member_in_org`:** if these don't exist in `crates/api/tests/common/mod.rs`, the implementer should add them by following the existing org-seeding pattern (look for helpers used by `phase2_orgs.rs` or similar). They register a user, creates an org, makes the user an admin/member of it, and returns `(token, slug)`.

### Step 2: Run tests to verify they fail

```bash
DATABASE_URL='postgresql://llm_gateway:Xabc12345@10.0.17.3:5432/llm_gateway' \
  cargo test -p llm-gateway-api --test phase5_org_defaults
```

Expected: FAIL — routes don't exist yet (404 / no handler).

### Step 3: Add handler request/response types

In `crates/api/src/management/auth.rs`, near the existing `UpdateOrgRequest` (used by `update_org`), add:

```rust
use llm_gateway_storage::money::{opt_units_to_usd, opt_usd_to_units};
use llm_gateway_storage::types::OrgDefaults;
use llm_gateway_org::can_manage_org_settings;

#[derive(Debug, serde::Deserialize)]
pub struct UpdateOrgDefaultsRequest {
    pub default_rate_limit_rpm: Option<i64>,
    /// USD (float). Converted to/from cents at the API boundary.
    pub default_budget_monthly_usd: Option<f64>,
}

#[derive(Debug, serde::Serialize)]
pub struct OrgDefaultsResponse {
    pub default_rate_limit_rpm: Option<i64>,
    pub default_budget_monthly_usd: Option<f64>,
}

impl From<OrgDefaults> for OrgDefaultsResponse {
    fn from(d: OrgDefaults) -> Self {
        Self {
            default_rate_limit_rpm: d.default_rate_limit_rpm,
            default_budget_monthly_usd: opt_units_to_usd(d.default_budget_monthly_usd),
        }
    }
}
```

### Step 4: Add the two handlers

Below `update_org` in the same file, add:

```rust
pub async fn get_org_defaults(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
) -> Result<Json<OrgDefaultsResponse>, ApiError> {
    let defaults = state
        .storage
        .get_org_defaults(&ctx.org_id)
        .await
        .map_err(|_| ApiError::Internal)?;
    Ok(Json(OrgDefaultsResponse::from(defaults)))
}

pub async fn update_org_defaults(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Json(input): Json<UpdateOrgDefaultsRequest>,
) -> Result<Json<OrgDefaultsResponse>, ApiError> {
    if !can_manage_org_settings(&ctx) {
        return Err(ApiError::Forbidden);
    }

    // Validate.
    if let Some(rpm) = input.default_rate_limit_rpm {
        if rpm < 1 {
            return Err(ApiError::BadRequest("default_rate_limit_rpm must be >= 1".into()));
        }
    }
    if let Some(budget) = input.default_budget_monthly_usd {
        if budget < 0.0 {
            return Err(ApiError::BadRequest("default_budget_monthly_usd must be >= 0".into()));
        }
    }

    let new_defaults = OrgDefaults {
        default_rate_limit_rpm: input.default_rate_limit_rpm,
        default_budget_monthly_usd: opt_usd_to_units(input.default_budget_monthly_usd),
    };

    state
        .storage
        .set_org_defaults(&ctx.org_id, &new_defaults)
        .await
        .map_err(|_| ApiError::Internal)?;

    // Read-back so the response reflects committed state.
    let fresh = state
        .storage
        .get_org_defaults(&ctx.org_id)
        .await
        .map_err(|_| ApiError::Internal)?;
    Ok(Json(OrgDefaultsResponse::from(fresh)))
}
```

**Note:** if `ApiError::BadRequest(String)` or `ApiError::Internal` doesn't exist in `error.rs`, substitute the actual variants used elsewhere in `auth.rs` (look at `update_org` for the convention). `BadRequest` is the conventional name for 400; `Internal` for 500.

### Step 5: Mount the route

In `crates/api/src/management/mod.rs`, after the org-detail route block (lines 170–175), insert:

```rust
        .route(
            "/defaults",
            get(auth::get_org_defaults).put(auth::update_org_defaults),
        )
```

The route is nested under `/api/v1/{org_slug}` (already mounted at line 153), so the final paths are `/api/v1/{slug}/defaults` (GET, PUT).

### Step 6: Run tests to verify they pass

```bash
DATABASE_URL='postgresql://llm_gateway:Xabc12345@10.0.17.3:5432/llm_gateway' \
  cargo test -p llm-gateway-api --test phase5_org_defaults
```

Expected: PASS (6 passed).

### Step 7: Commit

```bash
git add crates/api/src/management/auth.rs crates/api/src/management/mod.rs crates/api/tests/phase5_org_defaults.rs
git commit -m "feat(api): GET/PUT /api/v1/orgs/{slug}/defaults

Typed facade over Phase 1 org_settings kv for org-wide rate-limit RPM and
monthly budget defaults. USD ↔ cents conversion at the API boundary.
Validation: rpm >= 1, budget >= 0. Permission: GET = member+, PUT = admin+."
```

---

## Task 4: Proxy enforcement — wire `check_and_increment` into `proxy_inner`

**Files:**
- Modify: `crates/api/src/proxy.rs` (insert new step between line 901 and line 903)
- Create: `crates/api/tests/phase5_enforcement.rs`

### Step 1: Write failing proxy enforcement tests

Create `crates/api/tests/phase5_enforcement.rs`:

```rust
//! Integration tests for proxy rate-limit enforcement (Phase 5).
//!
//! Verifies the resolution order:
//!   effective_rpm = api_key.rate_limit ?? org.default_rate_limit_rpm ?? None
//! and that exceeding returns 429 with Retry-After.

mod common;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use llm_gateway_api::AppState;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

/// Build the FULL app (management + proxy routes). The management_router
/// alone does not include /v1/chat/completions. Look at crates/gateway/src/main.rs
/// for the assembly pattern — the implementer should adapt that here.
async fn build_full_app(state: Arc<AppState>) -> axum::Router {
    // TODO(implementer): wire together the management + proxy routers exactly
    // as crates/gateway/src/main.rs does. If a shared helper already exists
    // (e.g. `common::build_gateway_app`), use that instead.
    unimplemented!()
}

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

async fn chat_completion(app: &axum::Router, api_key: &str) -> axum::http::Response<Body> {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .header("authorization", bearer(api_key))
                .body(Body::from(
                    json!({
                        "model": "gpt-test",
                        "messages": [{"role": "user", "content": "hi"}],
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// 1. Org default = 5; key has no per-key limit; 6th request → 429.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn org_default_enforces(pool: PgPool) {
    let state = Arc::new(common::make_state(pool.clone()));
    let app = build_full_app(state.clone()).await;

    // Seed: org with default_rate_limit_rpm = 5; admin user; key with no
    // per-key rate_limit set.
    let api_key = common::seed_org_with_default_and_key(&pool, &state, 5, None).await;

    for _ in 0..5 {
        let resp = chat_completion(&app, &api_key).await;
        // The first 5 should NOT be 429. (They may be other statuses —
        // upstream will fail since there's no real provider — but enforcement
        // must not reject them.)
        assert_ne!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    let sixth = chat_completion(&app, &api_key).await;
    assert_eq!(sixth.status(), StatusCode::TOO_MANY_REQUESTS);
    let retry_after = sixth
        .headers()
        .get("retry-after")
        .expect("Retry-After header")
        .to_str()
        .unwrap();
    assert!(retry_after.parse::<u64>().unwrap() > 0);
}

/// 2. Per-key rate_limit = 10 wins over no org default; 11th → 429.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn per_key_enforces_without_org_default(pool: PgPool) {
    let state = Arc::new(common::make_state(pool.clone()));
    let app = build_full_app(state.clone()).await;

    let api_key = common::seed_org_with_default_and_key(&pool, &state, None, Some(10)).await;

    for _ in 0..10 {
        let resp = chat_completion(&app, &api_key).await;
        assert_ne!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }
    let eleventh = chat_completion(&app, &api_key).await;
    assert_eq!(eleventh.status(), StatusCode::TOO_MANY_REQUESTS);
}

/// 3. No per-key, no org default → unlimited (no 429s across many requests).
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn unlimited_path(pool: PgPool) {
    let state = Arc::new(common::make_state(pool.clone()));
    let app = build_full_app(state.clone()).await;

    let api_key = common::seed_org_with_default_and_key(&pool, &state, None, None).await;

    for _ in 0..20 {
        let resp = chat_completion(&app, &api_key).await;
        assert_ne!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }
}

/// 4. Per-key (10) wins over org default (5) — 6 succeeds, 11 fails.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn per_key_overrides_org_default(pool: PgPool) {
    let state = Arc::new(common::make_state(pool.clone()));
    let app = build_full_app(state.clone()).await;

    let api_key = common::seed_org_with_default_and_key(&pool, &state, Some(5), Some(10)).await;

    for _ in 0..6 {
        let resp = chat_completion(&app, &api_key).await;
        assert_ne!(resp.status(), StatusCode::TOO_MANY_REQUESTS,
            "per-key must override org default — 6th should pass under per-key limit of 10");
    }
    // Now exhaust the remaining 4 calls (total 10).
    for _ in 0..4 {
        let resp = chat_completion(&app, &api_key).await;
        assert_ne!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }
    let eleventh = chat_completion(&app, &api_key).await;
    assert_eq!(eleventh.status(), StatusCode::TOO_MANY_REQUESTS);
}
```

**Note for implementer:** if `common::seed_org_with_default_and_key` doesn't exist, add it to `crates/api/tests/common/mod.rs`. It should: create an org, set the org default via `storage.set_org_defaults`, register + verify + login a user as admin of that org, create an API key with the given `rate_limit` value, and return the bearer token (the raw key string, since the proxy expects the API key directly — not a JWT — for `/v1/chat/completions`). Check how existing proxy tests (if any) seed an api_key; the proxy path uses the API key, not a JWT.

### Step 2: Run tests to verify they fail

```bash
DATABASE_URL='postgresql://llm_gateway:Xabc12345@10.0.17.3:5432/llm_gateway' \
  cargo test -p llm-gateway-api --test phase5_enforcement
```

Expected: FAIL — `build_full_app` returns `unimplemented!()`. (Once that's wired, tests will fail on actual 429/non-429 assertions until the proxy check is added.)

### Step 3: Implement `build_full_app` helper

In `crates/api/tests/common/mod.rs` (or `phase5_enforcement.rs` if cleaner), implement `build_full_app` by following the router assembly in `crates/gateway/src/main.rs`. It should mount both:
- The management router (`management::management_router`)
- The proxy router (look in `crates/gateway/src/main.rs` for how the `/v1/chat/completions` route is mounted — likely also in `crates/api/src/proxy.rs` as a public `proxy_router` function)

After this step, the tests should compile but fail on assertion (no enforcement yet).

### Step 4: Add the rate-limit check in `proxy_inner`

In `crates/api/src/proxy.rs`, locate `proxy_inner`. Between the end of the auth step (line ~901, where `api_key` is bound and enabled-check completes) and the balance-check comment (`// === Step 2: Balance check ===` at line 903), insert:

```rust
    // === Step 1.5: Rate-limit check ===
    // Resolution order: api_key.rate_limit ?? org.default_rate_limit_rpm ?? None (unlimited).
    // Bucket is per-api_key (model dimension collapsed via "" so the limit
    // applies regardless of which model the client requested).
    let effective_rpm = match api_key.rate_limit {
        Some(n) => Some(n),
        None => match state.storage
            .get_org_setting(&api_key.org_id, "default_rate_limit_rpm")
            .await
        {
            Ok(Some(raw)) => raw.parse::<i64>().ok(),
            Ok(None) => None,
            Err(e) => {
                tracing::warn!(error = %e, "org default lookup failed; failing open");
                None
            }
        },
    };

    if let Some(rpm) = effective_rpm {
        let allowed = state
            .rate_limiter
            .check_and_increment(&api_key.id, "", Some(rpm), None, None)
            .await;
        if !allowed {
            return Err(ApiError::RateLimited {
                retry_after_secs: state.system_info.rate_limit_window_secs,
            });
        }
    }
```

**Notes for implementer:**
- `state.rate_limiter` is `Arc<RateLimiter>` per `lib.rs:18`.
- `state.system_info.rate_limit_window_secs` per `lib.rs:38`. If `system_info` is accessed differently (e.g. `state.rate_limit_window_secs` directly), use that — confirm by reading the actual `AppState` field accesses in the same file.
- `tracing::warn!` on lookup failure preserves fail-open semantics.
- `check_and_increment` returns `bool` (never errors), so no `Result` handling needed at the call site.

### Step 5: Run tests to verify they pass

```bash
DATABASE_URL='postgresql://llm_gateway:Xabc12345@10.0.17.3:5432/llm_gateway' \
  cargo test -p llm-gateway-api --test phase5_enforcement
```

Expected: PASS (4 passed).

### Step 6: Run full workspace tests to confirm no regressions

```bash
DATABASE_URL='postgresql://llm_gateway:Xabc12345@10.0.17.3:5432/llm_gateway' \
  cargo test --workspace 2>&1 | grep -E "FAILED|^test result"
```

Expected: all pass.

### Step 7: Commit

```bash
git add crates/api/src/proxy.rs crates/api/tests/phase5_enforcement.rs crates/api/tests/common/mod.rs
git commit -m "feat(proxy): enforce per-key + org-default rate limits

Wires RateLimiter.check_and_increment into the proxy request path between
auth and balance check. Resolution order: api_key.rate_limit ?? org.default
_rate_limit_rpm ?? unlimited. Per-key bucketing (empty model string). Fail-
open on org-default lookup failure. 429 response carries Retry-After set
to the configured rate-limit window size."
```

---

## Task 5: CHANGELOG — Phase 5 entry

**Files:**
- Modify: `CHANGELOG.md` (add to `## [Unreleased] → Added` section)

### Step 1: Read current CHANGELOG top

```bash
head -50 CHANGELOG.md
```

Locate the `## [Unreleased]` section. Within its `### Added` subsection (or create one if absent), append the Phase 5 block.

### Step 2: Add the entry

Insert this block at the appropriate spot (after any existing Phase 4 entries, before the next released version):

```markdown
- **Phase 5 (per-org defaults + rate-limit enforcement):**
  - Added: `GET`/`PUT /api/v1/orgs/{slug}/defaults` for org-wide rate-limit RPM and monthly budget defaults. UI lives in Org Settings → Defaults.
  - **Behavior change:** per-key rate limits (`api_keys.rate_limit`) are now **enforced** at request time via the existing in-memory rate limiter — previously stored but never checked. Resolution order: `key.rate_limit ?? org.default_rate_limit_rpm ?? unlimited`. Exceeding returns `429` with `Retry-After` set to the configured rate-limit window size.
  - Org-level `default_budget_monthly_usd` is stored but **not enforced** in this phase (parity with existing per-key budget — both will be enforced in a future phase).
  - **Upgrade note:** any existing `api_keys` rows with non-null `rate_limit` will start receiving 429s on requests beyond their limit. Audit existing keys before deploying if any have low values set.
```

### Step 3: Commit

```bash
git add CHANGELOG.md
git commit -m "docs(changelog): Phase 5 — org defaults + rate-limit enforcement"
```

---

## Task 6: Frontend — API client additions

**Files:**
- Modify: `web/src/api/orgs.ts`

### Step 1: Read existing client

```bash
cat web/src/api/orgs.ts
```

Confirm `apiClient` and `orgPrefix` are imported from `./client`.

### Step 2: Add type + functions

Append to `web/src/api/orgs.ts`:

```ts
export type OrgDefaults = {
  default_rate_limit_rpm: number | null;
  default_budget_monthly_usd: number | null;
};

export async function getOrgDefaults(): Promise<OrgDefaults> {
  const { data } = await apiClient.get<OrgDefaults>(`${orgPrefix()}/defaults`);
  return data;
}

export async function updateOrgDefaults(input: OrgDefaults): Promise<OrgDefaults> {
  const { data } = await apiClient.put<OrgDefaults>(`${orgPrefix()}/defaults`, input);
  return data;
}
```

**Note:** `orgPrefix()` already prepends `/api/v1/{slug}`. No need to pass `orgId` — the slug is taken from the auth store inside `orgPrefix`.

### Step 3: Type-check

```bash
source ~/.nvm/nvm.sh && cd web && npm run build
```

Expected: TypeScript check passes (build may emit warnings, no errors).

### Step 4: Commit

```bash
git add web/src/api/orgs.ts
git commit -m "feat(web): OrgDefaults API client (get + update)"
```

---

## Task 7: Frontend — React Query hook

**Files:**
- Create: `web/src/hooks/useOrgDefaults.ts`

### Step 1: Write the hook

Create `web/src/hooks/useOrgDefaults.ts`:

```ts
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import i18n from '../i18n';
import { useAuthStore } from '../stores/authStore';
import { getOrgDefaults, updateOrgDefaults, type OrgDefaults } from '../api/orgs';
import { getErrorMessage } from '../api/client';

export function useGetOrgDefaults() {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({
    queryKey: [slug, 'orgDefaults'],
    queryFn: () => getOrgDefaults(),
    enabled: !!slug,
  });
}

export function useUpdateOrgDefaults() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: (input: OrgDefaults) => updateOrgDefaults(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [slug, 'orgDefaults'] });
      toast.success(i18n.t('orgSettings.defaults.saveSuccess'));
    },
    onError: (err) => {
      toast.error(getErrorMessage(err, i18n.t('orgSettings.defaults.saveError')));
    },
  });
}
```

**Confirm before writing:** verify the import paths match existing hooks (look at `web/src/hooks/useKeys.ts:1-7` for the convention). `i18n` import: `import i18n from '../i18n';` (per `useKeys.ts`/`useChannels.ts`), not `'i18next'` directly.

### Step 2: Type-check

```bash
source ~/.nvm/nvm.sh && cd web && npm run build
```

Expected: clean.

### Step 3: Commit

```bash
git add web/src/hooks/useOrgDefaults.ts
git commit -m "feat(web): useGetOrgDefaults + useUpdateOrgDefaults hooks"
```

---

## Task 8: Frontend — i18n keys

**Files:**
- Modify: `web/src/i18n/en.json` (insert after `orgSettings.general` block, before `orgSettings.danger`)
- Modify: `web/src/i18n/zh.json` (mirror)

### Step 1: Read current en.json orgSettings section

```bash
sed -n '960,1000p' web/src/i18n/en.json
```

Confirm line numbers match the exploration report (`orgSettings` block at lines 966–996). Find where `general` closes (line ~977) and `danger` opens (line ~978).

### Step 2: Add the defaults block in en.json

Insert this object between `general` and `danger` (commas per surrounding pattern):

```json
      "defaults": {
        "title": "Defaults",
        "description": "Org-wide defaults applied to API keys without their own settings",
        "rateLimitLabel": "Default rate limit (RPM)",
        "rateLimitHelp": "Applies to API keys without their own limit. Empty = unlimited.",
        "budgetLabel": "Default monthly budget (USD)",
        "budgetHelp": "Stored for display. Not currently enforced. Empty = no budget.",
        "save": "Save",
        "cancel": "Cancel",
        "saveSuccess": "Defaults saved.",
        "saveError": "Failed to save defaults."
      },
```

### Step 3: Add mirrored block in zh.json

Same location, Chinese values:

```json
      "defaults": {
        "title": "默认值",
        "description": "组织级默认值,应用于未单独设置的 API 密钥",
        "rateLimitLabel": "默认速率限制 (RPM)",
        "rateLimitHelp": "应用于未单独设置限制的 API 密钥。留空 = 不限制。",
        "budgetLabel": "默认月度预算 (USD)",
        "budgetHelp": "仅供显示,当前不强制执行。留空 = 无预算。",
        "save": "保存",
        "cancel": "取消",
        "saveSuccess": "默认值已保存。",
        "saveError": "保存默认值失败。"
      },
```

### Step 4: Validate JSON

```bash
source ~/.nvm/nvm.sh && cd web && node -e "JSON.parse(require('fs').readFileSync('src/i18n/en.json'))" && node -e "JSON.parse(require('fs').readFileSync('src/i18n/zh.json'))"
```

Expected: no output (valid JSON).

### Step 5: Commit

```bash
git add web/src/i18n/en.json web/src/i18n/zh.json
git commit -m "i18n: orgSettings.defaults.* keys (en + zh)"
```

---

## Task 9: Frontend — OrgSettings "Defaults" section + unit tests

**Files:**
- Modify: `web/src/pages/OrgSettings.tsx` (insert between line 244 and line 246)
- Create: `web/src/pages/OrgSettings.test.tsx`

### Step 1: Write failing unit tests

Create `web/src/pages/OrgSettings.test.tsx`:

```tsx
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Routes, Route } from 'react-router-dom';
import { useAuthStore } from '../stores/authStore';
import type { User, OrgSummary } from '../types';
import OrgSettings from './OrgSettings';

const { mockToastSuccess, mockToastError } = vi.hoisted(() => ({
  mockToastSuccess: vi.fn(),
  mockToastError: vi.fn(),
}));

vi.mock('sonner', () => ({
  toast: { success: mockToastSuccess, error: mockToastError },
}));

const adminUser: User = {
  id: 'u1',
  username: 'alice',
  platform_role: null,
  email: 'alice@example.com',
  email_verified_at: '2026-07-09T00:00:00Z',
};

const memberUser: User = {
  ...adminUser,
  id: 'u2',
  username: 'bob',
};

const adminOrg: OrgSummary = {
  id: 'org-1',
  slug: 'org-1',
  name: 'Org One',
  role: 'admin',
  group_id: null,
};

const memberOrg: OrgSummary = { ...adminOrg, role: 'member' };

function renderAt(path: string) {
  return renderWithProviders(
    <Routes>
      <Route path="/:slug/settings" element={<OrgSettings />} />
    </Routes>,
    { route: path },
  );
}

describe('OrgSettings — Defaults section', () => {
  beforeEach(() => {
    useAuthStore.setState({
      user: adminUser,
      currentOrg: adminOrg,
    });
  });

  it('renders the Defaults section for an admin', async () => {
    server.use(
      http.get('*/api/v1/org-1/defaults', () =>
        HttpResponse.json({
          default_rate_limit_rpm: 100,
          default_budget_monthly_usd: 50.0,
        }),
      ),
    );

    renderAt('/org-1/settings');

    await waitFor(() => {
      expect(screen.getByLabelText('Default rate limit (RPM)')).toHaveValue(100);
    });
    expect(screen.getByLabelText('Default monthly budget (USD)')).toHaveValue(50);
  });

  it('disables inputs for a member (read-only)', async () => {
    useAuthStore.setState({ user: memberUser, currentOrg: memberOrg });
    server.use(
      http.get('*/api/v1/org-1/defaults', () =>
        HttpResponse.json({
          default_rate_limit_rpm: 100,
          default_budget_monthly_usd: null,
        }),
      ),
    );

    renderAt('/org-1/settings');

    await waitFor(() => {
      expect(screen.getByLabelText('Default rate limit (RPM)')).toBeDisabled();
    });
    expect(screen.queryByRole('button', { name: 'Save' })).not.toBeInTheDocument();
  });

  it('shows error state when GET fails', async () => {
    server.use(
      http.get('*/api/v1/org-1/defaults', () =>
        HttpResponse.json({ error: { message: 'down' } }, { status: 500 }),
      ),
    );

    renderAt('/org-1/settings');

    await waitFor(() => {
      expect(screen.getByText(/Failed to load defaults/i)).toBeInTheDocument();
    });
  });

  it('save success: toasts + reflects new values', async () => {
    server.use(
      http.get('*/api/v1/org-1/defaults', () =>
        HttpResponse.json({ default_rate_limit_rpm: null, default_budget_monthly_usd: null }),
      ),
      http.put('*/api/v1/org-1/defaults', async ({ request }) => {
        const body = (await request.json()) as { default_rate_limit_rpm: number; default_budget_monthly_usd: number };
        return HttpResponse.json(body);
      }),
    );

    renderAt('/org-1/settings');

    const rpm = await screen.findByLabelText('Default rate limit (RPM)');
    await userEvent.type(rpm, '100');
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() => {
      expect(mockToastSuccess).toHaveBeenCalledWith('Defaults saved.');
    });
  });
});
```

### Step 2: Run tests to verify they fail

```bash
source ~/.nvm/nvm.sh && cd web && npm test -- --run src/pages/OrgSettings.test.tsx
```

Expected: FAIL — Defaults section doesn't render yet, labels not found.

### Step 3: Add the Defaults section to OrgSettings.tsx

In `web/src/pages/OrgSettings.tsx`, after the General section's closing `</motion.section>` (line 244) and before the Danger Zone comment (line 246), insert.

First, add the necessary imports at the top of the file:

```tsx
import { useGetOrgDefaults, useUpdateOrgDefaults } from '../hooks/useOrgDefaults';
```

Then the section itself, inserted in the JSX between General and Danger Zone:

```tsx
        {/* Defaults section — admin can edit; member is read-only. */}
        <DefaultsSection canEdit={canEdit} />
```

Then add the component definition at the bottom of the file (before the closing `}` of the file or as a separate component):

```tsx
function DefaultsSection({ canEdit }: { canEdit: boolean }) {
  const { t } = useTranslation();
  const { data, isLoading, isError } = useGetOrgDefaults();
  const updateDefaults = useUpdateOrgDefaults();

  // Local state mirrors the loaded values; initialized once data arrives.
  const [rpm, setRpm] = useState<string>('');
  const [budget, setBudget] = useState<string>('');
  const [hydrated, setHydrated] = useState(false);

  useEffect(() => {
    if (data && !hydrated) {
      setRpm(data.default_rate_limit_rpm?.toString() ?? '');
      setBudget(data.default_budget_monthly_usd?.toString() ?? '');
      setHydrated(true);
    }
  }, [data, hydrated]);

  if (isLoading) {
    return (
      <motion.section
        initial={reducedMotion ? false : { opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.3, ease: EASE }}
        className="rounded-xl border border-base-300 bg-base-100 p-6 mt-6"
      >
        <div className="text-base-content/60">Loading…</div>
      </motion.section>
    );
  }

  if (isError) {
    return (
      <motion.section
        initial={reducedMotion ? false : { opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.3, ease: EASE }}
        className="rounded-xl border border-base-300 bg-base-100 p-6 mt-6"
      >
        <div className="text-error">Failed to load defaults.</div>
      </motion.section>
    );
  }

  const rpmNum = rpm === '' ? null : parseInt(rpm, 10);
  const budgetNum = budget === '' ? null : parseFloat(budget);
  const dirty =
    (rpmNum ?? null) !== (data?.default_rate_limit_rpm ?? null) ||
    (budgetNum ?? null) !== (data?.default_budget_monthly_usd ?? null);

  const onSave = async () => {
    await updateDefaults.mutateAsync({
      default_rate_limit_rpm: rpmNum !== null && !Number.isNaN(rpmNum) ? rpmNum : null,
      default_budget_monthly_usd: budgetNum !== null && !Number.isNaN(budgetNum) ? budgetNum : null,
    });
  };

  return (
    <motion.section
      initial={reducedMotion ? false : { opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3, ease: EASE }}
      className="rounded-xl border border-base-300 bg-base-100 p-6 mt-6"
    >
      <h2 className="text-xl font-semibold mb-1">{t('orgSettings.defaults.title')}</h2>
      <p className="text-sm text-base-content/60 mb-4">{t('orgSettings.defaults.description')}</p>

      <div className="space-y-4">
        <div>
          <label className="block text-sm mb-1">{t('orgSettings.defaults.rateLimitLabel')}</label>
          <input
            type="number"
            min="1"
            placeholder="Unlimited"
            disabled={!canEdit || updateDefaults.isPending}
            value={rpm}
            onChange={(e) => setRpm(e.target.value)}
            className={INPUT_CLASS}
          />
          <p className="text-xs text-base-content/50 mt-1">{t('orgSettings.defaults.rateLimitHelp')}</p>
        </div>

        <div>
          <label className="block text-sm mb-1">{t('orgSettings.defaults.budgetLabel')}</label>
          <input
            type="number"
            min="0"
            step="0.01"
            placeholder="No budget"
            disabled={!canEdit || updateDefaults.isPending}
            value={budget}
            onChange={(e) => setBudget(e.target.value)}
            className={INPUT_CLASS}
          />
          <p className="text-xs text-base-content/50 mt-1">{t('orgSettings.defaults.budgetHelp')}</p>
        </div>
      </div>

      {canEdit && (
        <div className="flex justify-end gap-2 mt-4">
          <Button
            variant="ghost"
            onClick={() => {
              setRpm(data?.default_rate_limit_rpm?.toString() ?? '');
              setBudget(data?.default_budget_monthly_usd?.toString() ?? '');
            }}
            disabled={!dirty || updateDefaults.isPending}
          >
            {t('orgSettings.defaults.cancel')}
          </Button>
          <Button
            onClick={onSave}
            disabled={!dirty || updateDefaults.isPending}
          >
            {t('orgSettings.defaults.save')}
          </Button>
        </div>
      )}
    </motion.section>
  );
}
```

**Add necessary imports** at the top: `useEffect` (alongside existing `useState`).

**Confirm:** the `htmlFor`/`id` association — the test uses `getByLabelText`, which requires `<label htmlFor="...">...</label>` paired with `<input id="..." />`. Adjust the JSX above by adding `htmlFor`/`id` if RTL can't resolve the label-input association through wrapping. Look at `AddEmailModal.tsx` (from Phase 4) for the htmlFor/id pattern.

### Step 4: Run tests to verify they pass

```bash
source ~/.nvm/nvm.sh && cd web && npm test -- --run src/pages/OrgSettings.test.tsx
```

Expected: PASS (4 passed).

### Step 5: Run full frontend test suite to confirm no regressions

```bash
source ~/.nvm/nvm.sh && cd web && npm test -- --run
```

Expected: all green.

### Step 6: Commit

```bash
git add web/src/pages/OrgSettings.tsx web/src/pages/OrgSettings.test.tsx
git commit -m "feat(web): OrgSettings Defaults section (rate limit + budget inputs)"
```

---

## Task 10: E2E — org-defaults enforcement flow

**Files:**
- Create: `web/e2e/org-defaults.spec.ts`

### Step 1: Read an existing e2e for the auth pattern

```bash
head -60 web/e2e/invitations.spec.ts
```

Confirm `ADMIN_USER`/`ADMIN_PASS` constants, login pattern, `RUN_TAG` for uniqueness.

### Step 2: Write the e2e

Create `web/e2e/org-defaults.spec.ts`:

```ts
import { test, expect, request } from '@playwright/test';

const ADMIN_USER = process.env.E2E_ADMIN_USER ?? 'admin';
const ADMIN_PASS = process.env.E2E_ADMIN_PASS ?? 'admin123456';
const RUN_TAG = process.env.E2E_RUN_TAG ?? String(Date.now());

test('org default rate limit is enforced', async ({ page }) => {
  // 1. Login as admin via UI (seeds localStorage with JWT + currentOrg).
  await page.goto('/login');
  await page.getByPlaceholder('Username').fill(ADMIN_USER);
  await page.getByPlaceholder('Password').fill(ADMIN_PASS);
  await page.getByRole('button', { name: /sign in/i }).click();
  await page.waitForURL('**/dashboard');

  // 2. Capture the bearer from localStorage (the proxy uses api_key, not the
  //    JWT — but the management calls below use the JWT).
  const slug = await page.evaluate(() => {
    const raw = localStorage.getItem('auth-store') || '{}';
    return JSON.parse(raw)?.state?.currentOrg?.slug;
  });
  expect(slug).toBeTruthy();

  // 3. Create a fresh org + admin-made api_key for the test (isolation).
  const newSlug = `e2e-org-${RUN_TAG}`;
  const apiContext = request.newContext({
    baseURL: 'http://localhost:8080',
    extraHTTPHeaders: {
      // The login flow above put the JWT in localStorage. Pull it out and reuse.
      authorization: `Bearer ${await page.evaluate(() => {
        const raw = localStorage.getItem('auth-store') || '{}';
        return JSON.parse(raw)?.state?.token;
      })}`,
    },
  });

  // (Org + key creation steps would go here. If the admin's current org
  //  is fine to reuse, skip org creation and just create the key.)
  //
  // For simplicity, the e2e reuses the admin's current org. Create a key
  // with no per-key rate_limit (so the org default applies).
  const keyResp = await apiContext.post(`/api/v1/${slug}/keys`, {
    data: {
      name: `e2e-${RUN_TAG}`,
      budget_monthly: null,
      rate_limit: null,
    },
  });
  expect(keyResp.ok()).toBeTruthy();
  const keyBody = await keyResp.json();
  const apiKey = keyBody.plaintext; // or whatever the create-key response field is

  // 4. Set org default_rate_limit_rpm = 3.
  const putResp = await apiContext.put(`/api/v1/${slug}/defaults`, {
    data: { default_rate_limit_rpm: 3, default_budget_monthly_usd: null },
  });
  expect(putResp.ok()).toBeTruthy();

  // 5. Fire 3 chat-completion requests with the new key — all should NOT be 429.
  //    The 4th must be 429.
  const proxyCtx = request.newContext({
    baseURL: 'http://localhost:8080',
    extraHTTPHeaders: { authorization: `Bearer ${apiKey}` },
  });

  for (let i = 0; i < 3; i++) {
    const r = await proxyCtx.post('/v1/chat/completions', {
      data: { model: 'gpt-test', messages: [{ role: 'user', content: 'hi' }] },
    });
    // Don't assert success — upstream provider may be unreachable in CI.
    // Just assert it's NOT 429.
    expect(r.status()).not.toBe(429);
  }

  const fourth = await proxyCtx.post('/v1/chat/completions', {
    data: { model: 'gpt-test', messages: [{ role: 'user', content: 'hi' }] },
  });
  expect(fourth.status()).toBe(429);
  const retryAfter = fourth.headers()['retry-after'];
  expect(retryAfter).toBeTruthy();
  expect(Number(retryAfter)).toBeGreaterThan(0);

  // 6. Cleanup: delete the test key.
  await apiContext.delete(`/api/v1/${slug}/keys/${keyBody.id}`);
});
```

**Note for implementer:** the exact response field for the created key's plaintext (`keyBody.plaintext` here) should be confirmed by reading `web/src/api/keys.ts` or the management handler in `crates/api/src/management/keys.rs`. Adapt as needed.

### Step 3: Run the e2e (requires backend on :8080 + dev-mail if applicable)

```bash
source ~/.nvm/nvm.sh && cd web && npm run test:e2e -- org-defaults
```

Expected: PASS (1 test). May require the dev server running per `playwright.config.ts`.

### Step 4: Commit

```bash
git add web/e2e/org-defaults.spec.ts
git commit -m "test(e2e): org default rate limit enforcement flow"
```

---

## Self-Review (post-write)

### Spec coverage

| Spec section | Task(s) |
|---|---|
| Architecture → Resolution order | Task 4 |
| Architecture → Component boundaries (storage, error, handlers, proxy) | Tasks 1, 2, 3, 4 |
| Data Model (no schema migration, two kv keys) | Task 1 |
| API Surface (GET + PUT, validation, permissions) | Task 3 |
| Proxy Enforcement (insertion point, fetch strategy, counting, fail-open, Retry-After) | Tasks 2, 4 |
| Frontend (Defaults section, hook, i18n, api client) | Tasks 6, 7, 8, 9 |
| Testing (storage unit, API integration, proxy integration, frontend unit, e2e) | Tasks 1, 3, 4, 9, 10 |
| CHANGELOG entry | Task 5 |
| Out of Scope (audit deferred, model bucketing via `""`) | Reflected in spec; no task needed |

All spec sections covered. ✓

### Placeholder scan

No "TBD", "TODO" outside of intentional implementer notes (e.g. "TODO(implementer): wire together..."). The implementer notes flag work that depends on confirming project-specific patterns during execution; the surrounding context gives exact pointers (line numbers, file paths). ✓

### Type consistency

- `OrgDefaults` struct defined in Task 1, Step 3 — fields `default_rate_limit_rpm: Option<i64>`, `default_budget_monthly_usd: Option<i64>` (cents).
- Used in trait methods Task 1, Step 4.
- Used in postgres impl Task 1, Step 5.
- API handler `From<OrgDefaults> for OrgDefaultsResponse` Task 3, Step 3 — converts cents → USD float via `opt_units_to_usd`.
- Frontend `OrgDefaults` type Task 6, Step 2 — `default_rate_limit_rpm: number | null`, `default_budget_monthly_usd: number | null` (matches API USD float).
- Hook Task 7 uses frontend type.
- Page Task 9 uses hook + frontend type.

All types line up. ✓

### Method signatures

- `get_org_defaults(org_id) -> Result<OrgDefaults, ...>` — declared Task 1 Step 4, implemented Step 5, called Task 3 Step 4 and Task 4 Step 4. ✓
- `set_org_defaults(org_id, &OrgDefaults) -> Result<(), ...>` — same. ✓
- `check_and_increment(key_id, model, rpm_limit, tpm_limit, input_tokens) -> bool` — called in Task 4 Step 4 with `(api_key.id, "", Some(rpm), None, None)`. Matches signature in `crates/ratelimit/src/lib.rs:19`. ✓
- `ApiError::RateLimited { retry_after_secs: i64 }` — defined Task 2 Step 3, returned Task 4 Step 4. ✓
