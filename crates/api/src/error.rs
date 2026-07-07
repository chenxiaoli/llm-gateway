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

impl From<llm_gateway_org::OrgError> for ApiError {
    fn from(e: llm_gateway_org::OrgError) -> Self {
        match e {
            llm_gateway_org::OrgError::NotFound(msg) => ApiError::NotFound(msg),
            llm_gateway_org::OrgError::NotMember(user, org) => {
                ApiError::Forbidden
            }
            llm_gateway_org::OrgError::Forbidden(_) => ApiError::Forbidden,
            llm_gateway_org::OrgError::SlugTaken(_) => ApiError::Conflict(e.to_string()),
            llm_gateway_org::OrgError::LastOwner(_) => ApiError::BadRequest(e.to_string()),
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
