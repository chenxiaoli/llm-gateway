use crate::error::OrgError;
use crate::types::{OrgContext, PlatformRole};
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use llm_gateway_auth::JwtClaims;
use llm_gateway_storage::Storage;

/// Resolve the per-request [`OrgContext`] from JWT claims plus a storage
/// membership lookup.
///
/// Phase 1: `org_id` always comes from `claims.current_org_id` (no path-based
/// routing yet). Phase 2 will look up via path `{org_slug}` and require active
/// membership.
///
/// Errors:
/// - [`OrgError::NotMember`] when the user has no member row in the org.
/// - [`OrgError::NotFound`] when the underlying storage call fails (surfaced
///   as a generic lookup failure for now).
// TODO(Task 9/10): storage failures are currently mapped to `NotFound`, which
// will mislead API handlers that branch on `NotFound` -> 404. Before wiring
// this into HTTP responses, add `OrgError::Internal` (or similar) and remap
// storage errors there so a DB outage becomes 500, not 404.
pub async fn resolve_org_context(
    claims: &JwtClaims,
    storage: &dyn Storage,
) -> Result<OrgContext, OrgError> {
    let org_id = claims.current_org_id.clone();

    let member = storage
        .get_member(&claims.sub, &org_id)
        .await
        .map_err(|e| OrgError::NotFound(format!("member lookup failed: {e}")))?
        .ok_or_else(|| OrgError::NotMember(claims.sub.clone(), org_id.clone()))?;

    // Platform_admin without a member row is rejected above. Phase 2 will
    // auto-create a temp row; Phase 1 simplification requires platform_admins
    // to also have a member row in the default org (the migration ensures
    // this for pre-existing admins).
    //
    // An unrecognized `platform_role` string on the JWT silently downgrades to
    // `None` — the JWT is authoritative but stale tokens might carry unexpected
    // values, and we prefer a degraded-but-working session over a hard failure.
    let platform_role = claims
        .platform_role
        .as_deref()
        .and_then(PlatformRole::parse);

    Ok(OrgContext {
        user_id: claims.sub.clone(),
        org_id,
        member_role: member.role,
        platform_role,
        group_id: member.group_id,
    })
}

/// Axum extractor for [`OrgContext`].
///
/// Pulls the context out of request extensions, where the membership middleware
/// (Phase 2 `membership_layer`) injects it after the JWT + membership lookup.
/// Handlers can therefore declare `ctx: OrgContext` as a parameter and receive
/// the pre-resolved context without re-running the storage lookup.
///
/// If the extension is missing we fail with 500 — the middleware chain is
/// misconfigured (e.g. a route mounted without `membership_layer`), and that's
/// a programmer error rather than a client error.
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
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "OrgContext missing from request extensions — middleware chain misconfigured",
                )
                    .into_response()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemberRole, OrgContext};
    use axum::http::Request;

    #[tokio::test]
    async fn extracts_org_context_from_extensions() {
        let ctx = OrgContext {
            user_id: "u".into(),
            org_id: "org_default".into(),
            member_role: MemberRole::Owner,
            platform_role: None,
            group_id: None,
        };
        let req: Request<()> = Request::default();
        let (mut parts, body) = req.into_parts();
        parts.extensions.insert(ctx.clone());
        // Body is unused — `from_request_parts` only inspects `Parts`.
        let _ = body;

        let extracted = OrgContext::from_request_parts(&mut parts, &())
            .await
            .expect("OrgContext should be in extensions");

        assert_eq!(extracted.org_id, ctx.org_id);
        assert_eq!(extracted.member_role, ctx.member_role);
    }

    #[tokio::test]
    async fn rejects_when_missing_from_extensions() {
        let req: Request<()> = Request::default();
        let (mut parts, _body) = req.into_parts();

        let result = OrgContext::from_request_parts(&mut parts, &()).await;
        assert!(result.is_err());
    }
}
