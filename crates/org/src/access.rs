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
