use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::error::ApiError;
use crate::extractors::require_platform_admin;
use crate::AppState;
use llm_gateway_storage::types::{PlatformRole, SetPlatformRoleError};

#[derive(Debug, Deserialize)]
pub struct PatchPlatformRoleBody {
    /// `"platform_admin"` to grant, `null` to revoke.
    pub platform_role: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct ListPlatformUsersQuery {
    /// Optional substring search for non-admin users by username/email.
    /// When present, the response includes a `candidates` array alongside
    /// `admins`. The frontend's "Add platform admin" search box passes this.
    pub q: Option<String>,
}

fn brief(u: llm_gateway_storage::User) -> serde_json::Value {
    serde_json::json!({
        "id": u.id,
        "username": u.username,
        "email": u.email,
        "platform_role": u.platform_role,
    })
}

pub async fn list_platform_users(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListPlatformUsersQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let _claims = require_platform_admin(&headers, &state.jwt_secret)?;
    let admins = state
        .storage
        .list_platform_admins()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let admins_json: Vec<_> = admins.into_iter().map(brief).collect();

    // Only do the candidate search when the query is non-empty. Empty
    // string would match every non-admin user in the DB.
    let candidates_json: Vec<serde_json::Value> = match query.q.as_deref() {
        Some(q) if !q.is_empty() => {
            let hits = state
                .storage
                .search_user_candidates(q)
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;
            hits.into_iter().map(brief).collect()
        }
        _ => Vec::new(),
    };

    Ok(Json(serde_json::json!({
        "admins": admins_json,
        "candidates": candidates_json,
    })))
}

pub async fn patch_platform_role(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<PatchPlatformRoleBody>,
) -> Result<impl IntoResponse, ApiError> {
    let claims = require_platform_admin(&headers, &state.jwt_secret)?;
    let role: Option<PlatformRole> = match body.platform_role.as_deref() {
        Some("platform_admin") => Some(PlatformRole::PlatformAdmin),
        None => None,
        Some(other) => {
            return Err(ApiError::BadRequest(format!(
                "unknown platform_role value: {other}"
            )));
        }
    };
    // Note: allow_last_admin_override is hard-coded false here. The CLI gets
    // the override; the API path does not, by design.
    state
        .storage
        .set_user_platform_role(&user_id, &claims.sub, role, false)
        .await
        .map_err(|e| match e {
            SetPlatformRoleError::UserNotFound => ApiError::NotFound("user not found".into()),
            SetPlatformRoleError::LastPlatformAdmin => ApiError::LastPlatformAdmin,
            SetPlatformRoleError::Database(_) => ApiError::Internal(e.to_string()),
        })?;
    Ok(Json(serde_json::json!({"id": user_id, "platform_role": body.platform_role})))
}
