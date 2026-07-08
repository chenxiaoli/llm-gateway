use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::sync::Arc;

use llm_gateway_org::{can_administer, OrgContext};
use llm_gateway_storage::MemberRole;

use crate::error::ApiError;
use crate::AppState;

/// A membership row joined with the username from `users`.
///
/// The frontend expects `role` as the lowercase string form
/// (`"owner" | "admin" | "member"`), matching `MemberRole::as_str`.
#[derive(serde::Serialize)]
pub struct MemberResponse {
    pub user_id: String,
    pub username: String,
    pub role: String,
    pub group_id: Option<String>,
    pub joined_at: DateTime<Utc>,
}

fn build_response(member: llm_gateway_storage::Member, username: String) -> MemberResponse {
    MemberResponse {
        user_id: member.user_id,
        username,
        role: member.role.as_str().to_string(),
        group_id: member.group_id,
        joined_at: member.created_at,
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
pub struct ChangeRoleRequest {
    pub role: String,
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

    // Join username per member. Member counts are small (tens, not thousands),
    // so the N queries are cheaper than a JOIN-on-storage-trait surface.
    let mut out = Vec::with_capacity(members.len());
    for m in members {
        let username = match state
            .storage
            .get_user(&m.user_id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
        {
            Some(u) => u.username,
            None => {
                // Orphan membership row (user deleted but member row left
                // behind). Skip rather than 500 — listing should be resilient.
                continue;
            }
        };
        out.push(build_response(m, username));
    }
    Ok(Json(out))
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

pub async fn change_member_role(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path((_org_slug, user_id)): Path<(String, String)>,
    Json(req): Json<ChangeRoleRequest>,
) -> Result<Json<MemberResponse>, ApiError> {
    if !can_administer(&ctx) {
        return Err(ApiError::Forbidden);
    }

    let existing = state
        .storage
        .get_member(&user_id, &ctx.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("member '{}' not found", user_id)))?;

    let username = state
        .storage
        .get_user(&user_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .map(|u| u.username)
        .unwrap_or_default();

    let new_role = parse_role(&req.role)?;

    // Last-owner guard: demoting the only owner would orphan the org.
    // NOTE: TOCTOU — count_owners and update_member_role run as separate
    // statements. Two concurrent demotions of the last two owners could
    // both pass. Acceptable while admin actions are rare and human-driven;
    // revisit with a row-locked transaction or SQL-conditional UPDATE if
    // this is ever exposed to automation.
    if existing.role == MemberRole::Owner && new_role != MemberRole::Owner {
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
        .update_member_role(&user_id, &ctx.org_id, new_role.clone())
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Compose response from the row we already have + the new role; no
    // need to re-fetch — update_member_role is a no-op on other fields.
    let updated = llm_gateway_storage::Member {
        role: new_role,
        ..existing
    };
    Ok(Json(build_response(updated, username)))
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
    // NOTE: TOCTOU — see change_member_role for the same caveat.
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
