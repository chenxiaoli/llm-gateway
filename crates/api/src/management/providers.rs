use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use std::sync::Arc;

use llm_gateway_storage::{CreateProvider as StorageCreateProvider, Provider, ProviderWithEndpoints, UpdateProvider as StorageUpdateProvider};

use crate::error::ApiError;
use crate::extractors::require_admin;
use crate::AppState;

/// Generate slug from provider name
fn make_slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub async fn create_provider(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<StorageCreateProvider>,
) -> Result<Json<Provider>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let now = chrono::Utc::now();
    let slug = input.slug.unwrap_or_else(|| make_slug(&input.name));
    let provider = Provider {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name,
        slug,
        endpoints: input.endpoints.and_then(|v| {
            if v.is_null() { None } else { Some(v.to_string()) }
        }),
        proxy_url: input.proxy_url,
        enabled: true,
        created_at: now,
        updated_at: now,
    };

    let created = state
        .storage
        .create_provider(&provider)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(created))
}

pub async fn list_providers(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ProviderWithEndpoints>>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let providers = state
        .storage
        .list_providers()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let with_endpoints: Vec<ProviderWithEndpoints> = providers.into_iter().map(|p| p.into()).collect();
    Ok(Json(with_endpoints))
}

pub async fn get_provider(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ProviderWithEndpoints>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let provider = state
        .storage
        .get_provider(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Provider '{}' not found", id)))?;

    Ok(Json(provider.into()))
}

pub async fn update_provider(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<StorageUpdateProvider>,
) -> Result<Json<Provider>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let mut provider = state
        .storage
        .get_provider(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Provider '{}' not found", id)))?;

    // Apply partial updates
    if let Some(name) = input.name {
        provider.name = name;
    }
    if let Some(endpoints) = input.endpoints {
        provider.endpoints = endpoints.and_then(|v| {
            if v.is_null() { None } else { Some(v.to_string()) }
        });
    }
    if let Some(proxy_url) = input.proxy_url {
        provider.proxy_url = proxy_url;
    }
    if let Some(enabled) = input.enabled {
        provider.enabled = enabled;
    }
    provider.updated_at = chrono::Utc::now();

    let updated = state
        .storage
        .update_provider(&provider)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(updated))
}

pub async fn delete_provider(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    state
        .storage
        .delete_provider(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
