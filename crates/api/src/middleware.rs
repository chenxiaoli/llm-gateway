//! Axum middlewares for the management API.
//!
//! Installed in this order on every `/api/v1/{org_slug}/*` route:
//!   1. auth_layer         — verify JWT, inject JwtClaims
//!   2. org_resolve_layer  — slug → Org, inject ResolvedOrg
//!   3. membership_layer   — verify (user, org) ∈ members, inject OrgContext
//!
//! Handlers pull `OrgContext` via `FromRequestParts`; they no longer call
//! `require_auth(&headers, ...)` directly.

use std::sync::Arc;

use axum::extract::{Path, Request, State};
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::Response;
use std::collections::HashMap;

use crate::error::ApiError;
use crate::extractors::require_auth;
use crate::AppState;
use llm_gateway_auth::JwtClaims;
use llm_gateway_org::{OrgContext, ResolvedOrg};
use llm_gateway_storage::PlatformRole;

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

/// Resolve `{org_slug}` from the matched path to an `Org` row and inject a
/// `ResolvedOrg` into request extensions for downstream layers/handlers.
///
/// Returns 404 if no org matches the slug (distinct from the 403 the later
/// `membership_layer` would raise for a non-member). Path params are only
/// populated by Axum after route matching, so this layer must come after the
/// router — it cannot short-circuit unmatched paths.
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
        .map_err(|e| ApiError::Internal(format!("org lookup failed: {e}")))?;
    let org = org.ok_or_else(|| ApiError::NotFound("org not found".into()))?;
    req.extensions_mut().insert(ResolvedOrg {
        id: org.id,
        slug: org.slug,
        name: org.name,
    });
    Ok(next.run(req).await)
}

/// Verify the authenticated user is a member of the resolved org and inject
/// an `OrgContext` into request extensions for downstream handlers.
///
/// Pulls `JwtClaims` and `ResolvedOrg` (set by the two upstream layers) from
/// extensions, looks up the membership row, and returns 403 Forbidden if no
/// such row exists. Storage failures surface as 500 Internal — distinct from
/// the clean 403 for a non-member, so a DB outage does not look like an
/// authz rejection.
///
/// `platform_role` is parsed via `PlatformRole::parse` rather than the
/// `map(|_| ...)` shortcut suggested in the plan — that shortcut silently
/// maps any non-None string to `PlatformAdmin`, which would mask a stale or
/// unexpected JWT claim. `parse` returns `None` for unrecognized values,
/// matching the behavior of `resolve_org_context` in `crates/org`.
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
        .map_err(|e| ApiError::Internal(format!("member lookup failed: {e}")))?
        .ok_or(ApiError::Forbidden)?;

    let ctx = OrgContext {
        user_id: claims.sub.clone(),
        org_id: org.id.clone(),
        member_role: member.role,
        platform_role: claims
            .platform_role
            .as_deref()
            .and_then(PlatformRole::parse),
        group_id: member.group_id,
    };
    req.extensions_mut().insert(ctx);

    Ok(next.run(req).await)
}
