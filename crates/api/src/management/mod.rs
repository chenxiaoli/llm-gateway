pub mod keys;
pub mod providers;
pub mod models;
pub mod usage;
pub mod logs;

use axum::routing::{get, patch, post};
use axum::Router;
use std::sync::Arc;
use crate::AppState;

pub fn management_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/keys", post(keys::create_key).get(keys::list_keys))
        .route(
            "/api/v1/keys/{id}",
            get(keys::get_key).patch(keys::update_key).delete(keys::delete_key),
        )
        .route(
            "/api/v1/providers",
            post(providers::create_provider).get(providers::list_providers),
        )
        .route(
            "/api/v1/providers/{id}",
            get(providers::get_provider).patch(providers::update_provider).delete(providers::delete_provider),
        )
        .route(
            "/api/v1/providers/{id}/models",
            post(models::create_model),
        )
        .route(
            "/api/v1/providers/{id}/models/{model_name}",
            patch(models::update_model).delete(models::delete_model),
        )
        .route("/api/v1/usage", get(usage::get_usage))
        .route("/api/v1/logs", get(logs::get_logs))
}
