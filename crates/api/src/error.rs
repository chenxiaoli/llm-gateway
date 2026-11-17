use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug)]
pub enum ApiError {
    Unauthorized,
    Forbidden,
    RateLimited,
    PaymentRequired,
    NotFound(String),
    BadRequest(String),
    Conflict(String),
    UpstreamError(u16, String),
    Internal(String),
}

impl ApiError {
    /// The human-readable message the client sees for this error.
    /// Used by the proxy to quote an upstream failure in the fallback chain.
    pub fn message(&self) -> String {
        match self {
            ApiError::Unauthorized => "Unauthorized".to_string(),
            ApiError::Forbidden => "Forbidden".to_string(),
            ApiError::RateLimited => "Rate limit exceeded".to_string(),
            ApiError::PaymentRequired => "Insufficient balance".to_string(),
            ApiError::NotFound(msg) => msg.clone(),
            ApiError::BadRequest(msg) => msg.clone(),
            ApiError::Conflict(msg) => msg.clone(),
            ApiError::UpstreamError(_, msg) => msg.clone(),
            ApiError::Internal(msg) => msg.clone(),
        }
    }

    /// The HTTP status code this error maps to.
    pub fn status_code(&self) -> u16 {
        match self {
            ApiError::Unauthorized => StatusCode::UNAUTHORIZED.as_u16(),
            ApiError::Forbidden => StatusCode::FORBIDDEN.as_u16(),
            ApiError::RateLimited => StatusCode::TOO_MANY_REQUESTS.as_u16(),
            ApiError::PaymentRequired => StatusCode::PAYMENT_REQUIRED.as_u16(),
            ApiError::NotFound(_) => StatusCode::NOT_FOUND.as_u16(),
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST.as_u16(),
            ApiError::Conflict(_) => StatusCode::CONFLICT.as_u16(),
            ApiError::UpstreamError(code, _) => *code,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden"),
            ApiError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded"),
            ApiError::PaymentRequired => (StatusCode::PAYMENT_REQUIRED, "Insufficient balance"),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.as_str()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.as_str()),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, msg.as_str()),
            ApiError::UpstreamError(code, msg) => (
                StatusCode::from_u16(*code).unwrap_or(StatusCode::BAD_GATEWAY),
                msg.as_str(),
            ),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.as_str()),
        };
        let body = json!({ "error": { "message": message, "type": status.as_u16() } });
        (status, axum::Json(body)).into_response()
    }
}
