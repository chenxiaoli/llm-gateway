use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use llm_gateway_org::{can_administer, OrgContext};
use llm_gateway_storage::MemberRole;

use crate::error::ApiError;
use crate::AppState;

/// Enriched per-membership view returned by the management API.
///
/// Sourced from `MemberWithDetails` (a join of `members` × `users` × `groups` ×
/// `accounts`). `balance` / `threshold` are converted from subunits (10⁸ per
/// USD) to USD floats via `units_to_usd`; `created_at` is RFC3339. The old
/// `joined_at` field name is gone — use `created_at`.
#[derive(serde::Serialize)]
pub struct MemberResponse {
    pub user_id: String,
    pub username: String,
    pub email: Option<String>,
    pub role: String,
    pub group_id: Option<String>,
    pub group_name: Option<String>,
    pub enabled: bool,
    pub balance: f64,
    pub threshold: f64,
    pub created_at: String,
}

impl From<llm_gateway_storage::MemberWithDetails> for MemberResponse {
    fn from(m: llm_gateway_storage::MemberWithDetails) -> Self {
        MemberResponse {
            user_id: m.user_id,
            username: m.username,
            email: m.email,
            role: m.role,
            group_id: m.group_id,
            group_name: m.group_name,
            enabled: m.enabled,
            balance: llm_gateway_storage::units_to_usd(m.balance),
            threshold: llm_gateway_storage::units_to_usd(m.threshold),
            created_at: m.created_at.to_rfc3339(),
        }
    }
}

/// Build a `MemberResponse` from a plain `Member` plus the username, filling
/// the enriched-view fields with neutral defaults. Used by `invite_member`
/// where we don't have a `MemberWithDetails` row yet (the account row may not
/// exist; we just inserted the membership). The defaults match what storage's
/// `list_members` SQL COALESCEs to for a freshly-created membership: zero
/// balance, default threshold (1.0 USD), no group, enabled user.
fn build_response(member: llm_gateway_storage::Member, username: String) -> MemberResponse {
    MemberResponse {
        user_id: member.user_id,
        username,
        email: None,
        role: member.role.as_str().to_string(),
        group_id: member.group_id,
        group_name: None,
        enabled: true,
        balance: 0.0,
        threshold: 1.0,
        created_at: member.created_at.to_rfc3339(),
    }
}

// --- Role parsing ---

fn parse_role(s: &str) -> Result<MemberRole, ApiError> {
    MemberRole::parse(s).ok_or_else(|| {
        ApiError::BadRequest(format!(
            "unknown role '{s}'; expected one of: owner, admin, member"
        ))
    })
}

#[derive(Debug, Deserialize)]
pub struct InviteRequest {
    pub username: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemberBody {
    pub role: Option<String>,
    pub enabled: Option<bool>,
    /// `null` clears the group; `Some(id)` assigns it.
    pub group_id: Option<Option<String>>,
}

pub async fn list_members(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
) -> Result<Json<Vec<MemberResponse>>, ApiError> {
    if !can_administer(&ctx) {
        return Err(ApiError::Forbidden);
    }
    let members = state
        .storage
        .list_members(&ctx.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(members.into_iter().map(MemberResponse::from).collect()))
}

pub async fn invite_member(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Json(req): Json<InviteRequest>,
) -> Result<(StatusCode, Json<MemberResponse>), ApiError> {
    if !can_administer(&ctx) {
        return Err(ApiError::Forbidden);
    }

    let user = state
        .storage
        .get_user_by_username(&req.username)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("user '{}' not found", req.username)))?;

    // Conflict if already a member.
    if state
        .storage
        .get_member(&user.id, &ctx.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .is_some()
    {
        return Err(ApiError::Conflict("user is already a member".into()));
    }

    let role = parse_role(&req.role)?;
    let now = chrono::Utc::now();
    let member = llm_gateway_storage::Member {
        user_id: user.id.clone(),
        org_id: ctx.org_id.clone(),
        role,
        group_id: None,
        created_by: Some(ctx.user_id.clone()),
        created_at: now,
    };
    let saved = state
        .storage
        .upsert_member(member)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(build_response(saved, user.username)),
    ))
}

pub async fn update_member(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path((_org_slug, user_id)): Path<(String, String)>,
    Json(body): Json<UpdateMemberBody>,
) -> Result<Json<MemberResponse>, ApiError> {
    if !can_administer(&ctx) {
        return Err(ApiError::Forbidden);
    }

    // Resolve target membership up front (also used for last-owner guard).
    let existing = state
        .storage
        .get_member(&user_id, &ctx.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("member '{}' not found", user_id)))?;

    // Validate role string up front (matches existing parse_role error behavior).
    let new_role = match body.role.as_deref() {
        Some(r) => Some(parse_role(r)?),
        None => None,
    };

    // Last-owner guard — preserve from change_member_role. Same TOCTOU caveat
    // (count_owners and update run as separate statements; acceptable while
    // admin actions are rare and human-driven).
    if existing.role == MemberRole::Owner
        && matches!(new_role, Some(ref r) if *r != MemberRole::Owner)
    {
        let owners = state
            .storage
            .count_owners(&ctx.org_id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        if owners <= 1 {
            return Err(ApiError::BadRequest(
                "cannot remove or demote the last owner of an org".into(),
            ));
        }
    }

    // Apply user-row update if `enabled` was provided.
    if let Some(enabled) = body.enabled {
        let mut user = state
            .storage
            .get_user(&user_id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("User '{user_id}' not found")))?;
        user.enabled = enabled;
        user.updated_at = chrono::Utc::now();
        state
            .storage
            .update_user(&user)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
    }

    // Apply member-row update if `role` or `group_id` was provided.
    if new_role.is_some() || body.group_id.is_some() {
        let mut member = existing.clone();
        if let Some(r) = new_role.clone() {
            member.role = r;
        }
        if let Some(gid_opt) = body.group_id.clone() {
            // Validate group exists in this org before assigning (matches the
            // old update_user handler's behavior in users.rs).
            if let Some(ref gid) = gid_opt {
                let exists = state
                    .storage
                    .get_group(&ctx.org_id, gid)
                    .await
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
                if exists.is_none() {
                    return Err(ApiError::BadRequest(format!(
                        "Group '{}' not found",
                        gid
                    )));
                }
            }
            member.group_id = gid_opt;
        }
        state
            .storage
            .upsert_member(member)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
    }

    // Re-fetch the enriched view for the response.
    let members = state
        .storage
        .list_members(&ctx.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let updated = members
        .into_iter()
        .find(|m| m.user_id == user_id)
        .ok_or_else(|| ApiError::NotFound("Member not found after update".into()))?;
    Ok(Json(MemberResponse::from(updated)))
}

pub async fn remove_member(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path((_org_slug, user_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    // Self-leave is allowed for any member; removing another requires admin+.
    let is_self = ctx.user_id == user_id;
    if !is_self && !can_administer(&ctx) {
        return Err(ApiError::Forbidden);
    }

    let existing = state
        .storage
        .get_member(&user_id, &ctx.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("member '{}' not found", user_id)))?;

    // Last-owner guard: removing the only owner would orphan the org.
    // NOTE: TOCTOU — see update_member for the same caveat.
    if existing.role == MemberRole::Owner {
        let owners = state
            .storage
            .count_owners(&ctx.org_id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        if owners <= 1 {
            return Err(ApiError::BadRequest(
                "cannot remove or demote the last owner of an org".into(),
            ));
        }
    }

    state
        .storage
        .delete_member(&user_id, &ctx.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
