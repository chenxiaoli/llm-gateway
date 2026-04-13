pub mod auth;
pub mod channels;
pub mod keys;
pub mod providers;
pub mod models;
pub mod usage;
pub mod logs;
pub mod users;
pub mod settings;

use axum::extract::State;
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use std::sync::Arc;
use crate::AppState;

pub fn management_router() -> Router<Arc<AppState>> {
    Router::new()
        // Auth (public)
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/config", get(auth::auth_config))
        .route("/api/v1/auth/me", get(auth::me))
        .route("/api/v1/auth/refresh", post(auth::refresh))
        .route("/api/v1/auth/change-password", post(auth::change_password))
        // Keys (authenticated)
        .route("/api/v1/keys", post(keys::create_key).get(keys::list_keys))
        .route(
            "/api/v1/keys/{id}",
            get(keys::get_key).patch(keys::update_key).delete(keys::delete_key),
        )
        // Providers (admin)
        .route(
            "/api/v1/providers",
            post(providers::create_provider).get(providers::list_providers),
        )
        .route(
            "/api/v1/providers/{id}",
            get(providers::get_provider).patch(providers::update_provider).delete(providers::delete_provider),
        )
        .route(
            "/api/v1/providers/{id}/channels",
            post(channels::create_channel).get(channels::list_channels),
        )
        .route(
            "/api/v1/channels/{id}",
            get(channels::get_channel).patch(channels::update_channel).delete(channels::delete_channel),
        )
        .route(
            "/api/v1/providers/{id}/models",
            get(models::list_models).post(models::create_model),
        )
        .route(
            "/api/v1/providers/{id}/models/{model_name}",
            patch(models::update_model).delete(models::delete_model),
        )
        // Usage (authenticated)
        .route("/api/v1/usage", get(usage::get_usage))
        // Logs (admin)
        .route("/api/v1/logs", get(logs::get_logs))
        // Users (admin)
        .route("/api/v1/users", get(users::list_users))
        .route(
            "/api/v1/users/{id}",
            patch(users::update_user).delete(users::delete_user),
        )
        // Settings (admin)
        .route("/api/v1/settings", get(settings::get_settings).patch(settings::update_settings))
        // Version (public)
        .route("/api/v1/version", get(version))
}

async fn version(State(_state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": option_env!("GIT_VERSION").unwrap_or(env!("CARGO_PKG_VERSION")),
    }))
}
