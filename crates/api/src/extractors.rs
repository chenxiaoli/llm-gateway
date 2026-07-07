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

/// Returns the JWT claims only if the bearer is a platform_admin.
///
/// Most handlers should prefer `resolve_org_context` + a `can_*` check
/// (per-org admin = owner/admin member role). This helper is reserved for
/// the few platform-only operations that have no per-org analogue in
/// Phase 1 (model_fallbacks, NATS status).
pub fn require_platform_admin(headers: &HeaderMap, jwt_secret: &str) -> Result<JwtClaims, ApiError> {
    let claims = require_auth(headers, jwt_secret)?;
    if claims.platform_role.as_deref() != Some("platform_admin") {
        return Err(ApiError::Forbidden);
    }
    Ok(claims)
}
