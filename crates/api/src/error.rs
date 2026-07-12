use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug)]
pub enum ApiError {
    Unauthorized,
    Forbidden,
    RateLimited { retry_after_secs: i64 },
    BudgetExceeded {
        key_id: String,
        month_bucket: String,
        limit_units: i64,    // 10^8 subunits per USD
        accrued_units: i64,  // 10^8 subunits per USD
    },
    PaymentRequired,
    NotFound(String),
    BadRequest(String),
    Conflict(String),
    Gone(String),
    UpstreamError(u16, String),
    Internal(String),

    // --- Phase 4: typed variants carrying a stable error code ---
    // Frontend branches on the `code` field; keep codes in sync with the spec
    // Section 8.2. Each variant maps to a fixed HTTP status + code + message.
    EmailRequired,              // 400 email_required
    EmailInUse,                 // 409 email_in_use
    EmailMismatchRegister,      // 400 email_mismatch (register via invite)
    EmailMismatchAccept,        // 403 email_mismatch (accept invite)
    EmailNotVerified,           // 403 email_not_verified (login gate)
    EmailVerificationRequired,  // 403 email_verification_required (accept gate)
    VerificationExpired,        // 410 verification_expired
    VerificationNotFound,       // 404 verification_not_found
    ResetExpired,               // 410 reset_expired
    ResetConsumed,              // 410 reset_consumed
    ResetNotFound,              // 404 reset_not_found
    LastPlatformAdmin,          // 409 last_platform_admin
}

impl From<llm_gateway_org::OrgError> for ApiError {
    fn from(e: llm_gateway_org::OrgError) -> Self {
        match e {
            llm_gateway_org::OrgError::NotFound(msg) => ApiError::NotFound(msg),
            llm_gateway_org::OrgError::NotMember(_user, _org) => {
                // Membership failures are authz rejections — surface as 403
                // rather than leaking which user/org was probed.
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
        // RateLimited is the only variant that emits an HTTP header (Retry-After),
        // so it can't share the flat (status, message, code) tuple path below.
        if let ApiError::RateLimited { retry_after_secs } = self {
            let body = axum::Json(json!({
                "error": {
                    "message": "Rate limit exceeded",
                    "type": StatusCode::TOO_MANY_REQUESTS.as_u16(),
                }
            }));
            let mut resp = (StatusCode::TOO_MANY_REQUESTS, body).into_response();
            resp.headers_mut().insert(
                "retry-after",
                axum::http::HeaderValue::from_str(&retry_after_secs.to_string())
                    .expect("retry_after_secs fits in a HeaderValue"),
            );
            return resp;
        }

        // BudgetExceeded carries a structured payload (key, bucket, USD figures),
        // so it doesn't fit the flat (status, message, code) path below.
        if let ApiError::BudgetExceeded {
            key_id,
            month_bucket,
            limit_units,
            accrued_units,
        } = self
        {
            let limit_usd = llm_gateway_storage::units_to_usd(limit_units);
            let accrued_usd = llm_gateway_storage::units_to_usd(accrued_units);
            let body = json!({
                "error": {
                    "type": "budget_exceeded",
                    "message": format!(
                        "Monthly budget exceeded. Spend: ${accrued_usd:.2} / Limit: ${limit_usd:.2}. Month: {month_bucket}."
                    ),
                    "key_id": key_id,
                    "month_bucket": month_bucket,
                    "limit": limit_usd,
                    "accrued": accrued_usd,
                }
            });
            return (StatusCode::TOO_MANY_REQUESTS, axum::Json(body)).into_response();
        }

        // (status, message, code) — code is a short stable string the frontend
        // can branch on. None for legacy variants keeps the existing JSON shape.
        // No explicit type annotation: the message borrows from `self` for the
        // String-carrying variants (NotFound, BadRequest, etc.), so we let the
        // compiler infer a single common `&str` lifetime across all arms.
        let (status, message, code) = match &self {
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized", None),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden", None),
            ApiError::PaymentRequired => (StatusCode::PAYMENT_REQUIRED, "Insufficient balance", None),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.as_str(), None),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.as_str(), None),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, msg.as_str(), None),
            ApiError::Gone(msg) => (StatusCode::GONE, msg.as_str(), None),
            ApiError::UpstreamError(code, msg) => (
                StatusCode::from_u16(*code).unwrap_or(StatusCode::BAD_GATEWAY),
                msg.as_str(),
                None,
            ),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.as_str(), None),

            ApiError::EmailRequired => (StatusCode::BAD_REQUEST, "Email is required", Some("email_required")),
            ApiError::EmailInUse => (StatusCode::CONFLICT, "Email is already in use", Some("email_in_use")),
            ApiError::EmailMismatchRegister => (
                StatusCode::BAD_REQUEST,
                "Email does not match the invitation recipient",
                Some("email_mismatch"),
            ),
            ApiError::EmailMismatchAccept => (
                StatusCode::FORBIDDEN,
                "This invitation was sent to a different address",
                Some("email_mismatch"),
            ),
            ApiError::EmailNotVerified => (
                StatusCode::FORBIDDEN,
                "Please verify your email before logging in",
                Some("email_not_verified"),
            ),
            ApiError::EmailVerificationRequired => (
                StatusCode::FORBIDDEN,
                "Verify your email first",
                Some("email_verification_required"),
            ),
            ApiError::VerificationExpired => (
                StatusCode::GONE,
                "This verification link has expired",
                Some("verification_expired"),
            ),
            ApiError::VerificationNotFound => (
                StatusCode::NOT_FOUND,
                "Verification token not found",
                Some("verification_not_found"),
            ),
            ApiError::ResetExpired => (
                StatusCode::GONE,
                "This password reset link has expired",
                Some("reset_expired"),
            ),
            ApiError::ResetConsumed => (
                StatusCode::GONE,
                "This password reset link has already been used",
                Some("reset_consumed"),
            ),
            ApiError::ResetNotFound => (
                StatusCode::NOT_FOUND,
                "Password reset token not found",
                Some("reset_not_found"),
            ),
            ApiError::LastPlatformAdmin => (
                StatusCode::CONFLICT,
                "Cannot demote the last platform admin",
                Some("last_platform_admin"),
            ),
            // Handled by the early-return above; unreachable here.
            ApiError::RateLimited { .. } => unreachable!("RateLimited handled above"),
            ApiError::BudgetExceeded { .. } => unreachable!("BudgetExceeded handled above"),
        };
        let body = if let Some(c) = code {
            json!({ "error": { "message": message, "type": status.as_u16(), "code": c } })
        } else {
            json!({ "error": { "message": message, "type": status.as_u16() } })
        };
        (status, axum::Json(body)).into_response()
    }
}
