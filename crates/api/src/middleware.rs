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

use axum::extract::{Request, State};
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::Response;

use crate::error::ApiError;
use crate::extractors::require_auth;
use crate::AppState;

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
