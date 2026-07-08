use axum::extract::State;
use axum::Json;
use chrono::{DateTime, Utc};
use std::sync::Arc;

use llm_gateway_org::{can_administer, OrgContext};

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
