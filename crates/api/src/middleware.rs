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
