use axum::extract::{Path, Query, State};
use axum::Json;
use llm_gateway_org::{can_manage_channels, OrgContext};
use llm_gateway_storage::{CreateGroup, DeleteGroupResult, Group, PaginatedResponse, PaginationParams, UpdateGroup};
use serde::Serialize;
use std::sync::Arc;

use crate::error::ApiError;
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
    ctx: OrgContext,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<GroupResponse>>, ApiError> {
    let (page, page_size) = pagination.normalized();
    let result = state
        .storage
        .list_groups_paginated(&ctx.org_id, page, page_size)
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
    ctx: OrgContext,
    Path((_org_slug, id)): Path<(String, String)>,
) -> Result<Json<GroupResponse>, ApiError> {
    let group = state
        .storage
        .get_group(&ctx.org_id, &id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Group '{}' not found", id)))?;
    Ok(Json(GroupResponse::from(group)))
}

pub async fn create_group(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Json(input): Json<CreateGroup>,
) -> Result<Json<GroupResponse>, ApiError> {
    if !can_manage_channels(&ctx) {
        return Err(ApiError::Forbidden);
    }
    // Force org_id to the caller's org regardless of what the body says.
    let input = CreateGroup {
        org_id: ctx.org_id.clone(),
        name: input.name,
        description: input.description,
    };
    let group = state
        .storage
        .create_group(&ctx.org_id, &input)
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
    ctx: OrgContext,
    Path((_org_slug, id)): Path<(String, String)>,
    Json(input): Json<UpdateGroup>,
) -> Result<Json<GroupResponse>, ApiError> {
    if !can_manage_channels(&ctx) {
        return Err(ApiError::Forbidden);
    }
    let group = state
        .storage
        .update_group(&ctx.org_id, &id, &input)
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
    ctx: OrgContext,
    Path((_org_slug, id)): Path<(String, String)>,
) -> Result<Json<DeleteGroupResult>, ApiError> {
    if !can_manage_channels(&ctx) {
        return Err(ApiError::Forbidden);
    }
    let result = state
        .storage
        .delete_group(&ctx.org_id, &id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(result))
}
