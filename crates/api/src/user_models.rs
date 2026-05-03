use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use std::sync::Arc;

use llm_gateway_storage::{UserModelView, UserPricingInfo};

use crate::error::ApiError;
use crate::extractors::require_auth;
use crate::AppState;

/// GET /api/v1/user/models — list models for console users (JWT auth)
pub async fn list_user_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<UserModelView>>, ApiError> {
    let _claims = require_auth(&headers, &state.jwt_secret)?;

    let models = state
        .storage
        .list_models()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let mut views: Vec<UserModelView> = Vec::new();
    for m in models {
        let channel_models = state
            .storage
            .get_channel_models_for_model(&m.model.id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        let is_available = channel_models.iter().any(|cm| cm.enabled);

        let (pricing_policy_name, pricing) = match &m.model.pricing_policy_id {
            Some(policy_id) => {
                let policy = state
                    .storage
                    .get_pricing_policy(policy_id)
                    .await
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
                match policy {
                    Some(p) => (
                        Some(p.name),
                        Some(UserPricingInfo {
                            billing_type: p.billing_type,
                            config: p.config,
                        }),
                    ),
                    None => (m.pricing_policy_name.clone(), None),
                }
            }
            None => (None, None),
        };

        views.push(UserModelView {
            name: m.model.name,
            model_type: m.model.model_type,
            pricing_policy_name,
            pricing,
            is_available,
            created_at: m.model.created_at.to_rfc3339(),
        });
    }

    let live: Vec<UserModelView> = views.into_iter().filter(|v| v.is_available).collect();

    Ok(Json(live))
}
