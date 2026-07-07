pub use llm_gateway_storage::{Member, MemberRole, Org, PlatformRole};

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

/// Per-request context derived from JWT + membership lookup.
/// In Phase 1 this is constructed from `claims.current_org_id` only
/// (no path-based routing yet — Phase 2 adds OrgResolveLayer/MembershipLayer).
///
/// Field order note: `user_id` is first so that when additional identity fields
/// arrive in later phases (e.g. an org-scoped display name) they slot in below
/// the identity block and above the role block.
#[derive(Debug, Clone)]
pub struct OrgContext {
    pub user_id: String,
    pub org_id: String,
    pub member_role: MemberRole,
    pub platform_role: Option<PlatformRole>,
    pub group_id: Option<String>,
}

impl OrgContext {
    pub fn is_platform_admin(&self) -> bool {
        matches!(self.platform_role, Some(PlatformRole::PlatformAdmin))
    }
}
