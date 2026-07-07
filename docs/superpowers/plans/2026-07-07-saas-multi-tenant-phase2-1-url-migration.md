# SaaS Multi-Tenant Orgs — Phase 2, Plan 2.1: URL Migration Foundation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move every management endpoint under `/api/v1/{org_slug}/...`, install the AuthLayer → OrgResolveLayer → MembershipLayer middleware chain, return 410 Gone for legacy paths, and migrate the frontend to `/:orgSlug/*` routes with a working OrgSwitcher. After this plan ships, users can create multiple orgs and switch between them — but each org behaves like the single-tenant gateway of v2.0.0 (no members page, no per-org catalog yet).

**Architecture:** The current codebase does auth via a per-handler `require_auth(&headers, &state.jwt_secret)?` call. We introduce three Axum `from_fn` middlewares that run in order on every `/api/v1/{org_slug}/*` route: `auth_layer` (validates JWT, injects `JwtClaims`), `org_resolve_layer` (looks up the org by slug, injects `ResolvedOrg`), and `membership_layer` (verifies the user belongs to the org, injects `OrgContext`). Handlers switch from reading `claims.current_org_id` to pulling `OrgContext` from the request — same data, cleaner contract. The frontend migrates `/console/*` → `/:orgSlug/*` and `/admin/*` → `/:orgSlug/admin/*`, React Query keys get prefixed with the slug, and the API client gains an `orgPrefix()` helper.

**Tech Stack:** Rust (Axum 0.7 `from_fn` middleware, `axum::extract::FromRequestParts`), React + TypeScript + React Router v6 + Zustand + React Query.

**Spec reference:** `docs/superpowers/specs/2026-07-07-saas-multi-tenant-orgs-design.md` — Phase 2 deliverables (lines 978-993), Request lifecycle (lines 76-87), Org-scoped endpoints (lines 691-708), Frontend Changes (lines 765-862).

---

## Scope Decomposition

Phase 2 from the spec is too large for one plan (~3× Phase 1's scope). Decomposed into three plans, each independently shippable:

- **Plan 2.1 (this plan):** URL migration foundation. Breaking change for API consumers. Ships the new route structure end-to-end. No new business logic — every org behaves like v2.0.0's default org.
- **Plan 2.2 (future):** Members page + Org Settings page. Backend members CRUD with last-owner guard; org PATCH/DELETE with password confirmation; frontend pages.
- **Plan 2.3 (future):** Platform-admin impersonation + catalog filter. Temp member rows + janitor; anti-shadowing enforcement; "Platform" vs "Ours" UI filter; "Viewing as org X" indicator.

Plans 2.2 and 2.3 can ship in either order after 2.1.

---

## File Structure

### Create

**Backend**
- `crates/api/src/middleware.rs` — three `from_fn` middlewares (auth, org_resolve, membership)
- `crates/api/src/management/orgs.rs` — `POST /api/v1/orgs` (create) + `GET /api/v1/{org_slug}` (read)

**Frontend**
- `web/src/components/OrgSwitcher.tsx` — sidebar dropdown
- `web/src/components/OrgRouteGuard.tsx` — validates `:orgSlug` against `useAuthStore.orgs`

### Modify

**Backend**
- `crates/api/src/lib.rs` — re-export middleware module
- `crates/api/src/extractors.rs` — `require_auth` stays as a helper used by the auth layer; no longer called per-handler
- `crates/api/src/management/mod.rs` — restructure `management_router()` to put org-scoped routes under `/api/v1/{org_slug}/*`, add legacy 410 catch-all, wire middleware chain, register new routes
- `crates/api/src/management/keys.rs`, `channels.rs`, `channel_models.rs`, `providers.rs`, `models.rs`, `groups.rs`, `usage.rs`, `logs.rs`, `requests.rs`, `users.rs`, `accounts.rs`, `settings.rs`, `seed.rs`, `pricing_policies.rs`, `nats.rs`, `model_fallbacks.rs` — every handler drops `headers: HeaderMap` + `require_auth(&headers, ...)?` and gains `ctx: OrgContext` instead; reads `ctx.org_id` rather than `claims.current_org_id`
- `crates/org/src/extractors.rs` — `OrgContext` implements `FromRequestParts` (reads from request extensions)
- `crates/org/src/types.rs` — add `ResolvedOrg { id, slug, name }` lightweight struct (injected by `org_resolve_layer`)

**Frontend**
- `web/src/App.tsx` — replace `/console/*` and `/admin/*` with `/:orgSlug/*` and `/:orgSlug/admin/*`; add `OrgRouteGuard`
- `web/src/components/Layout.tsx` — render `OrgSwitcher` at the top of the sidebar
- `web/src/components/RequireAuth.tsx` (or wherever `RequireAuth` / `RequireAdmin` live) — `RequireAdmin` reads `currentOrg.role` instead of `user.role`
- `web/src/api/client.ts` — add `orgPrefix(): string` helper that reads `useAuthStore.getState().currentOrg?.slug`
- `web/src/api/keys.ts`, `channels.ts`, `providers.ts`, `models.ts`, `groups.ts`, `usage.ts`, `logs.ts`, `users.ts`, `accounts.ts`, `settings.ts`, `pricing-policies.ts`, `channel-models.ts`, `model-fallbacks.ts`, `seed.ts` — every org-scoped endpoint prepends `orgPrefix()`
- `web/src/hooks/*.ts` — every React Query `queryKey` gains the current org slug as the first element
- `web/src/stores/authStore.ts` — `setCurrentOrg(org)` action: calls `POST /api/v1/me/current-org`, updates tokens, sets `currentOrg`

### Unchanged (intentionally)

- `crates/api/src/proxy.rs` — proxy paths (`/v1/chat/completions`, `/v1/messages`) are not org-scoped at the URL level. `api_key.org_id` resolution at the auth layer is unchanged from Phase 1.
- `crates/api/src/management/auth.rs` (login, register, me, refresh, change-password) — these stay at `/api/v1/auth/*` and `/api/v1/me/*`, no `org_slug` prefix.
- `crates/storage/` — no schema changes in Plan 2.1. All Phase 1 tables and methods are reused.

---

## Deployment Notes

**Breaking change.** Every existing API client URL changes from `/api/v1/keys` to `/api/v1/{org_slug}/keys`. The frontend and backend must ship together — a v2.1.0 release with a prominent CHANGELOG entry listing every renamed endpoint and a one-week deprecation window for any known external integrators.

**Migration guide snippet for CHANGELOG:**

```
### Breaking — URL migration

Every management endpoint moved under `/{org_slug}/`. Replace:

    GET /api/v1/keys → GET /api/v1/{org_slug}/keys

`{org_slug}` for existing data is `default`. Auth (`/api/v1/auth/*`),
account (`/api/v1/me/*`), and org listing (`/api/v1/orgs`) endpoints
are unchanged. Legacy paths return 410 Gone with a pointer to the new
location.
```

**No DB migration.** Plan 2.1 is purely routing + UI. The Phase 1 migration (`20260708000000_saas_orgs.sql`) already provides everything needed.

---

### Task 1: Create middleware module skeleton

**Files:**
- Create: `crates/api/src/middleware.rs`
- Modify: `crates/api/src/lib.rs`

- [ ] **Step 1: Add empty middleware module**

`crates/api/src/middleware.rs`:

```rust
//! Axum middlewares for the management API.
//!
//! Installed in this order on every `/api/v1/{org_slug}/*` route:
//!   1. auth_layer         — verify JWT, inject JwtClaims
//!   2. org_resolve_layer  — slug → Org, inject ResolvedOrg
//!   3. membership_layer   — verify (user, org) ∈ members, inject OrgContext
//!
//! Handlers pull `OrgContext` via `FromRequestParts`; they no longer call
//! `require_auth(&headers, ...)` directly.

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

/// Placeholder — real implementation in Task 2.
pub async fn auth_layer(_req: Request, _next: Next) -> Response {
    unimplemented!("auth_layer — filled in by Task 2")
}
```

- [ ] **Step 2: Re-export from `crates/api/src/lib.rs`**

Add to the existing `mod` block:

```rust
pub mod middleware;
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo build -p llm-gateway-api
```

Expected: clean build. No warnings about the unimplemented body (it's a stub).

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/middleware.rs crates/api/src/lib.rs
git commit -m "feat(api): scaffold middleware module for Phase 2 URL migration"
```

---

### Task 2: Implement `auth_layer`

**Files:**
- Modify: `crates/api/src/middleware.rs`

The current `require_auth(&headers, &state.jwt_secret)?` does the right work; we lift it into a `from_fn_with_state` middleware that injects `JwtClaims` into request extensions so downstream layers and handlers can read it without re-parsing the header.

- [ ] **Step 1: Write failing test**

Append to `crates/api/src/middleware.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::middleware::from_fn_with_state;
    use axum::routing::get;
    use axum::Router;
    use llm_gateway_auth::create_jwt;
    use tower::ServiceExt;

    fn make_state(secret: &str) -> Arc<crate::AppState> {
        // Minimal AppState with just jwt_secret populated.
        // Adjust to match your AppState's real constructor.
        crate::AppState::test_with_secret(secret.to_string())
    }

    #[tokio::test]
    async fn auth_layer_rejects_missing_header() {
        let state = make_state("secret");
        let app = Router::new()
            .route("/ok", get(|| async { "ok" }))
            .layer(from_fn_with_state(state, auth_layer));

        let resp = app
            .oneshot(Request::builder().uri("/ok").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn auth_layer_accepts_valid_token() {
        let state = make_state("secret");
        let app = Router::new()
            .route("/ok", get(|| async { "ok" }))
            .layer(from_fn_with_state(state, auth_layer));

        let token = create_jwt("user-1", "org_default", None, "secret").unwrap();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/ok")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
```

If `AppState::test_with_secret` doesn't exist, write a minimal helper in `crates/api/src/lib.rs` that builds an `AppState` with only `jwt_secret` populated and the rest defaulted. This is a test-only constructor; mark it `#[cfg(test)]` or `pub(crate)`.

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p llm-gateway-api middleware::tests -- --nocapture
```

Expected: FAIL (compile error — `auth_layer` body is `unimplemented!`, or signature mismatch).

- [ ] **Step 3: Implement `auth_layer`**

Replace the stub body in `crates/api/src/middleware.rs`:

```rust
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::Response;
use axum::extract::Request;
use axum::middleware::{from_fn_with_state, Next};
use std::sync::Arc;

use crate::error::ApiError;
use crate::AppState;
use crate::extractors::require_auth;
use llm_gateway_auth::JwtClaims;

/// Verify the bearer JWT and inject `JwtClaims` into request extensions.
///
/// Rejects with 401 Unauthorized on missing/invalid token. Token validation
/// logic is shared with the existing `require_auth` helper — we wrap it so
/// downstream layers (org_resolve, membership) and handlers don't repeat the
/// header-parse + decode work.
pub async fn auth_layer(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}
```

If `ApiError: IntoResponse` isn't implemented, the layer's `Result<Response, ApiError>` return won't satisfy Axum's `Handler` trait. The crate already has `impl IntoResponse for ApiError` in `crates/api/src/error.rs` (the existing handlers return `Result<Json<...>, ApiError>`); reuse it.

- [ ] **Step 4: Run test to verify it passes**

```bash
cargo test -p llm-gateway-api middleware::tests -- --nocapture
```

Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/middleware.rs crates/api/src/lib.rs
git commit -m "feat(api): auth_layer middleware injects JwtClaims"
```

---

### Task 3: Implement `org_resolve_layer`

**Files:**
- Modify: `crates/api/src/middleware.rs`
- Modify: `crates/org/src/types.rs` (add `ResolvedOrg`)

`org_resolve_layer` extracts `{org_slug}` from the matched path (Axum fills `Path<HashMap<String, String>>` after route matching) and looks up the org via `storage.get_org_by_slug()`. 404 if the slug doesn't match any org.

- [ ] **Step 1: Add `ResolvedOrg` type**

`crates/org/src/types.rs`:

```rust
/// Lightweight org reference injected by `org_resolve_layer`.
///
/// Heavier `OrgContext` (with role + group_id) is added later by
/// `membership_layer`. Splitting the two lets `org_resolve_layer` run
/// before the membership check, so 404 (no such org) is distinct from
/// 403 (you're not a member).
#[derive(Debug, Clone)]
pub struct ResolvedOrg {
    pub id: String,
    pub slug: String,
    pub name: String,
}
```

Re-export from `crates/org/src/lib.rs`:

```rust
pub use types::{ResolvedOrg, /* existing exports */};
```

- [ ] **Step 2: Write failing test**

Append to `middleware::tests`:

```rust
use axum::extract::Path;
use std::collections::HashMap;
use llm_gateway_org::ResolvedOrg;

#[tokio::test]
async fn org_resolve_layer_404s_unknown_slug() {
    // Set up state with empty storage — get_org_by_slug returns None.
    let state = make_state_with_storage("secret", /* seed: */ vec![]).await;

    let app = Router::new()
        .route("/{org_slug}/keys", get(|| async { "ok" }))
        .layer(from_fn_with_state(state, org_resolve_layer));

    let token = make_token("user-1", "org_default");
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/ghost-org/keys")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn org_resolve_layer_injects_resolved_org_extension() {
    let state = make_state_with_storage("secret", vec!["default"]).await;

    let app = Router::new()
        .route(
            "/{org_slug}/probe",
            get(|req: Request| async move {
                let org = req.extensions().get::<ResolvedOrg>().unwrap();
                format!("{}/{}", org.id, org.slug)
            }),
        )
        .layer(from_fn_with_state(state, org_resolve_layer));

    let token = make_token("user-1", "org_default");
    let body = axum::body::to_bytes(
        app.oneshot(
            Request::builder()
                .uri("/default/probe")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .into_body(),
        usize::MAX,
    )
    .await
    .unwrap();
    assert_eq!(&body[..], b"org_default/default");
}
```

The `make_state_with_storage` helper builds an `AppState` with an in-memory or test-postgres storage seeded with the named orgs. If you already have a `#[sqlx::test]` pattern in `crates/api/tests/`, use it here instead.

- [ ] **Step 3: Run tests — expect FAIL**

```bash
cargo test -p llm-gateway-api middleware::tests -- --nocapture
```

Expected: FAIL (`org_resolve_layer` doesn't exist yet).

- [ ] **Step 4: Implement `org_resolve_layer`**

Append to `crates/api/src/middleware.rs`:

```rust
use axum::extract::Path;
use std::collections::HashMap;
use llm_gateway_org::ResolvedOrg;

/// Look up the org named by `{org_slug}` and inject `ResolvedOrg`.
///
/// Path parameters are populated by Axum's router after matching; this layer
/// expects `org_slug` to be present. 404 if no org has that slug.
pub async fn org_resolve_layer(
    State(state): State<Arc<AppState>>,
    Path(params): Path<HashMap<String, String>>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let slug = params
        .get("org_slug")
        .ok_or_else(|| ApiError::BadRequest("missing org_slug path param".into()))?;

    let org = state
        .storage
        .get_org_by_slug(slug)
        .await
        .map_err(|_| ApiError::Internal("storage error during org lookup".into()))?
        .ok_or(ApiError::NotFound)?;

    req.extensions_mut().insert(ResolvedOrg {
        id: org.id,
        slug: org.slug,
        name: org.name,
    });

    Ok(next.run(req).await)
}
```

`ApiError::NotFound` and `ApiError::BadRequest` should already exist; if `BadRequest` doesn't, add it as a 400 variant.

- [ ] **Step 5: Run tests — expect PASS**

```bash
cargo test -p llm-gateway-api middleware::tests -- --nocapture
```

Expected: 4 tests pass (2 from Task 2 + 2 new).

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/middleware.rs crates/org/src/types.rs crates/org/src/lib.rs
git commit -m "feat(api,org): org_resolve_layer injects ResolvedOrg"
```

---

### Task 4: Implement `membership_layer`

**Files:**
- Modify: `crates/api/src/middleware.rs`
- Modify: `crates/org/src/types.rs` (add `OrgContext` if not already there from Phase 1)

Phase 1 already added `OrgContext` to `crates/org/src/types.rs` with `org_id`, `member_role`, `platform_role`, `group_id`. The membership layer constructs one from `JwtClaims` + `ResolvedOrg` + a `members` row lookup.

Plan 2.3 will add platform-admin impersonation (temp member rows). For Plan 2.1, platform_admins who aren't members just get a 403 — same as everyone else.

- [ ] **Step 1: Write failing test**

Append to `middleware::tests`:

```rust
#[tokio::test]
async fn membership_layer_403s_non_member() {
    // User is a member of "default" but not "other".
    let state = make_state_with_members("secret",
        vec![("user-1", "org_default", MemberRole::Member)]).await;

    let app = Router::new()
        .route("/{org_slug}/keys", get(|| async { "ok" }))
        .layer(from_fn_with_state(state.clone(), membership_layer));

    let token = make_token("user-1", "org_default");
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/other/keys")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn membership_layer_injects_org_context_for_member() {
    let state = make_state_with_members("secret",
        vec![("user-1", "org_default", MemberRole::Admin)]).await;

    let app = Router::new()
        .route(
            "/{org_slug}/probe",
            get(|req: Request| async move {
                let ctx = req.extensions().get::<OrgContext>().unwrap();
                format!("{}:{:?}", ctx.org_id, ctx.member_role)
            }),
        )
        .layer(from_fn_with_state(state, membership_layer));

    let token = make_token("user-1", "org_default");
    let body = axum::body::to_bytes(
        app.oneshot(
            Request::builder()
                .uri("/default/probe")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .into_body(),
        usize::MAX,
    )
    .await
    .unwrap();
    assert_eq!(&body[..], b"org_default:Admin");
}
```

`OrgContext` import:

```rust
use llm_gateway_org::OrgContext;
use llm_gateway_storage::MemberRole;
```

- [ ] **Step 2: Run tests — expect FAIL**

```bash
cargo test -p llm-gateway-api middleware::tests -- --nocapture
```

Expected: FAIL (`membership_layer` undefined).

- [ ] **Step 3: Implement `membership_layer`**

Append to `crates/api/src/middleware.rs`:

```rust
use llm_gateway_auth::JwtClaims;
use llm_gateway_org::{OrgContext, ResolvedOrg};
use llm_gateway_storage::MemberRole;

/// Verify the user is a member of the resolved org, then inject `OrgContext`.
///
/// Reads `JwtClaims` (set by `auth_layer`) and `ResolvedOrg` (set by
/// `org_resolve_layer`) from extensions. 403 if no membership row exists.
///
/// Plan 2.3 will add platform-admin impersonation here: if the JWT has
/// `platform_role = platform_admin` and no member row exists, create a
/// temp `members` row (role=admin, created_by='system'). For Plan 2.1,
/// platform_admins get the same 403 as anyone else when they're not a
/// member — impersonation is a separate concern.
pub async fn membership_layer(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let claims = req
        .extensions()
        .get::<JwtClaims>()
        .cloned()
        .ok_or_else(|| ApiError::Internal("auth_layer did not run".into()))?;

    let org = req
        .extensions()
        .get::<ResolvedOrg>()
        .cloned()
        .ok_or_else(|| ApiError::Internal("org_resolve_layer did not run".into()))?;

    let member = state
        .storage
        .get_member(&claims.sub, &org.id)
        .await
        .map_err(|_| ApiError::Internal("storage error during membership check".into()))?
        .ok_or(ApiError::Forbidden)?;

    let ctx = OrgContext {
        org_id: org.id.clone(),
        member_role: member.role,
        platform_role: claims.platform_role.as_deref().map(|_| PlatformRole::PlatformAdmin),
        group_id: member.group_id,
    };
    req.extensions_mut().insert(ctx);

    Ok(next.run(req).await)
}
```

`JwtClaims` needs to be `Clone` for `.cloned()` here. If it isn't, derive `Clone` on it in `crates/auth/src/lib.rs`.

`PlatformRole` import:

```rust
use llm_gateway_storage::PlatformRole;
```

- [ ] **Step 4: Run tests — expect PASS**

```bash
cargo test -p llm-gateway-api middleware::tests -- --nocapture
```

Expected: 6 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/middleware.rs
git commit -m "feat(api): membership_layer injects OrgContext (no platform-admin yet)"
```

---

### Task 5: Make `OrgContext` an Axum extractor

**Files:**
- Modify: `crates/org/src/extractors.rs`

Handlers want `async fn list_keys(ctx: OrgContext, ...)`. Axum lets us add this by implementing `FromRequestParts`.

- [ ] **Step 1: Write failing test**

`crates/org/src/extractors.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::RequestParts;
    use axum::http::Request;
    use crate::{MemberRole, PlatformRole};

    #[tokio::test]
    async fn extracts_org_context_from_extensions() {
        let ctx = OrgContext {
            org_id: "org_default".into(),
            member_role: MemberRole::Admin,
            platform_role: None,
            group_id: None,
        };
        let mut req: Request<()> = Request::default();
        req.extensions_mut().insert(ctx.clone());

        let mut parts = RequestParts::new(req);
        let extracted = OrgContext::from_request_parts(&mut parts)
            .await
            .expect("OrgContext should be in extensions");

        assert_eq!(extracted.org_id, ctx.org_id);
        assert!(matches!(extracted.member_role, MemberRole::Admin));
    }
}
```

- [ ] **Step 2: Run test — expect FAIL**

```bash
cargo test -p llm-gateway-org extractors::tests -- --nocapture
```

Expected: FAIL (no `FromRequestParts` impl).

- [ ] **Step 3: Implement `FromRequestParts`**

`crates/org/src/extractors.rs` (replace Phase 1 stub):

```rust
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::response::IntoResponse;
use http::StatusCode;

use crate::OrgContext;

impl<S> FromRequestParts<S> for OrgContext
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<OrgContext>()
            .cloned()
            .ok_or_else(|| {
                (StatusCode::INTERNAL_SERVER_ERROR, "OrgContext missing").into_response()
            })
    }
}
```

This requires `OrgContext: Clone` — derive it in `crates/org/src/types.rs` if not already present.

Add `axum` (and `http`) to `crates/org/Cargo.toml`:

```toml
[dependencies]
axum = { workspace = true }
http = { workspace = true }
# ... existing deps
```

- [ ] **Step 4: Run test — expect PASS**

```bash
cargo test -p llm-gateway-org extractors::tests -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/org/src/extractors.rs crates/org/Cargo.toml crates/org/src/types.rs
git commit -m "feat(org): OrgContext implements FromRequestParts"
```

---

### Task 6: Restructure `management_router` with `/{org_slug}/` prefix and middleware chain

**Files:**
- Modify: `crates/api/src/management/mod.rs`

This is the big mechanical task. Split `management_router()` into two builders:

- **Global routes** (`/api/v1/auth/*`, `/api/v1/me/*`, `/api/v1/orgs`, `/api/v1/version`, `/api/v1/admin/system-info`, `/api/v1/admin/nats/*`) — no org context, only `auth_layer`.
- **Org-scoped routes** (`/api/v1/{org_slug}/*`) — full middleware chain.

- [ ] **Step 1: Rewrite `management_router()`**

Replace the existing function body in `crates/api/src/management/mod.rs`:

```rust
use axum::middleware::from_fn_with_state;
use crate::middleware::{auth_layer, membership_layer, org_resolve_layer};

pub fn management_router() -> Router<Arc<AppState>> {
    let org_scoped = org_scoped_routes()
        .layer(from_fn_with_state(/* state */, membership_layer))
        .layer(from_fn_with_state(/* state */, org_resolve_layer))
        .layer(from_fn_with_state(/* state */, auth_layer));

    Router::new()
        // Global — auth only
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/config", get(auth::auth_config))
        .route("/api/v1/auth/me", get(auth::me))
        .route("/api/v1/auth/me/balance", get(auth::me_balance))
        .route("/api/v1/auth/refresh", post(auth::refresh))
        .route("/api/v1/auth/change-password", post(auth::change_password))
        .route("/api/v1/me/current-org", post(auth::switch_org))
        .route("/api/v1/orgs", get(auth::list_orgs).post(orgs::create_org))
        .route("/api/v1/version", get(version))
        .route("/api/v1/admin/system-info", get(system_info))
        .route("/api/v1/admin/nats/status", get(nats::get_nats_status))
        // Legacy catch-all — 410 Gone (Task 7 fills this in)
        .route("/api/v1/{*legacy}", get(legacy_gone))
        // Org-scoped — full chain
        .nest("/api/v1/{org_slug}", org_scoped)
}

fn org_scoped_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(orgs::get_org))
        // Keys (authenticated)
        .route("/keys", post(keys::create_key).get(keys::list_keys))
        .route("/keys/{id}", get(keys::get_key).patch(keys::update_key).delete(keys::delete_key))
        // Model Fallbacks
        .route("/model-fallbacks", post(model_fallbacks::create_model_fallback).get(model_fallbacks::list_model_fallbacks))
        .route("/model-fallbacks/{id}", get(model_fallbacks::get_model_fallback).patch(model_fallbacks::update_model_fallback).delete(model_fallbacks::delete_model_fallback))
        // Providers (admin)
        .route("/admin/providers", post(providers::create_provider).get(providers::list_providers))
        .route("/admin/providers/{id}", get(providers::get_provider).patch(providers::update_provider).delete(providers::delete_provider))
        .route("/admin/providers/{id}/channels", post(channels::create_channel).get(channels::list_channels))
        .route("/admin/providers/{id}/models", get(models::list_provider_models).put(models::update_provider_models))
        .route("/admin/channels", post(channels::create_channel).get(channels::list_all_channels))
        .route("/admin/channels/{id}", get(channels::get_channel).patch(channels::update_channel).delete(channels::delete_channel))
        .route("/admin/channels/{id}/api-key", patch(channels::update_channel_api_key))
        .route("/admin/channels/{id}/test", post(channels::test_channel))
        // Groups (admin)
        .route("/admin/groups", post(groups::create_group).get(groups::list_groups))
        .route("/admin/groups/{id}", get(groups::get_group).patch(groups::update_group).delete(groups::delete_group))
        .route("/admin/models", get(models::list_all_models).post(models::create_model_global))
        .route("/admin/models/{model_name}", patch(models::update_model).delete(models::delete_model))
        // ChannelModels (admin)
        .route("/admin/providers/{provider_id}/channel-models", post(channel_models::create_channel_model).get(channel_models::list_channel_models))
        .route("/admin/channels/{channel_id}/channel-models", get(channel_models::list_channel_models_by_channel))
        .route("/admin/channels/{channel_id}/channel-models", post(channel_models::create_channel_model_by_channel))
        .route("/admin/channel-models/{id}", get(channel_models::get_channel_model).patch(channel_models::update_channel_model).delete(channel_models::delete_channel_model))
        // Usage (authenticated)
        .route("/usage", get(usage::get_usage))
        .route("/usage/summary", get(usage::get_usage_summary))
        .route("/usage/channel-summary", get(usage::get_channel_usage_summary))
        .route("/usage/daily", get(usage::get_daily_usage))
        // Logs (admin)
        .route("/admin/logs", get(logs::get_logs))
        .route("/admin/logs/{id}", get(logs::get_log))
        // Request details (admin)
        .route("/admin/requests/{request_id}", get(requests::get_request_details))
        // Users (admin)
        .route("/admin/users", get(users::list_users))
        .route("/admin/users/{id}", patch(users::update_user).delete(users::delete_user))
        // Account / Balance (admin)
        .route("/admin/users/{id}/balance", get(accounts::get_balance))
        .route("/admin/users/{id}/recharge", post(accounts::recharge))
        .route("/admin/users/{id}/adjust", post(accounts::adjust))
        .route("/admin/users/{id}/threshold", patch(accounts::update_threshold))
        // Settings (admin)
        .route("/admin/settings", get(settings::get_settings).patch(settings::update_settings))
        // Seed data (reads static JSON)
        .route("/admin/seed", get(seed::get_seed_data))
        // Pricing Policies (admin)
        .route("/admin/pricing-policies", post(pricing_policies::create).get(pricing_policies::list))
        .route("/admin/pricing-policies/{id}", get(pricing_policies::get).patch(pricing_policies::update).delete(pricing_policies::delete))
}
```

Notes for the implementer:

- The `from_fn_with_state` calls need the `Arc<AppState>` to be available at router-build time. Axum 0.7's pattern is `Router::with_state(state)` at the outermost level — pass state through `Router::new().with_state(...)` rather than per-layer if your AppState is `Clone`. Check `crates/gateway/src/main.rs` for the existing pattern; if `management_router()` is called without state today, change the signature to `pub fn management_router(state: Arc<AppState>) -> Router`.
- The `{org_slug}` capture name must match what `org_resolve_layer` reads from `Path<HashMap>`.
- The `{*legacy}` catch-all must come AFTER all legitimate global routes but BEFORE the nested org-scoped routes. Test ordering carefully.

- [ ] **Step 2: Verify it compiles**

```bash
cargo build -p llm-gateway-api
```

Expected: many errors (every handler still expects `headers: HeaderMap` and `claims: JwtClaims`). These are fixed in Task 7. For now, just confirm the router structure compiles by stubbing the handlers temporarily if needed.

If the router won't compile due to handler signature mismatches, do Task 7 first for one handler (e.g., `keys::list_keys`) to verify the pattern, then return to this step.

- [ ] **Step 3: Commit**

```bash
git add crates/api/src/management/mod.rs
git commit -m "feat(api): nest management routes under /{org_slug}/* with middleware chain"
```

---

### Task 7: Update handlers to use `OrgContext` extractor

**Files:**
- Modify: `crates/api/src/management/keys.rs`
- Modify: `crates/api/src/management/channels.rs`
- Modify: `crates/api/src/management/channel_models.rs`
- Modify: `crates/api/src/management/providers.rs`
- Modify: `crates/api/src/management/models.rs`
- Modify: `crates/api/src/management/model_fallbacks.rs`
- Modify: `crates/api/src/management/groups.rs`
- Modify: `crates/api/src/management/usage.rs`
- Modify: `crates/api/src/management/logs.rs`
- Modify: `crates/api/src/management/requests.rs`
- Modify: `crates/api/src/management/users.rs`
- Modify: `crates/api/src/management/accounts.rs`
- Modify: `crates/api/src/management/settings.rs`
- Modify: `crates/api/src/management/seed.rs`
- Modify: `crates/api/src/management/pricing_policies.rs`

Each handler currently looks like:

```rust
pub async fn list_keys(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<KeySummary>>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;
    let org_id = &claims.current_org_id;
    let keys = state.storage.list_keys(org_id).await?;
    Ok(Json(keys.into_iter().map(Into::into).collect()))
}
```

The new shape:

```rust
pub async fn list_keys(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
) -> Result<Json<Vec<KeySummary>>, ApiError> {
    let keys = state.storage.list_keys(&ctx.org_id).await?;
    Ok(Json(keys.into_iter().map(Into::into).collect()))
}
```

Same change applies to admin endpoints that previously called `require_platform_admin`:

```rust
// Before:
pub async fn create_provider(...) -> Result<..., ApiError> {
    let claims = require_platform_admin(&headers, &state.jwt_secret)?;
    ...
}

// After:
pub async fn create_provider(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
) -> Result<..., ApiError> {
    if !matches!(ctx.member_role, MemberRole::Owner | MemberRole::Admin)
        && !ctx.is_platform_admin()
    {
        return Err(ApiError::Forbidden);
    }
    ...
}
```

- [ ] **Step 1: Pick one handler as the canonical migration (`keys::list_keys`)**

Apply the change above to `crates/api/src/management/keys.rs::list_keys`. Update existing tests in `crates/api/tests/` that call this handler to construct an `OrgContext` directly rather than a JWT-bearing `HeaderMap`.

- [ ] **Step 2: Run keys tests**

```bash
cargo test -p llm-gateway-api keys -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Migrate remaining handlers mechanically**

Apply the same pattern to every handler in every file listed above. The transformations:

| Before | After |
|---|---|
| `headers: HeaderMap` parameter | (remove) |
| `let claims = require_auth(&headers, &state.jwt_secret)?;` | (remove) |
| `let claims = require_platform_admin(&headers, &state.jwt_secret)?;` | `if !can_administer(&ctx) { return Err(ApiError::Forbidden); }` |
| `claims.current_org_id` | `ctx.org_id.clone()` or `&ctx.org_id` |
| `claims.sub` | `ctx.user_id` (add `user_id: String` field to `OrgContext`) |

Add a helper for the admin check. `crates/org/src/access.rs`:

```rust
pub fn can_administer(ctx: &OrgContext) -> bool {
    matches!(ctx.member_role, MemberRole::Owner | MemberRole::Admin)
        || ctx.is_platform_admin()
}
```

`OrgContext` needs `user_id` (currently only in `JwtClaims`). Add it:

`crates/org/src/types.rs`:

```rust
#[derive(Debug, Clone)]
pub struct OrgContext {
    pub user_id: String,        // NEW — was only in JwtClaims
    pub org_id: String,
    pub member_role: MemberRole,
    pub platform_role: Option<PlatformRole>,
    pub group_id: Option<String>,
}
```

Update `membership_layer` to set it:

```rust
let ctx = OrgContext {
    user_id: claims.sub.clone(),
    org_id: org.id.clone(),
    member_role: member.role,
    platform_role: claims.platform_role.as_deref().map(|_| PlatformRole::PlatformAdmin),
    group_id: member.group_id,
};
```

- [ ] **Step 4: Build the workspace**

```bash
cargo build --workspace
```

Expected: clean build. If any handler in `gateway/`, `usage-worker/`, or `audit-worker/` calls Storage methods that changed signature in Phase 1, those were already fixed — Phase 2.1 doesn't change Storage.

- [ ] **Step 5: Run full test suite**

```bash
cargo test --workspace
```

Expected: PASS (with fixture updates for tests that constructed `HeaderMap` directly).

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/management/ crates/org/src/
git commit -m "refactor(api): management handlers read OrgContext via FromRequestParts"
```

---

### Task 8: Implement `legacy_gone` 410 handler

**Files:**
- Modify: `crates/api/src/management/mod.rs`

The router's `/api/v1/{*legacy}` catch-all (registered in Task 6) calls this handler for any `/api/v1/X` path that isn't a global route and isn't under `/{org_slug}/`. Returns 410 Gone with a JSON body pointing at the new location.

- [ ] **Step 1: Write failing test**

Append to a tests module in `crates/api/src/management/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn legacy_keys_path_returns_410() {
        let app = Router::new()
            .route("/api/v1/{*legacy}", get(legacy_gone))
            .with_state(test_state().await);

        let resp = app
            .oneshot(Request::builder().uri("/api/v1/keys").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::GONE);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"], "gone");
        assert_eq!(body["new_path"], "/api/v1/{org_slug}/keys");
    }
}
```

- [ ] **Step 2: Run test — expect FAIL**

```bash
cargo test -p llm-gateway-api management::tests -- --nocapture
```

- [ ] **Step 3: Implement `legacy_gone`**

```rust
use axum::extract::Path;

/// 410 Gone for pre-Phase-2 routes. Returns a JSON body describing the new path.
///
/// The `*legacy` capture is the part after `/api/v1/`. We don't try to be
/// clever about mapping — every old path gets the same shape of response
/// since the migration is mechanical: insert `{org_slug}/` after `/api/v1/`.
async fn legacy_gone(Path(legacy): Path<String>) -> Json<serde_json::Value> {
    let new_path = format!("/api/v1/{{org_slug}}/{legacy}");
    Json(serde_json::json!({
        "error": "gone",
        "message": "This endpoint moved in v2.1.0. Add your org slug after /api/v1/.",
        "new_path": new_path,
        "doc_url": "https://github.com/<owner>/llm-gateway/blob/main/CHANGELOG.md#v210"
    }))
}
```

Wrap in `(StatusCode::GONE, Json(...))` if you want the explicit status tuple rather than relying on a custom `IntoResponse`.

- [ ] **Step 4: Run test — expect PASS**

```bash
cargo test -p llm-gateway-api management::tests -- --nocapture
```

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/management/mod.rs
git commit -m "feat(api): legacy /api/v1/X paths return 410 Gone with new path"
```

---

### Task 9: Implement `POST /api/v1/orgs` (create-org)

**Files:**
- Create: `crates/api/src/management/orgs.rs`
- Modify: `crates/api/src/management/mod.rs` (register route — already done in Task 6 if you used `orgs::create_org`)

The create-org endpoint takes a slug + name, creates the org, makes the caller the owner, and returns an `OrgSummary`. Slug charset and uniqueness are enforced at the DB layer (`^[a-z0-9-]{3,64}$` + `UNIQUE`).

- [ ] **Step 1: Write failing test**

`crates/api/tests/test_orgs.rs`:

```rust
use crate::common::{make_client, make_user_token};

#[tokio::test]
#[sqlx::test(fixtures("users_seed"))]
async fn create_org_makes_caller_owner(pool: sqlx::PgPool) {
    let app = make_client(pool).await;
    let token = make_user_token("user-1", /* current_org_id */ "org_default");

    let resp = app
        .post("/api/v1/orgs")
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "slug": "acme-inc",
            "name": "Acme Inc",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["slug"], "acme-inc");
    assert_eq!(body["name"], "Acme Inc");
    assert_eq!(body["role"], "owner");

    // Caller is now a member with role=owner
    let memberships = app.list_orgs_for_user("user-1").await;
    assert!(memberships.iter().any(|m| m.org.slug == "acme-inc" && m.role == "owner"));
}

#[tokio::test]
#[sqlx::test(fixtures("users_seed"))]
async fn create_org_rejects_invalid_slug() {
    let app = make_client(pool).await;
    let token = make_user_token("user-1", "org_default");

    let resp = app
        .post("/api/v1/orgs")
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "slug": "ACME INC!!",  // bad charset
            "name": "Acme",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
}

#[tokio::test]
#[sqlx::test(fixtures("users_seed", "default_org_seed"))]
async fn create_org_rejects_duplicate_slug() {
    let app = make_client(pool).await;
    let token = make_user_token("user-1", "org_default");

    let resp = app
        .post("/api/v1/orgs")
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "slug": "default",  // already exists
            "name": "Default",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 409);
}
```

- [ ] **Step 2: Run tests — expect FAIL**

```bash
cargo test -p llm-gateway-api --test test_orgs -- --nocapture
```

- [ ] **Step 3: Implement `create_org` handler**

`crates/api/src/management/orgs.rs` (new file):

```rust
use axum::extract::State;
use axum::http::StatusCode;
use axum::{Json, response::IntoResponse};
use regex::Regex;
use std::sync::{Arc, LazyLock};

use crate::error::ApiError;
use crate::AppState;
use llm_gateway_auth::JwtClaims;
use llm_gateway_storage::{CreateOrg, MemberRole, Member};

static SLUG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-z0-9-]{3,64}$").expect("valid regex")
});

#[derive(serde::Deserialize)]
pub struct CreateOrgRequest {
    pub slug: String,
    pub name: String,
}

#[derive(serde::Serialize)]
pub struct OrgSummary {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub role: String,
}

pub async fn create_org(
    State(state): State<Arc<AppState>>,
    claims: JwtClaims,  // extract via FromRequestParts (already impl'd in auth crate)
    Json(req): Json<CreateOrgRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if !SLUG_RE.is_match(&req.slug) {
        return Err(ApiError::BadRequest("slug must match ^[a-z0-9-]{3,64}$".into()));
    }
    if req.name.trim().is_empty() {
        return Err(ApiError::BadRequest("name must not be empty".into()));
    }

    let org = state
        .storage
        .create_org(CreateOrg {
            slug: req.slug.clone(),
            name: req.name.clone(),
            owner_id: Some(claims.sub.clone()),
        })
        .await
        .map_err(|e| {
            if e.to_string().contains("unique constraint") || e.to_string().contains("duplicate key") {
                ApiError::Conflict
            } else {
                ApiError::Internal(e.to_string())
            }
        })?;

    state
        .storage
        .upsert_member(Member {
            user_id: claims.sub.clone(),
            org_id: org.id.clone(),
            role: MemberRole::Owner,
            group_id: None,
            created_by: Some(claims.sub.clone()),
            created_at: chrono::Utc::now(),
        })
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(OrgSummary {
            id: org.id,
            slug: org.slug,
            name: org.name,
            role: "owner".into(),
        }),
    ))
}

/// GET /api/v1/{org_slug} — read org details. Caller must be a member.
pub async fn get_org(
    State(state): State<Arc<AppState>>,
    ctx: llm_gateway_org::OrgContext,
) -> Result<Json<OrgSummary>, ApiError> {
    let org = state
        .storage
        .get_org(&ctx.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound)?;

    Ok(Json(OrgSummary {
        id: org.id,
        slug: org.slug,
        name: org.name,
        role: format!("{:?}", ctx.member_role).to_lowercase(),
    }))
}
```

Notes:
- `ApiError::Conflict` (409) and `ApiError::BadRequest` (400) variants must exist. Add them if missing.
- `JwtClaims` needs `FromRequestParts`. If it doesn't impl it, add a thin extractor `AuthenticatedUser(JwtClaims)` in `crates/api/src/extractors.rs` and use that here instead.
- The error-to-status mapping for "slug already taken" relies on string-matching the storage error. If `StorageError` has a typed variant for unique-violation, use that instead.

- [ ] **Step 4: Register the route**

Already done in Task 6's router rewrite:

```rust
.route("/api/v1/orgs", get(auth::list_orgs).post(orgs::create_org))
```

If `auth::list_orgs` doesn't exist (Phase 1 spec mentioned it but may not have been implemented), move it here or leave it in `auth.rs`. Either is fine.

- [ ] **Step 5: Run tests — expect PASS**

```bash
cargo test -p llm-gateway-api --test test_orgs -- --nocapture
```

Expected: 3 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/management/orgs.rs crates/api/src/management/mod.rs
git commit -m "feat(api): POST /api/v1/orgs create-org; GET /{org_slug} read-org"
```

---

### Task 10: Frontend — Add `orgPrefix()` helper to API client

**Files:**
- Modify: `web/src/api/client.ts`

Every org-scoped endpoint prepends `/api/v1/${currentOrg.slug}`. Centralize this in one helper so a future org switch updates every call automatically.

- [ ] **Step 1: Write failing test**

`web/src/api/client.test.ts`:

```typescript
import { describe, it, expect, beforeEach } from 'vitest'
import { orgPrefix } from './client'
import { useAuthStore } from '../stores/authStore'

describe('orgPrefix', () => {
  beforeEach(() => {
    useAuthStore.setState({
      currentOrg: null,
      user: null,
      token: null,
    })
  })

  it('throws when no current org is set', () => {
    expect(() => orgPrefix()).toThrow(/no current org/i)
  })

  it('returns the prefixed path when currentOrg is set', () => {
    useAuthStore.setState({
      currentOrg: { id: 'org-1', slug: 'acme', name: 'Acme', role: 'admin', group_id: null },
    })
    expect(orgPrefix()).toBe('/api/v1/acme')
  })
})
```

- [ ] **Step 2: Run test — expect FAIL**

```bash
source ~/.nvm/nvm.sh && cd web && npm test -- src/api/client.test.ts
```

- [ ] **Step 3: Implement `orgPrefix()`**

`web/src/api/client.ts`:

```typescript
import { useAuthStore } from '../stores/authStore'

/**
 * Returns `/api/v1/${currentOrg.slug}` for org-scoped endpoints.
 *
 * @throws if no current org is set — callers must ensure the user is
 *   authenticated and has selected an org before invoking. OrgRouteGuard
 *   guarantees this at the route level.
 */
export function orgPrefix(): string {
  const slug = useAuthStore.getState().currentOrg?.slug
  if (!slug) throw new Error('no current org — cannot build org-scoped URL')
  return `/api/v1/${slug}`
}
```

- [ ] **Step 4: Run test — expect PASS**

```bash
npm test -- src/api/client.test.ts
```

- [ ] **Step 5: Commit**

```bash
git add web/src/api/client.ts web/src/api/client.test.ts
git commit -m "feat(web): orgPrefix() helper for org-scoped API paths"
```

---

### Task 11: Frontend — Update all API endpoint modules to use `orgPrefix()`

**Files:**
- Modify: `web/src/api/keys.ts`
- Modify: `web/src/api/channels.ts`
- Modify: `web/src/api/channel-models.ts`
- Modify: `web/src/api/providers.ts`
- Modify: `web/src/api/models.ts`
- Modify: `web/src/api/model-fallbacks.ts`
- Modify: `web/src/api/groups.ts`
- Modify: `web/src/api/usage.ts`
- Modify: `web/src/api/logs.ts`
- Modify: `web/src/api/users.ts`
- Modify: `web/src/api/accounts.ts`
- Modify: `web/src/api/settings.ts`
- Modify: `web/src/api/seed.ts`
- Modify: `web/src/api/pricing-policies.ts`

Mechanical change. Every function currently shaped like:

```typescript
export async function listKeys(): Promise<Key[]> {
  const { data } = await api.get('/api/v1/keys')
  return data
}
```

becomes:

```typescript
import { orgPrefix } from './client'

export async function listKeys(): Promise<Key[]> {
  const { data } = await api.get(`${orgPrefix()}/keys`)
  return data
}
```

- [ ] **Step 1: Pick one module (`keys.ts`) and migrate every function**

For each `api.get/post/patch/delete('/api/v1/...')` call:
- If the path is `/api/v1/auth/*`, `/api/v1/me/*`, `/api/v1/orgs`, or `/api/v1/version` → leave unchanged (global endpoint).
- Otherwise → replace `/api/v1` prefix with `${orgPrefix()}`.

- [ ] **Step 2: Verify keys migration builds**

```bash
source ~/.nvm/nvm.sh && cd web && npm run build
```

Expected: TypeScript check passes.

- [ ] **Step 3: Migrate remaining 13 modules**

Apply the same transformation. Run `npm run build` after every 2-3 modules to catch typos early.

- [ ] **Step 4: Run frontend tests**

```bash
npm test
```

Expected: PASS. Any test that mocks `/api/v1/keys` needs updating to mock `${orgPrefix()}/keys` — set up the test's authStore with a known currentOrg and assert against the prefixed URL.

- [ ] **Step 5: Commit**

```bash
git add web/src/api/
git commit -m "refactor(web): all org-scoped API calls use orgPrefix()"
```

---

### Task 12: Frontend — Prefix React Query keys with `currentOrg.slug`

**Files:**
- Modify: `web/src/hooks/useKeys.ts`
- Modify: `web/src/hooks/useChannels.ts`
- Modify: every `web/src/hooks/use*.ts`

Stale cross-org data is the highest-risk bug in this migration. If a user switches from Org A to Org B and React Query still has cached results for Org A's keys list, the UI will render Org A's data until the new query resolves. Prefixing keys with the org slug makes the cache org-scoped — switching orgs becomes a cache miss by construction.

- [ ] **Step 1: Write a failing test**

`web/src/hooks/useKeys.test.tsx`:

```tsx
import { describe, it, expect } from 'vitest'
import { renderHook } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useKeys } from './useKeys'
import { useAuthStore } from '../stores/authStore'

describe('useKeys', () => {
  it('uses the current org slug as the first queryKey element', () => {
    useAuthStore.setState({
      currentOrg: { id: 'org-1', slug: 'acme', name: 'Acme', role: 'admin', group_id: null },
    })

    const qc = new QueryClient()
    const wrapper = ({ children }) => (
      <QueryClientProvider client={qc}>{children}</QueryClientProvider>
    )

    const { result } = renderHook(() => useKeys(), { wrapper })
    expect(result.current.queryKey).toEqual(['acme', 'keys'])
  })
})
```

- [ ] **Step 2: Run test — expect FAIL**

```bash
npm test -- src/hooks/useKeys.test.tsx
```

- [ ] **Step 3: Update `useKeys`**

```typescript
// web/src/hooks/useKeys.ts
import { useQuery } from '@tanstack/react-query'
import { useAuthStore } from '../stores/authStore'
import { listKeys } from '../api/keys'

export function useKeys() {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? ''
  return useQuery({
    queryKey: [slug, 'keys'],
    queryFn: listKeys,
    enabled: !!slug,
  })
}
```

The `enabled: !!slug` guards against the brief window where `currentOrg` hasn't been set yet.

- [ ] **Step 4: Migrate remaining hooks mechanically**

Pattern:

```typescript
const slug = useAuthStore((s) => s.currentOrg?.slug) ?? ''
useQuery({
  queryKey: [slug, '<resource>'],
  // ...
})
```

For mutation hooks (`useCreateKey`, etc.), the mutation's `onSuccess` should call `queryClient.invalidateQueries({ queryKey: [slug, 'keys'] })`.

- [ ] **Step 5: Run frontend tests + build**

```bash
npm test && npm run build
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add web/src/hooks/
git commit -m "refactor(web): prefix React Query keys with current org slug"
```

---

### Task 13: Frontend — Implement `OrgSwitcher` component

**Files:**
- Create: `web/src/components/OrgSwitcher.tsx`
- Modify: `web/src/stores/authStore.ts` (add `setCurrentOrg` action if not present from Phase 1)

The OrgSwitcher is a dropdown in the sidebar header showing the current org name + a chevron. Clicking it opens a list of the user's orgs. Selecting one calls `setCurrentOrg`, clears the React Query cache (prevents stale cross-org data), and navigates to `/${newSlug}/dashboard`.

- [ ] **Step 1: Write failing test**

`web/src/components/OrgSwitcher.test.tsx`:

```tsx
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { MemoryRouter, useNavigate } from 'react-router-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { OrgSwitcher } from './OrgSwitcher'
import { useAuthStore } from '../stores/authStore'

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom')
  return { ...actual, useNavigate: vi.fn() }
})

describe('OrgSwitcher', () => {
  beforeEach(() => {
    useAuthStore.setState({
      currentOrg: { id: 'org-1', slug: 'acme', name: 'Acme', role: 'admin', group_id: null },
      orgs: [
        { id: 'org-1', slug: 'acme', name: 'Acme', role: 'admin', group_id: null },
        { id: 'org-2', slug: 'personal', name: 'Personal', role: 'owner', group_id: null },
      ],
    })
  })

  it('shows the current org name', () => {
    render(<OrgSwitcher />, { wrapper: MemoryRouter })
    expect(screen.getByText('Acme')).toBeInTheDocument()
  })

  it('switches org on click — calls setCurrentOrg, clears cache, navigates', async () => {
    const mockSetCurrentOrg = vi.fn().mockResolvedValue(undefined)
    const mockNavigate = vi.fn()
    vi.mocked(useNavigate).mockReturnValue(mockNavigate)
    useAuthStore.setState({ setCurrentOrg: mockSetCurrentOrg })

    const qc = new QueryClient()
    const clearSpy = vi.spyOn(qc, 'clear')
    const wrapper = ({ children }) => (
      <QueryClientProvider client={qc}>
        <MemoryRouter>{children}</MemoryRouter>
      </QueryClientProvider>
    )
    render(<OrgSwitcher />, { wrapper })

    fireEvent.click(screen.getByText('Acme'))
    fireEvent.click(screen.getByText('Personal'))

    await waitFor(() => {
      expect(mockSetCurrentOrg).toHaveBeenCalledWith(expect.objectContaining({ slug: 'personal' }))
      expect(clearSpy).toHaveBeenCalled()
      expect(mockNavigate).toHaveBeenCalledWith('/personal/dashboard')
    })
  })
})
```

- [ ] **Step 2: Run test — expect FAIL**

```bash
npm test -- src/components/OrgSwitcher.test.tsx
```

- [ ] **Step 3: Add `setCurrentOrg` action to authStore**

`web/src/stores/authStore.ts` (Phase 1 left this as a stub):

```typescript
import { queryClient } from '../lib/queryClient'  // or wherever the singleton lives

interface AuthState {
  // ... existing fields
  setCurrentOrg(org: OrgSummary): Promise<void>
}

export const useAuthStore = create<AuthState>((set, get) => ({
  // ... existing impl

  setCurrentOrg: async (org) => {
    // Notify the backend — it rotates tokens and persists current_org_id
    const resp = await api.post('/api/v1/me/current-org', { org_slug: org.slug })
    setToken(resp.token)
    setRefreshToken(resp.refresh_token)
    set({ currentOrg: org, token: resp.token, refreshToken: resp.refresh_token })
    queryClient.clear()
  },
}))
```

If you don't have a `queryClient` singleton yet, export it from `web/src/main.tsx` or `web/src/lib/queryClient.ts`. The component-level `useQueryClient()` doesn't work here because `setCurrentOrg` is called from the store, not a component.

- [ ] **Step 4: Implement `OrgSwitcher`**

`web/src/components/OrgSwitcher.tsx`:

```tsx
import { useState, useRef, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { ChevronDown, Plus } from 'lucide-react'
import { useAuthStore } from '../stores/authStore'
import { cn } from '../lib/cn'

export function OrgSwitcher() {
  const [open, setOpen] = useState(false)
  const ref = useRef<HTMLDivElement>(null)
  const navigate = useNavigate()
  const { currentOrg, orgs, setCurrentOrg } = useAuthStore()

  useEffect(() => {
    function onClick(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false)
    }
    document.addEventListener('mousedown', onClick)
    return () => document.removeEventListener('mousedown', onClick)
  }, [])

  if (!currentOrg) return null

  async function switchTo(slug: string) {
    const target = orgs.find((o) => o.slug === slug)
    if (!target) return
    setOpen(false)
    await setCurrentOrg(target)
    navigate(`/${slug}/dashboard`)
  }

  return (
    <div ref={ref} className="relative">
      <button
        onClick={() => setOpen(!open)}
        className="flex w-full items-center justify-between rounded-md px-3 py-2 hover:bg-white/5"
      >
        <span className="truncate text-sm font-medium">{currentOrg.name}</span>
        <ChevronDown className="h-4 w-4 opacity-50" />
      </button>

      {open && (
        <div className="absolute left-0 right-0 top-full mt-1 rounded-md border border-white/10 bg-zinc-900 py-1 shadow-lg">
          {orgs.map((org) => (
            <button
              key={org.id}
              onClick={() => switchTo(org.slug)}
              className={cn(
                'flex w-full items-center justify-between px-3 py-1.5 text-sm hover:bg-white/5',
                org.slug === currentOrg.slug && 'text-emerald-400',
              )}
            >
              <span>{org.name}</span>
              {org.slug === currentOrg.slug && <span>✓</span>}
            </button>
          ))}
          <div className="my-1 border-t border-white/10" />
          <button
            onClick={() => { setOpen(false); navigate('/orgs/new') }}
            className="flex w-full items-center gap-2 px-3 py-1.5 text-sm text-zinc-400 hover:bg-white/5"
          >
            <Plus className="h-4 w-4" /> Create org
          </button>
        </div>
      )}
    </div>
  )
}
```

`/orgs/new` will be a stub for Plan 2.1 — the actual create-org page is Plan 2.2's job. For now, just route to a placeholder that calls the API and redirects.

- [ ] **Step 5: Run test — expect PASS**

```bash
npm test -- src/components/OrgSwitcher.test.tsx
```

- [ ] **Step 6: Commit**

```bash
git add web/src/components/OrgSwitcher.tsx web/src/components/OrgSwitcher.test.tsx web/src/stores/authStore.ts
git commit -m "feat(web): OrgSwitcher — switch org, clears cache, navigates"
```

---

### Task 14: Frontend — Migrate routes to `/:orgSlug/*` + add `OrgRouteGuard`

**Files:**
- Modify: `web/src/App.tsx`
- Create: `web/src/components/OrgRouteGuard.tsx`
- Modify: `web/src/components/RequireAuth.tsx` (or wherever it lives)

Routes change:

```
/console/login               → /login                  (no org prefix)
/console/register            → /register
/console/dashboard           → /:orgSlug/dashboard
/console/keys                → /:orgSlug/keys
/console/keys/:id            → /:orgSlug/keys/:id
/console/account             → /:orgSlug/account       (or /account — account is user-scoped)
/console/change-password     → /change-password         (user-scoped, no prefix)
/console/usage               → /:orgSlug/usage
/console/models              → /:orgSlug/models
/console/model-fallbacks     → /:orgSlug/model-fallbacks
/admin/dashboard             → /:orgSlug/admin/dashboard
/admin/channels              → /:orgSlug/admin/channels
... etc
```

- [ ] **Step 1: Write failing test for `OrgRouteGuard`**

`web/src/components/OrgRouteGuard.test.tsx`:

```tsx
import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { MemoryRouter, Routes, Route } from 'react-router-dom'
import { OrgRouteGuard } from './OrgRouteGuard'
import { useAuthStore } from '../stores/authStore'

describe('OrgRouteGuard', () => {
  it('renders children when orgSlug is in user.orgs', () => {
    useAuthStore.setState({
      currentOrg: { id: 'org-1', slug: 'acme', name: 'Acme', role: 'admin', group_id: null },
      orgs: [{ id: 'org-1', slug: 'acme', name: 'Acme', role: 'admin', group_id: null }],
    })
    render(
      <MemoryRouter initialEntries={['/acme/dashboard']}>
        <Routes>
          <Route path="/:orgSlug/*" element={<OrgRouteGuard><div>content</div></OrgRouteGuard>} />
        </Routes>
      </MemoryRouter>,
    )
    expect(screen.getByText('content')).toBeInTheDocument()
  })

  it('redirects when orgSlug is not in user.orgs', () => {
    useAuthStore.setState({
      currentOrg: { id: 'org-1', slug: 'acme', name: 'Acme', role: 'admin', group_id: null },
      orgs: [{ id: 'org-1', slug: 'acme', name: 'Acme', role: 'admin', group_id: null }],
    })
    render(
      <MemoryRouter initialEntries={['/ghost/dashboard']}>
        <Routes>
          <Route path="/:orgSlug/*" element={<OrgRouteGuard><div>content</div></OrgRouteGuard>} />
          <Route path="/acme/dashboard" element={<div>fallback</div>} />
        </Routes>
      </MemoryRouter>,
    )
    expect(screen.queryByText('content')).not.toBeInTheDocument()
    expect(screen.getByText('fallback')).toBeInTheDocument()
  })
})
```

- [ ] **Step 2: Run test — expect FAIL**

```bash
npm test -- src/components/OrgRouteGuard.test.tsx
```

- [ ] **Step 3: Implement `OrgRouteGuard`**

`web/src/components/OrgRouteGuard.tsx`:

```tsx
import { useEffect } from 'react'
import { useParams, Navigate, Outlet } from 'react-router-dom'
import { useAuthStore } from '../stores/authStore'

/**
 * Guards `/:orgSlug/*` routes. Three responsibilities:
 *
 * 1. If the slug isn't in `user.orgs` → redirect to current org (or /login).
 * 2. If the slug differs from `currentOrg.slug` → call `setCurrentOrg` first.
 * 3. Otherwise render children.
 */
export function OrgRouteGuard() {
  const { orgSlug } = useParams<{ orgSlug: string }>()
  const { currentOrg, orgs, setCurrentOrg } = useAuthStore()

  const matched = orgs.find((o) => o.slug === orgSlug)

  useEffect(() => {
    if (matched && currentOrg?.slug !== matched.slug) {
      void setCurrentOrg(matched)
    }
  }, [matched, currentOrg, setCurrentOrg])

  if (!matched) {
    return <Navigate to={currentOrg ? `/${currentOrg.slug}/dashboard` : '/login'} replace />
  }

  return <Outlet />
}
```

- [ ] **Step 4: Update `App.tsx` routes**

```tsx
<BrowserRouter>
  <Routes>
    <Route path="/" element={<Home />} />
    <Route path="/docs/*" element={<DocsLayout />} />

    {/* User-scoped — no org prefix */}
    <Route path="/login" element={<Login />} />
    <Route path="/register" element={<Register />} />
    <Route path="/change-password" element={<ChangePassword />} />
    <Route path="/account" element={<Account />} />

    {/* Org-scoped */}
    <Route path="/:orgSlug" element={<Layout />}>
      <Route element={<RequireAuth />}>
        <Route element={<OrgRouteGuard />}>
          <Route index element={<Navigate to="dashboard" replace />} />
          <Route path="dashboard" element={<Dashboard />} />
          <Route path="keys" element={<Keys />} />
          <Route path="keys/:id" element={<KeyDetail />} />
          <Route path="model-fallbacks" element={<ModelFallbacks />} />
          <Route path="models" element={<ConsoleModels />} />
          <Route path="usage" element={<Usage />} />
        </Route>
      </Route>

      <Route element={<RequireAdmin />}>
        <Route element={<OrgRouteGuard />}>
          <Route path="admin/dashboard" element={<AdminDashboard />} />
          <Route path="admin/channels" element={<Channels />} />
          <Route path="admin/channels/:id" element={<ChannelDetail />} />
          <Route path="admin/providers" element={<Providers />} />
          <Route path="admin/providers/:id" element={<ProviderDetail />} />
          <Route path="admin/models" element={<Models />} />
          <Route path="admin/pricing-policies" element={<PricingPolicies />} />
          <Route path="admin/users" element={<Users />} />
          <Route path="admin/groups" element={<Groups />} />
          <Route path="admin/users/:userId/balance" element={<AccountBalance />} />
          <Route path="admin/settings" element={<Settings />} />
          <Route path="admin/logs" element={<Logs />} />
        </Route>
      </Route>
    </Route>

    {/* Legacy /console/* and /admin/* paths — redirect to current org */}
    <Route path="/console/*" element={<LegacyRedirect />} />
    <Route path="/admin/*" element={<LegacyRedirect />} />

    <Route path="*" element={<Navigate to="/" replace />} />
  </Routes>
</BrowserRouter>
```

`LegacyRedirect`:

```tsx
import { Navigate, useLocation, useParams } from 'react-router-dom'
import { useAuthStore } from '../stores/authStore'

function LegacyRedirect() {
  const location = useLocation()
  const currentOrg = useAuthStore((s) => s.currentOrg)
  if (!currentOrg) return <Navigate to="/login" replace />

  // /console/keys → /${orgSlug}/keys
  // /admin/channels → /${orgSlug}/admin/channels
  const tail = location.pathname.replace(/^\/(console|admin)/, '')
  const isAdmin = location.pathname.startsWith('/admin')
  const newPath = `/${currentOrg.slug}${isAdmin ? '/admin' : ''}${tail}`
  return <Navigate to={newPath} replace />
}
```

- [ ] **Step 5: Update `RequireAdmin`**

`web/src/components/RequireAuth.tsx` (or `RequireAdmin.tsx`):

```tsx
import { useAuthStore } from '../stores/authStore'

export function RequireAdmin({ children }: { children: React.ReactNode }) {
  const { user, currentOrg } = useAuthStore()
  if (!user) return <Navigate to="/login" replace />

  const isOrgAdmin = currentOrg && ['admin', 'owner'].includes(currentOrg.role)
  const isPlatformAdmin = user.platform_role === 'platform_admin'

  if (!isOrgAdmin && !isPlatformAdmin) {
    return <Navigate to={`/${currentOrg?.slug ?? ''}/dashboard`} replace />
  }
  return <>{children}</>
}
```

- [ ] **Step 6: Run tests**

```bash
npm test -- src/components/OrgRouteGuard.test.tsx
```

Expected: PASS.

- [ ] **Step 7: Run full frontend test + build**

```bash
npm test && npm run build
```

Expected: PASS. Any Playwright e2e tests that navigate to `/console/keys` need updating to `/<slug>/keys`.

- [ ] **Step 8: Commit**

```bash
git add web/src/App.tsx web/src/components/
git commit -m "refactor(web): routes under /:orgSlug/* with OrgRouteGuard; legacy paths redirect"
```

---

### Task 15: Frontend — Render `OrgSwitcher` in `Layout`

**Files:**
- Modify: `web/src/components/Layout.tsx`

- [ ] **Step 1: Add OrgSwitcher to the sidebar**

```tsx
import { OrgSwitcher } from './OrgSwitcher'

export function Layout() {
  // ... existing
  return (
    <div className="flex h-screen">
      <aside className="w-60 border-r border-white/10 flex flex-col">
        <div className="border-b border-white/10 p-3">
          <OrgSwitcher />
        </div>
        <nav className="flex-1 p-3">
          {/* ... existing nav items */}
        </nav>
      </aside>
      <main className="flex-1">
        <Outlet />
      </main>
    </div>
  )
}
```

- [ ] **Step 2: Manual smoke test**

```bash
source ~/.nvm/nvm.sh && cd web && npm run dev
```

Open `http://localhost:5173`, log in, verify:
- OrgSwitcher shows "Default Org" in the sidebar
- Clicking it shows the orgs list (only one entry for now — "Default Org")
- The "Create org" button navigates to `/orgs/new` (stub OK for Plan 2.1)
- The existing nav items still work

- [ ] **Step 3: Commit**

```bash
git add web/src/components/Layout.tsx
git commit -m "feat(web): OrgSwitcher in sidebar"
```

---

### Task 16: End-to-end verification

**Files:** (no file changes — verification only)

- [ ] **Step 1: Full backend test suite**

```bash
cargo test --workspace
```

Expected: all tests pass.

- [ ] **Step 2: Full frontend test + build**

```bash
source ~/.nvm/nvm.sh && cd web && npm test && npm run build
```

Expected: tests pass, build clean.

- [ ] **Step 3: Backend smoke — manual HTTP**

Start the backend:

```bash
cargo run &
```

Try the legacy path — should get 410:

```bash
curl -i http://localhost:8080/api/v1/keys
# Expected: HTTP/1.1 410 Gone
# Body: {"error":"gone","new_path":"/api/v1/{org_slug}/keys",...}
```

Try the new path without auth — should get 401:

```bash
curl -i http://localhost:8080/api/v1/default/keys
# Expected: HTTP/1.1 401 Unauthorized
```

Try with auth — should get 200:

```bash
TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"admin","password":"<your-admin-password>"}' | jq -r .token)

curl -i -H "Authorization: Bearer $TOKEN" http://localhost:8080/api/v1/default/keys
# Expected: HTTP/1.1 200 OK
```

Try a non-member org — should get 403:

```bash
# Create a second org
curl -i -X POST -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"slug":"acme","name":"Acme"}' http://localhost:8080/api/v1/orgs
# Expected: 201 Created

# Try to read acme's keys as a different user (or before being added as member)
# Should be 403 if user isn't a member, 200 if they are (creator is auto-member).
```

- [ ] **Step 4: Frontend smoke — browser test**

```bash
cd web && npm run dev
```

In a browser:
1. Open `http://localhost:5173/login` — log in
2. Verify URL redirects to `/default/dashboard`
3. Verify the OrgSwitcher shows "Default Org" in the sidebar
4. Verify keys, channels, providers, models, usage, logs pages all load
5. Visit `http://localhost:5173/console/keys` — should redirect to `/default/keys`
6. Click "Create org" in the switcher → fill in slug + name → should land in `/<new-slug>/dashboard`
7. Switch back to "Default Org" — URL changes, cache cleared, no cross-org data leaks

- [ ] **Step 5: Cross-org isolation test**

With two orgs created:
1. Switch to Org A — create a key
2. Switch to Org B — the keys list should NOT show Org A's key
3. Inspect `localStorage` / network tab — no stale cached queries

- [ ] **Step 6: Database spot-check**

```bash
psql -U llm_gateway -d llm_gateway -c "
  SELECT id, slug, name FROM orgs ORDER BY created_at;
  SELECT org_id, COUNT(*) FROM api_keys GROUP BY org_id;
  SELECT org_id, COUNT(*) FROM members GROUP BY org_id;
"
```

Expected:
- 2+ orgs (the default + any you created during smoke test)
- Each api_key is associated with exactly one org
- Memberships exist for the test user in each org

- [ ] **Step 7: Final cleanup commit (if any)**

```bash
git status
git log --oneline -20
```

Confirm: clean tree, commits for each task, no `--no-verify` shortcuts.

---

## Self-Review Notes

**Spec coverage check** — every Phase 2 deliverable that Plan 2.1 covers maps to a task:

| Spec deliverable (Phase 2) | Task |
|---|---|
| Management API moved to `/api/v1/{org_slug}/...` | Task 6 + Task 7 |
| Middleware chain: AuthLayer → OrgResolveLayer → MembershipLayer | Tasks 2, 3, 4 |
| `POST /api/v1/orgs` create-org | Task 9 |
| Frontend: `/:orgSlug/*` routes | Task 14 |
| OrgSwitcher fully functional | Task 13 |
| React Query keys prefixed with orgSlug | Task 12 |
| API client uses `orgPrefix()` helper | Tasks 10, 11 |
| Old `/api/v1/{resource}` URLs return 410 Gone | Task 8 |

**Out of scope for Plan 2.1** (deferred to Plans 2.2 + 2.3):

| Spec deliverable | Plan |
|---|---|
| Platform-admin impersonation via temp member row + janitor | 2.3 |
| Members page + Org Settings page | 2.2 |
| Org-private catalog CRUD with anti-shadowing | 2.3 |
| UI "Platform" vs "Ours" filter on catalog listings | 2.3 |

**Placeholder scan** — no TBD / TODO outside the explicit `// Plan 2.3 will add ...` and `// Plan 2.2 ...` forward references in Task 4 (membership_layer), Task 13 (OrgSwitcher Create-org link), and Task 14 (`/orgs/new` stub). These are intentional hooks, not gaps.

**Type consistency** — `OrgContext` gains `user_id: String` (Task 7) that wasn't in Phase 1's struct. The membership_layer is updated to set it. `ResolvedOrg` is new in Plan 2.1 and lives in `crates/org/src/types.rs` alongside `OrgContext`.

**Risks worth flagging to the implementer** (call out in PR description):

1. **Breaking change.** Every external API client breaks. Coordinate with known integrators; document the migration in CHANGELOG. Provide the 410 Gone response with a pointer to the new path so client-side debugging is straightforward.

2. **Middleware ordering matters.** Axum runs `from_fn` layers in registration order from outermost to innermost (last registered = innermost = runs first when unwinding). Task 6 registers them as `auth → org_resolve → membership` outermost-to-innermost, which means execution order on a request is `auth → org_resolve → membership → handler`. Verify with a unit test if unsure.

3. **`require_auth` is now dead code from a handler's perspective**, but the `auth_layer` middleware calls it. Don't delete the function — the middleware reuses it. Mark it `#[deprecated(note = "use the auth_layer middleware instead")]` if you want a compile-time hint for future contributors.

4. **React Query cache leak is the highest-risk frontend bug.** Task 12 mitigates it by prefixing keys with slug, but `queryClient.clear()` in `setCurrentOrg` (Task 13) is the second line of defense. Both must ship together.

5. **`LegacyRedirect` for `/console/*` and `/admin/*` is a UX call, not a contract.** If the team prefers a hard 410 on the frontend too (matching the backend), replace the redirect with a "this page moved" landing page that links to the new URL.

6. **JwtClaims must impl `FromRequestParts`** for Task 9's `create_org` handler. If adding the impl in `crates/auth` is undesirable, write a thin extractor `AuthenticatedUser(JwtClaims)` in `crates/api/src/extractors.rs` that wraps `require_auth`, and use that everywhere a global (non-org-scoped) endpoint needs auth.

7. **Slug charset `^[a-z0-9-]{3,64}$` is enforced twice** — at the DB layer (migration's `CHECK` constraint) and in `orgs::create_org` (Task 9's `SLUG_RE`). The DB check is the source of truth; the Rust check gives a cleaner 400 than the DB's error. Keep both.
