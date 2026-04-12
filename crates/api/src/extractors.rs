use axum::http::HeaderMap;
use crate::error::ApiError;

pub fn extract_bearer_token(headers: &HeaderMap) -> Result<String, ApiError> {
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;
    if !auth.starts_with("Bearer ") {
        return Err(ApiError::Unauthorized);
    }
    Ok(auth[7..].to_string())
}

pub fn verify_admin_token(headers: &HeaderMap, expected_token: &str) -> Result<(), ApiError> {
    let token = extract_bearer_token(headers)?;
    if token != expected_token {
        return Err(ApiError::Unauthorized);
    }
    Ok(())
}
