use llm_gateway_storage::{Member, MemberRole, Org, PlatformRole};

/// Per-request context derived from JWT + membership lookup.
/// In Phase 1 this is constructed from `claims.current_org_id` only
/// (no path-based routing yet — Phase 2 adds OrgResolveLayer/MembershipLayer).
#[derive(Debug, Clone)]
pub struct OrgContext {
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

pub use llm_gateway_storage::{Member, MemberRole, Org, PlatformRole};
