use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use llm_gateway_auth::{hash_password, verify_password, create_jwt, create_refresh_jwt, verify_refresh_jwt, validate_password, validate_username};
use llm_gateway_storage::User;

use crate::error::ApiError;
use crate::extractors::require_auth;
use crate::AppState;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub refresh_token: String,
    pub user: UserInfo,
}

#[derive(Serialize, Clone)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub role: String,
}

#[derive(Serialize)]
pub struct MeResponse {
    pub id: String,
    pub username: String,
    pub role: String,
    pub allow_registration: bool,
}

#[derive(Serialize)]
pub struct AuthConfigResponse {
    pub allow_registration: bool,
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Serialize)]
pub struct RefreshResponse {
    pub token: String,
    pub refresh_token: String,
}

impl From<&User> for UserInfo {
    fn from(u: &User) -> Self {
        UserInfo {
            id: u.id.clone(),
            username: u.username.clone(),
            role: u.role.clone(),
        }
    }
}

async fn get_allow_registration(state: &AppState) -> bool {
    state
        .storage
        .get_setting("allow_registration")
        .await
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(true)
}

async fn store_refresh_token(state: &AppState, user: &User, refresh_jwt: &str) -> Result<(), ApiError> {
    let mut updated_user = user.clone();
    updated_user.refresh_token = Some(refresh_jwt.to_string());
    updated_user.updated_at = chrono::Utc::now();
    state
        .storage
        .update_user(&updated_user)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(())
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(input): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    validate_password(&input.password).map_err(ApiError::BadRequest)?;

    let user = state
        .storage
        .get_user_by_username(&input.username)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;

    if !user.enabled {
        return Err(ApiError::Unauthorized);
    }

    if !verify_password(&input.password, &user.password) {
        return Err(ApiError::Unauthorized);
    }

    let token = create_jwt(&user.id, &user.role, &state.jwt_secret)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let refresh_jwt = create_refresh_jwt(&user.id, &state.jwt_secret)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    store_refresh_token(&state, &user, &refresh_jwt).await?;

    Ok(Json(AuthResponse {
        token,
        refresh_token: refresh_jwt,
        user: UserInfo::from(&user),
    }))
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(input): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let allow_reg = get_allow_registration(&state).await;

    let user_count = state
        .storage
        .user_count()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let is_first_user = user_count == 0;

    if !is_first_user && !allow_reg {
        return Err(ApiError::Forbidden);
    }

    validate_username(&input.username).map_err(ApiError::BadRequest)?;
    validate_password(&input.password).map_err(ApiError::BadRequest)?;

    if state
        .storage
        .get_user_by_username(&input.username)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .is_some()
    {
        return Err(ApiError::BadRequest("Username already exists".to_string()));
    }

    let now = chrono::Utc::now();
    let role = if is_first_user { "admin" } else { "user" };
    let user = User {
        id: uuid::Uuid::new_v4().to_string(),
        username: input.username.clone(),
        password: hash_password(&input.password).map_err(|e| ApiError::Internal(e.to_string()))?,
        role: role.to_string(),
        enabled: true,
        refresh_token: None,
        created_at: now,
        updated_at: now,
    };

    state
        .storage
        .create_user(&user)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Auto-create account for new user
    let account = llm_gateway_storage::Account {
        id: uuid::Uuid::new_v4().to_string(),
        user_id: user.id.clone(),
        balance: 0.0,
        threshold: 1.0,
        currency: "USD".to_string(),
        created_at: now,
        updated_at: now,
    };
    state
        .storage
        .create_account(&account)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let token = create_jwt(&user.id, &user.role, &state.jwt_secret)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let refresh_jwt = create_refresh_jwt(&user.id, &state.jwt_secret)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    store_refresh_token(&state, &user, &refresh_jwt).await?;

    Ok(Json(AuthResponse {
        token,
        refresh_token: refresh_jwt,
        user: UserInfo::from(&user),
    }))
}

pub async fn me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<MeResponse>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let user = state
        .storage
        .get_user(&claims.sub)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;

    let allow_reg = get_allow_registration(&state).await;

    Ok(Json(MeResponse {
        id: user.id,
        username: user.username,
        role: user.role,
        allow_registration: allow_reg,
    }))
}

use llm_gateway_storage::{PaginatedResponse, TransactionResponse};

pub async fn me_balance(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(pagination): Query<llm_gateway_storage::PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let account = state
        .storage
        .get_account_by_user_id(&claims.sub)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound("Account not found".to_string()))?;

    let (page, page_size) = pagination.normalized();
    let transactions = state
        .storage
        .list_transactions(&account.id, page, page_size)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "balance": account.balance,
        "threshold": account.threshold,
        "currency": account.currency,
        "transactions": PaginatedResponse {
            items: transactions.items.iter().map(TransactionResponse::from).collect(),
            total: transactions.total,
            page: transactions.page,
            page_size: transactions.page_size,
        }
    })))
}

pub async fn auth_config(
    State(state): State<Arc<AppState>>,
) -> Result<Json<AuthConfigResponse>, ApiError> {
    let allow_reg = get_allow_registration(&state).await;

    Ok(Json(AuthConfigResponse {
        allow_registration: allow_reg,
    }))
}

pub async fn refresh(
    State(state): State<Arc<AppState>>,
    Json(input): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, ApiError> {
    // Verify the refresh token JWT
    let claims = verify_refresh_jwt(&input.refresh_token, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    // Look up user by id from claims
    let user = state
        .storage
        .get_user(&claims.sub)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;

    if !user.enabled {
        return Err(ApiError::Unauthorized);
    }

    // Issue new access token
    let new_token = create_jwt(&user.id, &user.role, &state.jwt_secret)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Issue new refresh token (rotation)
    let new_refresh_jwt = create_refresh_jwt(&user.id, &state.jwt_secret)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Atomically rotate: only succeeds if stored token matches
    let rotated = state
        .storage
        .rotate_refresh_token(&user.id, &input.refresh_token, &new_refresh_jwt)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    if !rotated {
        return Err(ApiError::Unauthorized);
    }

    Ok(Json(RefreshResponse {
        token: new_token,
        refresh_token: new_refresh_jwt,
    }))
}

pub async fn change_password(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<ChangePasswordRequest>,
) -> Result<Json<UserInfo>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let user = state
        .storage
        .get_user(&claims.sub)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;

    if !verify_password(&input.current_password, &user.password) {
        return Err(ApiError::BadRequest("Current password is incorrect".to_string()));
    }

    validate_password(&input.new_password).map_err(ApiError::BadRequest)?;

    let mut updated_user = user.clone();
    updated_user.password = hash_password(&input.new_password).map_err(|e| ApiError::Internal(e.to_string()))?;
    updated_user.updated_at = chrono::Utc::now();
    state
        .storage
        .update_user(&updated_user)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(UserInfo::from(&updated_user)))
}
