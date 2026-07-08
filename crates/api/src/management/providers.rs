use axum::extract::{Path, State};
use axum::Json;
use std::sync::Arc;

use llm_gateway_org::{can_create_org_catalog, can_create_platform_catalog, OrgContext};
use llm_gateway_storage::{CreateProvider as StorageCreateProvider, Provider, ProviderWithEndpoints, UpdateProvider as StorageUpdateProvider};

use crate::error::ApiError;
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
    ctx: OrgContext,
    Json(input): Json<StorageCreateProvider>,
) -> Result<Json<ProviderWithEndpoints>, ApiError> {
    // Decide ownership: explicit body value wins, otherwise non-platform-admin
    // callers get an org-private entry; platform_admin defaults to platform-level.
    let owner_org_id = input.owner_org_id.clone().or_else(|| {
        if can_create_platform_catalog(&ctx) { None } else { Some(ctx.org_id.clone()) }
    });
    if owner_org_id.as_deref() == Some(ctx.org_id.as_str()) {
        if !can_create_org_catalog(&ctx) {
            return Err(ApiError::Forbidden);
        }
    } else if owner_org_id.is_none() {
        if !can_create_platform_catalog(&ctx) {
            return Err(ApiError::Forbidden);
        }
    } else {
        // Caller tried to attach the entry to a different org.
        return Err(ApiError::Forbidden);
    }

    let now = chrono::Utc::now();
    let slug = input.slug.unwrap_or_else(|| make_slug(&input.name));
    let provider = Provider {
        id: uuid::Uuid::new_v4().to_string(),
        owner_org_id,
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
        .create_provider(&ctx.org_id, &provider)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(created.into()))
}

pub async fn list_providers(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
) -> Result<Json<Vec<ProviderWithEndpoints>>, ApiError> {
    let providers = state
        .storage
        .list_providers(&ctx.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let with_endpoints: Vec<ProviderWithEndpoints> = providers.into_iter().map(|p| p.into()).collect();
    Ok(Json(with_endpoints))
}

pub async fn get_provider(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path((_org_slug, id)): Path<(String, String)>,
) -> Result<Json<ProviderWithEndpoints>, ApiError> {
    let provider = state
        .storage
        .get_provider(&ctx.org_id, &id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Provider '{}' not found", id)))?;

    Ok(Json(provider.into()))
}

pub async fn update_provider(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path((_org_slug, id)): Path<(String, String)>,
    Json(input): Json<StorageUpdateProvider>,
) -> Result<Json<ProviderWithEndpoints>, ApiError> {
    let mut provider = state
        .storage
        .get_provider(&ctx.org_id, &id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Provider '{}' not found", id)))?;

    // Ownership check before mutating.
    if !llm_gateway_org::can_mutate_catalog_entry(&ctx, provider.owner_org_id.as_deref()) {
        return Err(ApiError::Forbidden);
    }

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
        .update_provider(&ctx.org_id, &provider)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(updated.into()))
}

pub async fn delete_provider(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path((_org_slug, id)): Path<(String, String)>,
) -> Result<axum::http::StatusCode, ApiError> {
    let provider = state
        .storage
        .get_provider(&ctx.org_id, &id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Provider '{}' not found", id)))?;

    if !llm_gateway_org::can_mutate_catalog_entry(&ctx, provider.owner_org_id.as_deref()) {
        return Err(ApiError::Forbidden);
    }

    state
        .storage
        .delete_provider(&ctx.org_id, &id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
