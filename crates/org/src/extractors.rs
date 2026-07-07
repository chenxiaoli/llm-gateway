use crate::error::OrgError;
use crate::types::{OrgContext, PlatformRole};
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
        org_id,
        member_role: member.role,
        platform_role,
        group_id: member.group_id,
    })
}
