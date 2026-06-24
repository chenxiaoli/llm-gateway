use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use llm_gateway_storage::{CreateGroup, DeleteGroupResult, Group, PaginatedResponse, PaginationParams, UpdateGroup};
use serde::Serialize;
use std::sync::Arc;

use crate::error::ApiError;
use crate::extractors::require_admin;
use crate::AppState;

#[derive(Serialize)]
pub struct GroupResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Group> for GroupResponse {
    fn from(g: Group) -> Self {
        GroupResponse {
            id: g.id,
            name: g.name,
            description: g.description,
            created_at: g.created_at.to_rfc3339(),
            updated_at: g.updated_at.to_rfc3339(),
        }
    }
}

pub async fn list_groups(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<GroupResponse>>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;
    let (page, page_size) = pagination.normalized();
    let result = state
        .storage
        .list_groups_paginated(page, page_size)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(PaginatedResponse {
        items: result.items.into_iter().map(GroupResponse::from).collect(),
        total: result.total,
        page: result.page,
        page_size: result.page_size,
    }))
}

pub async fn get_group(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<GroupResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;
    let group = state
        .storage
        .get_group(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Group '{}' not found", id)))?;
    Ok(Json(GroupResponse::from(group)))
}

pub async fn create_group(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreateGroup>,
) -> Result<Json<GroupResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;
    let group = state
        .storage
        .create_group(&input)
        .await
        .map_err(|e| match e.downcast_ref::<sqlx::Error>() {
            Some(sqlx::Error::Database(db)) if db.is_unique_violation() => {
                ApiError::Conflict(format!("Group '{}' already exists", input.name))
            }
            _ => ApiError::Internal(e.to_string()),
        })?;
    Ok(Json(GroupResponse::from(group)))
}

pub async fn update_group(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<UpdateGroup>,
) -> Result<Json<GroupResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;
    let group = state
        .storage
        .update_group(&id, &input)
        .await
        .map_err(|e| match e.downcast_ref::<sqlx::Error>() {
            Some(sqlx::Error::Database(db)) if db.is_unique_violation() => {
                ApiError::Conflict("Group name already exists".to_string())
            }
            Some(sqlx::Error::RowNotFound) => {
                ApiError::NotFound(format!("Group '{}' not found", id))
            }
            _ => ApiError::Internal(e.to_string()),
        })?;
    Ok(Json(GroupResponse::from(group)))
}

pub async fn delete_group(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<DeleteGroupResult>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;
    let result = state
        .storage
        .delete_group(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(result))
}