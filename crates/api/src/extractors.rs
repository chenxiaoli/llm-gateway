use axum::http::HeaderMap;
use crate::error::ApiError;
use llm_gateway_auth::verify_jwt;
use llm_gateway_auth::JwtClaims;

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

pub fn require_auth(headers: &HeaderMap, jwt_secret: &str) -> Result<JwtClaims, ApiError> {
    let token = extract_bearer_token(headers)?;
    let claims = verify_jwt(&token, jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;
    Ok(claims)
}

pub fn require_admin(headers: &HeaderMap, jwt_secret: &str) -> Result<JwtClaims, ApiError> {
    let claims = require_auth(headers, jwt_secret)?;
    if claims.role != "admin" {
        return Err(ApiError::Forbidden);
    }
    Ok(claims)
}
