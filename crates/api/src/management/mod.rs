pub mod auth;
pub mod channels;
pub mod keys;
pub mod providers;
pub mod models;
pub mod usage;
pub mod logs;
pub mod users;
pub mod settings;
pub mod channel_models;
pub mod pricing_policies;
pub mod seed;

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
            "/api/v1/admin/providers",
            post(providers::create_provider).get(providers::list_providers),
        )
        .route(
            "/api/v1/admin/providers/{id}",
            get(providers::get_provider).patch(providers::update_provider).delete(providers::delete_provider),
        )
        .route(
            "/api/v1/admin/providers/{id}/channels",
            post(channels::create_channel).get(channels::list_channels),
        )
        .route(
            "/api/v1/admin/channels",
            get(channels::list_all_channels),
        )
        .route(
            "/api/v1/admin/channels/{id}",
            get(channels::get_channel).patch(channels::update_channel).delete(channels::delete_channel),
        )
        .route(
            "/api/v1/admin/models",
            get(models::list_all_models).post(models::create_model_global),
        )
        .route(
            "/api/v1/admin/providers/{id}/models",
            get(models::list_models).post(models::create_model),
        )
        .route(
            "/api/v1/admin/providers/{id}/models/{model_name}",
            patch(models::update_model).delete(models::delete_model),
        )
        .route(
            "/api/v1/admin/providers/{id}/sync-models",
            post(models::sync_models),
        )
        // ChannelModels (admin)
        .route(
            "/api/v1/admin/providers/{provider_id}/channel-models",
            post(channel_models::create_channel_model).get(channel_models::list_channel_models),
        )
        .route(
            "/api/v1/admin/channel-models/{id}",
            get(channel_models::get_channel_model).patch(channel_models::update_channel_model).delete(channel_models::delete_channel_model),
        )
        // Usage (authenticated)
        .route("/api/v1/usage", get(usage::get_usage))
        // Logs (admin)
        .route("/api/v1/admin/logs", get(logs::get_logs))
        // Users (admin)
        .route("/api/v1/admin/users", get(users::list_users))
        .route(
            "/api/v1/admin/users/{id}",
            patch(users::update_user).delete(users::delete_user),
        )
        // Settings (admin)
        .route("/api/v1/admin/settings", get(settings::get_settings).patch(settings::update_settings))
        // Seed data (reads static JSON)
        .route("/api/v1/admin/seed", get(seed::get_seed_data))
        // Pricing Policies (admin)
        .route(
            "/api/v1/admin/pricing-policies",
            post(pricing_policies::create).get(pricing_policies::list),
        )
        .route(
            "/api/v1/admin/pricing-policies/{id}",
            get(pricing_policies::get).patch(pricing_policies::update).delete(pricing_policies::delete),
        )
        // Version (public)
        .route("/api/v1/version", get(version))
}

async fn version(State(_state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": option_env!("GIT_VERSION").unwrap_or(env!("CARGO_PKG_VERSION")),
    }))
}
