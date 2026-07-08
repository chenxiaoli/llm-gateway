use crate::types::{MemberRole, OrgContext};

/// Admin-or-above in the current org, OR platform_admin.
pub fn can_manage_org_settings(ctx: &OrgContext) -> bool {
    matches!(ctx.member_role, MemberRole::Owner | MemberRole::Admin)
        || ctx.is_platform_admin()
}

pub fn can_invite_members(ctx: &OrgContext) -> bool {
    matches!(ctx.member_role, MemberRole::Owner | MemberRole::Admin)
        || ctx.is_platform_admin()
}

pub fn can_delete_org(ctx: &OrgContext) -> bool {
    matches!(ctx.member_role, MemberRole::Owner) || ctx.is_platform_admin()
}

pub fn can_manage_channels(ctx: &OrgContext) -> bool {
    matches!(ctx.member_role, MemberRole::Owner | MemberRole::Admin)
        || ctx.is_platform_admin()
}

pub fn can_create_org_catalog(ctx: &OrgContext) -> bool {
    matches!(ctx.member_role, MemberRole::Owner | MemberRole::Admin)
        || ctx.is_platform_admin()
}

pub fn can_create_platform_catalog(ctx: &OrgContext) -> bool {
    ctx.is_platform_admin()
}

/// Returns true if the user may mutate a catalog entry with the given `owner_org_id`.
///
/// - `None` (platform-level entry): only platform admins.
/// - `Some(org_id)` (org-private entry): admin/owner of that org, or platform admin.
///
/// Use this for update/delete on existing `providers`, `models`, and
/// `pricing_policies` rows. Create handlers do NOT call this — they must
/// decide the new row's `owner_org_id` (where to attach) before they can
/// evaluate permission, so they keep the inline three-branch check.
/// Read paths do NOT call this — catalog visibility is filtered at the
/// storage layer.
pub fn can_mutate_catalog_entry(ctx: &OrgContext, entry_owner_org_id: Option<&str>) -> bool {
    if ctx.is_platform_admin() {
        return true;
    }
    match entry_owner_org_id {
        None => false,
        Some(entry_org) => {
            entry_org == ctx.org_id
                && matches!(ctx.member_role, MemberRole::Owner | MemberRole::Admin)
        }
    }
}

/// Used by channel-listing filter: members see channels in their group + ungrouped;
/// admin/owner/platform_admin see everything.
pub fn can_access_channel(ctx: &OrgContext, channel_group_id: Option<&str>) -> bool {
    match ctx.member_role {
        MemberRole::Owner | MemberRole::Admin => true,
        MemberRole::Member => {
            ctx.is_platform_admin()
                || channel_group_id.is_none()
                || ctx.group_id.as_deref() == channel_group_id
        }
    }
}

/// Generic admin check: org Owner/Admin OR platform_admin.
///
/// Prefer the more specific `can_*` helpers (e.g. `can_manage_channels`,
/// `can_manage_org_settings`) when the policy is named — they make the intent
/// of the call site self-documenting and stay accurate if the policy for that
/// surface diverges from the generic admin rule in a later phase. Use this
/// helper only when the check is truly "is this user an admin-like actor for
/// the migrated surface that used to be gated on `require_platform_admin`".
pub fn can_administer(ctx: &OrgContext) -> bool {
    matches!(ctx.member_role, MemberRole::Owner | MemberRole::Admin)
        || ctx.is_platform_admin()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MemberRole, OrgContext, PlatformRole};

    fn ctx(role: MemberRole, platform: Option<PlatformRole>, group_id: Option<&str>) -> OrgContext {
        OrgContext {
            user_id: "u".into(),
            org_id: "o".into(),
            member_role: role,
            platform_role: platform,
            group_id: group_id.map(String::from),
        }
    }

    #[test]
    fn owner_can_delete_org() {
        assert!(can_delete_org(&ctx(MemberRole::Owner, None, None)));
    }

    #[test]
    fn admin_cannot_delete_org() {
        assert!(!can_delete_org(&ctx(MemberRole::Admin, None, None)));
    }

    #[test]
    fn platform_admin_can_delete_org_even_as_member() {
        assert!(can_delete_org(&ctx(
            MemberRole::Member,
            Some(PlatformRole::PlatformAdmin),
            None
        )));
    }

    #[test]
    fn member_cannot_invite() {
        assert!(!can_invite_members(&ctx(MemberRole::Member, None, None)));
    }

    #[test]
    fn member_sees_ungrouped_channels() {
        assert!(can_access_channel(
            &ctx(MemberRole::Member, None, None),
            None
        ));
    }

    #[test]
    fn member_sees_own_group_channels() {
        assert!(can_access_channel(
            &ctx(MemberRole::Member, None, Some("g1")),
            Some("g1")
        ));
    }

    #[test]
    fn member_blocked_from_other_group_channels() {
        assert!(!can_access_channel(
            &ctx(MemberRole::Member, None, Some("g1")),
            Some("g2")
        ));
    }

    #[test]
    fn admin_sees_all_channels() {
        assert!(can_access_channel(
            &ctx(MemberRole::Admin, None, None),
            Some("anything")
        ));
    }

    #[test]
    fn owner_can_mutate_own_org_private_entry() {
        assert!(can_mutate_catalog_entry(
            &ctx(MemberRole::Owner, None, None),
            Some("o")
        ));
    }

    #[test]
    fn admin_can_mutate_own_org_private_entry() {
        assert!(can_mutate_catalog_entry(
            &ctx(MemberRole::Admin, None, None),
            Some("o")
        ));
    }

    #[test]
    fn member_cannot_mutate_own_org_private_entry() {
        assert!(!can_mutate_catalog_entry(
            &ctx(MemberRole::Member, None, None),
            Some("o")
        ));
    }

    #[test]
    fn owner_cannot_mutate_other_org_private_entry() {
        assert!(!can_mutate_catalog_entry(
            &ctx(MemberRole::Owner, None, None),
            Some("other")
        ));
    }

    #[test]
    fn owner_cannot_mutate_platform_entry_without_platform_admin() {
        assert!(!can_mutate_catalog_entry(
            &ctx(MemberRole::Owner, None, None),
            None
        ));
    }

    #[test]
    fn platform_admin_can_mutate_platform_entry() {
        assert!(can_mutate_catalog_entry(
            &ctx(MemberRole::Member, Some(PlatformRole::PlatformAdmin), None),
            None
        ));
    }

    #[test]
    fn platform_admin_can_mutate_own_org_private_entry() {
        assert!(can_mutate_catalog_entry(
            &ctx(MemberRole::Member, Some(PlatformRole::PlatformAdmin), None),
            Some("o")
        ));
    }

    #[test]
    fn platform_admin_can_mutate_other_org_private_entry() {
        assert!(can_mutate_catalog_entry(
            &ctx(MemberRole::Member, Some(PlatformRole::PlatformAdmin), None),
            Some("other")
        ));
    }
}
