pub mod accounts;
pub mod auth;
pub mod channels;
pub mod groups;
pub mod keys;
pub mod model_fallbacks;
pub mod providers;
pub mod models;
pub mod usage;
pub mod logs;
pub mod members;
pub mod requests;
pub mod users;
pub mod settings;
pub mod channel_models;
pub mod pricing_policies;
pub mod seed;
pub mod nats;

use axum::extract::State;
use axum::middleware::from_fn_with_state;
use axum::response::IntoResponse;
use axum::routing::{any, get, patch, post};
use axum::{Json, Router};
use std::sync::Arc;

use crate::middleware::{auth_layer, membership_layer, org_resolve_layer};
use crate::AppState;

/// Build the management API router.
///
/// Splits routes into two groups:
///
/// - **Global** (no per-org context): `/api/v1/auth/*`, `/api/v1/me/*`,
///   `/api/v1/orgs`, `/api/v1/version`, `/api/v1/admin/system-info`,
///   `/api/v1/admin/nats/status`. Auth is enforced inside the handlers via
///   `require_auth` / `require_platform_admin` — these routes either predate
///   the Phase 2 middleware chain or are explicitly platform-level.
///
/// - **Org-scoped** (full middleware chain): everything under
///   `/api/v1/{org_slug}/*`. The chain runs `auth_layer → org_resolve_layer
///   → membership_layer` (outermost first) and injects `OrgContext`, which
///   handlers pull via Axum's `FromRequestParts` extractor.
///
/// Requests to legacy global paths that moved under `/{org_slug}/` (e.g.
/// `/api/v1/keys`, `/api/v1/admin/users`) fall through to [`legacy_gone`]
/// and get a 410 with a pointer to the new path. We attach it via `.nest()`
/// under a catch-all inside this router so it does not interfere with the
/// gateway-level SPA fallback in `main.rs`.
pub fn management_router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // Org-scoped routes: full middleware chain.
    //
    // Axum applies `.layer()` outside-in: the last `layer()` call wraps the
    // innermost position in the chain, so to get execution order
    //   auth → org_resolve → membership
    // we add them in reverse: membership first, then org_resolve, then auth.
    let org_scoped = org_scoped_routes()
        .layer(from_fn_with_state(state.clone(), membership_layer))
        .layer(from_fn_with_state(state.clone(), org_resolve_layer))
        .layer(from_fn_with_state(state.clone(), auth_layer));

    Router::new()
        // --- Global routes (auth handled inside handlers) ---
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/config", get(auth::auth_config))
        .route("/api/v1/auth/me", get(auth::me))
        .route("/api/v1/auth/me/balance", get(auth::me_balance))
        .route("/api/v1/auth/refresh", post(auth::refresh))
        .route("/api/v1/auth/change-password", post(auth::change_password))
        // Orgs (authenticated) — list/create/switch membership context.
        // These are global: they operate on the user's set of memberships,
        // not on a single org scoped by path.
        .route("/api/v1/orgs", get(auth::list_orgs).post(auth::create_org))
        .route("/api/v1/me/current-org", post(auth::switch_org))
        // Version + system info + NATS status (global platform-level).
        .route("/api/v1/version", get(version))
        .route("/api/v1/admin/system-info", get(system_info))
        .route("/api/v1/admin/nats/status", get(nats::get_nats_status))
        // --- Explicit 410 routes for single-segment legacy paths ---
        //
        // Pre-Phase-2 endpoints whose root lived directly at `/api/v1/<name>`
        // (e.g. `/api/v1/keys`, `/api/v1/model-fallbacks`, `/api/v1/usage`)
        // MUST return 410 Gone, but Axum's matchit treats these segments as
        // valid captures for the `/{org_slug}` nest below, which causes the
        // request to enter the middleware chain and fail at `auth_layer`
        // with 401 before `legacy_gone` can run. The spec promises 410 here,
        // not 401, so we register these three paths as literal routes
        // *before* the `/{org_slug}` nest. Axum prioritizes literal segments
        // over captures, so these win the match and route to `legacy_gone`
        // for any HTTP method. Each root is registered twice — once for the
        // bare path and once with a `{*rest}` catch-all — because paths
        // like `/api/v1/keys/abc-123` would otherwise still be captured as
        // `org_slug=keys, inner=abc-123` by the nest below.
        //
        // Do not delete these as "redundant" with the `.fallback` below:
        // the fallback only sees paths that didn't match a route *or* a
        // nest. The `/{org_slug}` nest IS a match (for any string), so
        // single-segment legacy paths never reach the fallback.
        //
        // Tradeoff: a slug collision is now possible. If an org is later
        // created with `slug = "keys" | "model-fallbacks" | "usage"`, then
        // `/api/v1/{that-slug}/...` would be absorbed by these literal
        // routes (returning 410) rather than reaching the org-scoped
        // handler. Reserved-word denylist on org creation is a future fix.
        .route("/api/v1/keys", any(legacy_gone))
        .route("/api/v1/keys/{*rest}", any(legacy_gone))
        .route("/api/v1/model-fallbacks", any(legacy_gone))
        .route("/api/v1/model-fallbacks/{*rest}", any(legacy_gone))
        .route("/api/v1/usage", any(legacy_gone))
        .route("/api/v1/usage/{*rest}", any(legacy_gone))
        // --- Org-scoped routes (middleware chain applied) ---
        .nest("/api/v1/{org_slug}", org_scoped)
        // --- Legacy catch-all ---
        // Anything else under /api/v1/ that didn't match a global route or
        // the org-scoped nest above is a pre-Phase-2 path. Return 410 with
        // a hint. Use a dedicated sub-router with its own fallback so it
        // doesn't steal matches from the routes above and doesn't collide
        // with the gateway-level SPA fallback.
        .nest("/api/v1", legacy_router())
}

/// Per-org routes. Mounted under `/api/v1/{org_slug}` via [`management_router`].
///
/// Path params are relative to that prefix (so `/keys` here matches
/// `/api/v1/{org_slug}/keys`).
fn org_scoped_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Org details (authenticated member) — GET /api/v1/{org_slug}
        .route("/", get(auth::get_org))
        // Keys (authenticated)
        .route("/keys", post(keys::create_key).get(keys::list_keys))
        .route(
            "/keys/{id}",
            get(keys::get_key).patch(keys::update_key).delete(keys::delete_key),
        )
        // Model Fallbacks (authenticated)
        .route(
            "/model-fallbacks",
            post(model_fallbacks::create_model_fallback).get(model_fallbacks::list_model_fallbacks),
        )
        .route(
            "/model-fallbacks/{id}",
            get(model_fallbacks::get_model_fallback).patch(model_fallbacks::update_model_fallback).delete(model_fallbacks::delete_model_fallback),
        )
        // Providers (admin)
        .route(
            "/admin/providers",
            post(providers::create_provider).get(providers::list_providers),
        )
        .route(
            "/admin/providers/{id}",
            get(providers::get_provider).patch(providers::update_provider).delete(providers::delete_provider),
        )
        .route(
            "/admin/providers/{id}/channels",
            post(channels::create_channel).get(channels::list_channels),
        )
        .route(
            "/admin/providers/{id}/models",
            get(models::list_provider_models).put(models::update_provider_models),
        )
        .route(
            "/admin/channels",
            post(channels::create_channel).get(channels::list_all_channels),
        )
        .route(
            "/admin/channels/{id}",
            get(channels::get_channel).patch(channels::update_channel).delete(channels::delete_channel),
        )
        .route(
            "/admin/channels/{id}/api-key",
            patch(channels::update_channel_api_key),
        )
        .route(
            "/admin/channels/{id}/test",
            post(channels::test_channel),
        )
        // Groups (admin)
        .route(
            "/admin/groups",
            post(groups::create_group).get(groups::list_groups),
        )
        .route(
            "/admin/groups/{id}",
            get(groups::get_group).patch(groups::update_group).delete(groups::delete_group),
        )
        .route(
            "/admin/models",
            get(models::list_all_models).post(models::create_model_global),
        )
        .route(
            "/admin/models/{model_name}",
            patch(models::update_model).delete(models::delete_model),
        )
        // ChannelModels (admin)
        .route(
            "/admin/providers/{provider_id}/channel-models",
            post(channel_models::create_channel_model).get(channel_models::list_channel_models),
        )
        .route(
            "/admin/channels/{channel_id}/channel-models",
            get(channel_models::list_channel_models_by_channel).post(channel_models::create_channel_model_by_channel),
        )
        .route(
            "/admin/channel-models/{id}",
            get(channel_models::get_channel_model).patch(channel_models::update_channel_model).delete(channel_models::delete_channel_model),
        )
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
        .route(
            "/admin/users/{id}",
            patch(users::update_user).delete(users::delete_user),
        )
        // Account / Balance (admin)
        .route(
            "/admin/users/{id}/balance",
            get(accounts::get_balance),
        )
        .route(
            "/admin/users/{id}/recharge",
            post(accounts::recharge),
        )
        .route(
            "/admin/users/{id}/adjust",
            post(accounts::adjust),
        )
        .route(
            "/admin/users/{id}/threshold",
            patch(accounts::update_threshold),
        )
        // Settings (admin)
        .route("/admin/settings", get(settings::get_settings).patch(settings::update_settings))
        // Seed data (reads static JSON)
        .route("/admin/seed", get(seed::get_seed_data))
        // Pricing Policies (admin)
        .route(
            "/admin/pricing-policies",
            post(pricing_policies::create).get(pricing_policies::list),
        )
        .route(
            "/admin/pricing-policies/{id}",
            get(pricing_policies::get).patch(pricing_policies::update).delete(pricing_policies::delete),
        )
        // Members (admin)
        .route(
            "/members",
            get(members::list_members).post(members::invite_member),
        )
        .route(
            "/members/{user_id}",
            patch(members::change_member_role).delete(members::remove_member),
        )
}

/// Sub-router that turns any unmatched `/api/v1/*` path into a 410 Gone.
///
/// Mounted under `/api/v1` via `.nest()` in [`management_router`]. Because
/// the global routes (`/auth/*`, `/orgs`, `/me/*`, `/version`, two
/// `/admin/*` platform routes) and the `/{org_slug}` nest are registered on
/// the *outer* router first, Axum will route those ahead of this fallback
/// for matching paths. Only paths that match none of the above fall through
/// here — i.e. the pre-Phase-2 management endpoints.
///
/// Using a `.fallback()` on a nested router (instead of an explicit
/// `/api/v1/{*legacy}` route on the outer router) sidesteps the
/// `RouteConflictError` that arises when a top-level catch-all shadows the
/// `/{org_slug}/...` nest.
fn legacy_router() -> Router<Arc<AppState>> {
    Router::new().fallback(legacy_gone)
}

/// 410 Gone for pre-Phase-2 management routes.
///
/// Returns JSON describing where the endpoint moved. The migration is
/// mechanical: insert `{org_slug}/` after `/api/v1/`.
///
/// The unmatched URI is read from the request directly (rather than via
/// `Path`) because Axum does not populate `Path` params for `.fallback()`
/// handlers — there is no named capture in the matched "route" to deserialize
/// from. Reading `req.uri().path()` lets us echo back the legacy suffix.
async fn legacy_gone(req: axum::extract::Request) -> impl IntoResponse {
    // Strip the /api/v1 prefix so the suggested new_path reads cleanly as
    // /api/v1/{org_slug}/<suffix>. The nest prefix is already stripped from
    // `req.uri().path()` in some Axum versions and not others, so we handle
    // both shapes defensively.
    let full = req.uri().path();
    let legacy = full
        .strip_prefix("/api/v1/")
        .or_else(|| full.strip_prefix("/api/v1"))
        .unwrap_or(full);
    let legacy = legacy.trim_start_matches('/');
    let new_path = format!("/api/v1/{{org_slug}}/{legacy}");
    (
        axum::http::StatusCode::GONE,
        Json(serde_json::json!({
            "error": "gone",
            "message": "This endpoint moved in v2.1.0. Add your org slug after /api/v1/.",
            "new_path": new_path,
        })),
    )
}

async fn version(State(_state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": option_env!("GIT_VERSION").unwrap_or(concat!("v", env!("CARGO_PKG_VERSION"))),
    }))
}

async fn system_info(State(state): State<Arc<AppState>>) -> Json<crate::SystemInfo> {
    Json(state.system_info.clone())
}
