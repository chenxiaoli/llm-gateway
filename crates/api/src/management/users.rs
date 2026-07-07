use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use llm_gateway_org::{can_manage_channels, resolve_org_context};
use llm_gateway_storage::{units_to_usd, PaginatedResponse, PaginationParams, UpdateUser as StorageUpdateUser, User, UserWithBalance};
use serde::Serialize;
use std::sync::Arc;

use crate::error::ApiError;
use crate::extractors::require_auth;
use crate::AppState;

#[derive(Serialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub role: String,
    pub enabled: bool,
    pub group_id: Option<String>,
    pub group_name: Option<String>,
    pub balance: f64,
    pub threshold: f64,
    pub created_at: String,
    pub updated_at: String,
}

impl From<UserWithBalance> for UserResponse {
    fn from(u: UserWithBalance) -> Self {
        UserResponse {
            id: u.id,
            username: u.username,
            role: u.role,
            enabled: u.enabled,
            group_id: u.group_id,
            group_name: u.group_name,
            balance: units_to_usd(u.balance),
            threshold: units_to_usd(u.threshold),
            created_at: u.created_at.to_rfc3339(),
            updated_at: u.updated_at.to_rfc3339(),
        }
    }
}

pub async fn list_users(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<UserResponse>>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;
    let ctx = resolve_org_context(&claims, state.storage.as_ref()).await?;
    if !can_manage_channels(&ctx) {
        return Err(ApiError::Forbidden);
    }
    let (page, page_size) = pagination.normalized();
    let result = state.storage.list_users_paginated(&ctx.org_id, page, page_size).await.map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(PaginatedResponse {
        items: result.items.into_iter().map(UserResponse::from).collect(),
        total: result.total,
        page: result.page,
        page_size: result.page_size,
    }))
}

pub async fn update_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<StorageUpdateUser>,
) -> Result<Json<UserResponse>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;
    let ctx = resolve_org_context(&claims, state.storage.as_ref()).await?;
    if !can_manage_channels(&ctx) {
        return Err(ApiError::Forbidden);
    }

    let mut user = state.storage.get_user(&id).await.map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("User '{}' not found", id)))?;

    // TODO(Task 11/12): the `role` and `group_id` fields in UpdateUser target
    // the membership layer (members.role / members.group_id), not the users
    // table. Phase 1 storage doesn't yet expose methods to mutate those, so we
    // accept the fields in the request body but only apply `enabled` here.
    // The frontend will be updated in Task 11 to call dedicated membership
    // endpoints once they exist.
    if let Some(enabled) = input.enabled { user.enabled = enabled; }
    user.updated_at = chrono::Utc::now();

    let updated = state.storage.update_user(&user).await.map_err(|e| ApiError::Internal(e.to_string()))?;

    // Re-fetch the synthesized view row so the response carries the
    // membership-derived role/group_name fields the frontend expects.
    let users = state.storage.list_users_paginated(&ctx.org_id, 1, i64::MAX).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let uwb = users.items.into_iter().find(|u| u.id == updated.id)
        .unwrap_or_else(|| UserWithBalance {
            id: updated.id.clone(),
            username: updated.username.clone(),
            role: "member".into(),
            enabled: updated.enabled,
            group_id: None,
            group_name: None,
            balance: 0,
            threshold: 0,
            created_at: updated.created_at,
            updated_at: updated.updated_at,
        });
    Ok(Json(UserResponse::from(uwb)))
}

pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;
    let ctx = resolve_org_context(&claims, state.storage.as_ref()).await?;
    if !can_manage_channels(&ctx) {
        return Err(ApiError::Forbidden);
    }

    // TODO(Task 11/12): the old `count_admin_users` check is gone — Task 4
    // removed it because role moved from users to members. Phase 1 simplifies
    // and trusts the can_manage_channels gate; a follow-up should re-introduce
    // a "last owner" guard via count_owners once delete_user also tears down
    // the membership row.
    let _ = state.storage.get_user(&id).await.map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("User '{}' not found", id)))?;

    state.storage.delete_user(&id).await.map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
