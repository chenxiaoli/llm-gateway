use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Serialize;
use std::sync::Arc;

use llm_gateway_storage::{units_to_usd, PaginatedResponse, PaginationParams, UpdateUser as StorageUpdateUser, User, UserWithBalance};

use crate::error::ApiError;
use crate::extractors::require_admin;
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

impl From<&User> for UserResponse {
    fn from(u: &User) -> Self {
        UserResponse {
            id: u.id.clone(),
            username: u.username.clone(),
            role: u.role.clone(),
            enabled: u.enabled,
            group_id: u.group_id.clone(),
            group_name: None,
            balance: 0.0,
            threshold: 1.0,
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
    require_admin(&headers, &state.jwt_secret)?;
    let (page, page_size) = pagination.normalized();
    let result = state.storage.list_users_paginated(page, page_size).await.map_err(|e| ApiError::Internal(e.to_string()))?;
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
    require_admin(&headers, &state.jwt_secret)?;

    let mut user = state.storage.get_user(&id).await.map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("User '{}' not found", id)))?;

    if let Some(false) = input.enabled {
        if user.role == "admin" {
            let admin_count = state.storage.count_admin_users().await.map_err(|e| ApiError::Internal(e.to_string()))?;
            if admin_count <= 1 {
                return Err(ApiError::BadRequest("Cannot disable the last admin user".to_string()));
            }
        }
    }

    if let Some(ref role) = input.role {
        if user.role == "admin" && role != "admin" {
            let admin_count = state.storage.count_admin_users().await.map_err(|e| ApiError::Internal(e.to_string()))?;
            if admin_count <= 1 {
                return Err(ApiError::BadRequest("Cannot demote the last admin user".to_string()));
            }
        }
        user.role = role.clone();
    }

    if let Some(enabled) = input.enabled { user.enabled = enabled; }

    if let Some(ref group_id) = input.group_id {
        if let Some(gid) = group_id {
            // Validate the group exists
            let exists = state.storage.get_group(gid).await
                .map_err(|e| ApiError::Internal(e.to_string()))?;
            if exists.is_none() {
                return Err(ApiError::BadRequest(format!("Group '{}' not found", gid)));
            }
        }
        user.group_id = group_id.clone();
    }

    user.updated_at = chrono::Utc::now();

    let updated = state.storage.update_user(&user).await.map_err(|e| ApiError::Internal(e.to_string()))?;
    let mut resp = UserResponse::from(&updated);
    if let Some(ref gid) = resp.group_id {
        if let Some(g) = state.storage.get_group(gid).await
            .map_err(|e| ApiError::Internal(e.to_string()))? {
            resp.group_name = Some(g.name);
        }
    }
    Ok(Json(resp))
}

pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let user = state.storage.get_user(&id).await.map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("User '{}' not found", id)))?;

    if user.role == "admin" {
        let admin_count = state.storage.count_admin_users().await.map_err(|e| ApiError::Internal(e.to_string()))?;
        if admin_count <= 1 {
            return Err(ApiError::BadRequest("Cannot delete the last admin user".to_string()));
        }
    }

    state.storage.delete_user(&id).await.map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
