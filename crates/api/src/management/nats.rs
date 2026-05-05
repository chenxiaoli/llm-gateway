use crate::AppState;
use crate::error::ApiError;
use crate::extractors::require_admin;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
pub struct NatsStatusResponse {
    pub streams: Vec<llm_gateway_nats_publisher::StreamStatusInfo>,
}

pub async fn get_nats_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<NatsStatusResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let mut streams = Vec::new();
    for name in &["LLM_GATEWAY_USAGE", "LLM_GATEWAY_AUDIT"] {
        match state.nats_publisher.stream_info(name).await {
            Ok(info) => streams.push(info),
            Err(e) => {
                return Err(ApiError::Internal(format!(
                    "NATS stream '{}' error: {}",
                    name, e
                )))
            }
        }
    }
    Ok(Json(NatsStatusResponse { streams }))
}
