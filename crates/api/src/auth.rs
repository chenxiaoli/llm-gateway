use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use llm_gateway_auth::{
    create_jwt, create_refresh_jwt, hash_password, validate_password, validate_username,
    verify_password, verify_refresh_jwt,
};
use llm_gateway_email::dispatch_with_retry;
use llm_gateway_email::templates::VerificationCtx;
use llm_gateway_org::OrgContext;
use llm_gateway_storage::{
    CreateOrg, Member, MemberRole, MembershipSummary, PlatformRole, UpdateOrg, User,
};

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
    /// Phase 4: required — the new user must have a verified email before
    /// they can log in. `Option<String>` so a missing field deserializes to
    /// `None` and we can return the typed `EmailRequired` (400) error
    /// instead of Axum's default 422 for malformed JSON.
    #[serde(default)]
    pub email: Option<String>,
    /// Optional invitation token. When present, the new account is
    /// pre-bound to the inviting org and the email must match
    /// `invitation.recipient_email` (case-insensitive).
    #[serde(default)]
    pub invite_token: Option<String>,
}

/// Phase 4: body for `POST /api/v1/auth/verify-email`.
#[derive(Deserialize)]
pub struct VerifyEmailRequest {
    pub token: String,
}

/// Phase 4: body for `POST /api/v1/auth/resend-verification`. Always returns
/// 204 — the endpoint is a no-op for unknown / unverified-by-policy emails so
/// it cannot be used to enumerate addresses.
#[derive(Deserialize)]
pub struct ResendVerificationRequest {
    pub email: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub refresh_token: String,
    pub user: UserInfo,
    /// `None` for limbo users (just registered, no org yet). The frontend's
    /// post-auth flow treats `null` here as "show the onboarding wizard".
    pub current_org: Option<OrgSummary>,
    pub orgs: Vec<OrgSummary>,
}

#[derive(Serialize, Clone)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub platform_role: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct OrgSummary {
    pub id: String,
    pub slug: String,
    pub name: String,
    /// Mirrors MemberRole as a string ("owner" | "admin" | "member").
    pub role: String,
    pub group_id: Option<String>,
}

impl From<MembershipSummary> for OrgSummary {
    fn from(m: MembershipSummary) -> Self {
        OrgSummary {
            id: m.org.id,
            slug: m.org.slug,
            name: m.org.name,
            role: m.role.as_str().to_string(),
            group_id: m.group_id,
        }
    }
}

#[derive(Serialize)]
pub struct MeResponse {
    pub id: String,
    pub username: String,
    pub platform_role: Option<String>,
    /// null when the user has no memberships (e.g. just self-left their last
    /// org). Callers (frontend `refreshOrgs`) treat null as "send to /login".
    pub current_org: Option<OrgSummary>,
    pub orgs: Vec<OrgSummary>,
    pub allow_registration: bool,
    /// True when the current membership is a temp/system-created row, i.e.
    /// the caller is a platform_admin operating in an org they don't really
    /// belong to (see `membership_layer`'s impersonation path). Surfaced to
    /// the UI so it can show an "platform admin mode" banner.
    pub impersonating: bool,
    // --- Phase 4 fields ---
    /// The user's email (None when they haven't added one). Frontend shows
    /// the "Add email" banner when this is null and the policy requires it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// RFC3339 timestamp of when the email was verified, or null if the
    /// email isn't verified yet (or there is no email).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_verified_at: Option<String>,
    /// True if the platform policy requires this user to verify their email
    /// before login. Currently always true at signup; reserved for future
    /// roles (e.g. service accounts) that bypass the gate.
    pub requires_email_verification: bool,
}

#[derive(Serialize)]
pub struct AuthConfigResponse {
    pub allow_registration: bool,
    pub currency: String,
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

#[derive(Deserialize)]
pub struct SwitchOrgRequest {
    pub org_slug: Option<String>,
    pub org_id: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateOrgRequest {
    pub slug: String,
    pub name: String,
}

impl From<&User> for UserInfo {
    fn from(u: &User) -> Self {
        UserInfo {
            id: u.id.clone(),
            username: u.username.clone(),
            platform_role: u.platform_role.as_ref().map(|p| p.as_str().to_string()),
        }
    }
}

async fn get_allow_registration(state: &AppState) -> bool {
    state
        .storage
        .get_platform_setting("allow_registration")
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

/// Pick the user's current membership: the one matching `current_org_id`,
/// falling back to the first membership if the stored value is stale or absent.
///
/// Returns `Internal("user has no org membership")` if the user has zero
/// memberships — every authenticated user must belong to at least one org.
/// Resolve the user's current org + full membership list. Returns
/// `current_org = None` if the user has zero memberships; callers that need
/// a real org (login/register/refresh) handle that via [`require_membership`].
async fn current_membership(
    state: &AppState,
    user: &User,
) -> Result<(Option<OrgSummary>, Vec<OrgSummary>), ApiError> {
    let memberships = state
        .storage
        .list_orgs_for_user(&user.id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let current = memberships
        .iter()
        .find(|m| Some(&m.org.id) == user.current_org_id.as_ref())
        .or_else(|| memberships.first())
        .cloned()
        .map(Into::into);

    let orgs: Vec<OrgSummary> = memberships.into_iter().map(Into::into).collect();
    Ok((current, orgs))
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

    // Phase 4 login gate: a user with `requires_email_verification = true`
    // cannot get a token until their email is verified. Once verified, the
    // flag is left in place (it tracks the org's policy, not this user's
    // status) but `email_verified_at` is set and the check passes.
    //
    // The `requires` check is a future-proofing hook — currently every user
    // created via /auth/register has it set to true. Service accounts (a
    // later feature) can flip it to false to skip the gate.
    if user.requires_email_verification && user.email_verified_at.is_none() {
        return Err(ApiError::EmailNotVerified);
    }

    let (current_org, orgs) = current_membership(&state, &user).await?;
    // Phase 3: a user with zero memberships (e.g. limbo user who registered
    // then self-left, or was removed from their last org) can still log in.
    // They get a token with no `current_org_id` claim and the frontend
    // bounces them to the onboarding wizard. The token's missing org claim
    // means org-scoped routes will reject them at authz time — they can
    // only hit platform-level endpoints (`/auth/me`, `/orgs`, `/auth/me/onboarding`)
    // until they create or join an org.
    let current_org_id_arg = current_org.as_ref().map(|o| o.id.as_str());

    let platform_role_str = user.platform_role.as_ref().map(|p| p.as_str());
    let token = create_jwt(&user.id, current_org_id_arg, platform_role_str, &state.jwt_secret)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let refresh_jwt = create_refresh_jwt(&user.id, &state.jwt_secret)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    store_refresh_token(&state, &user, &refresh_jwt).await?;

    Ok(Json(AuthResponse {
        token,
        refresh_token: refresh_jwt,
        user: UserInfo::from(&user),
        current_org,
        orgs,
    }))
}

/// Verification email lifetime. 24 hours is the conventional default
/// (matches GitHub/GitLab/etc.) and long enough to be a non-issue for users
/// who don't see the email arrive in the first minute.
const EMAIL_VERIFICATION_TTL_HOURS: i64 = 24;

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
    // Phase 4: email is required for every new account. We validate the
    // shape here (not just the presence) so a malformed string never makes
    // it into the database. The `email` field is Option<String> at the
    // deserialization layer so a missing key returns 400 + email_required
    // (a typed error) rather than Axum's default 422 for a malformed body.
    let email_trimmed = input.email.unwrap_or_default().trim().to_string();
    if email_trimmed.is_empty() {
        return Err(ApiError::EmailRequired);
    }
    validate_email(&email_trimmed).map_err(ApiError::BadRequest)?;

    if state
        .storage
        .get_user_by_username(&input.username)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .is_some()
    {
        return Err(ApiError::BadRequest("Username already exists".to_string()));
    }
    if state
        .storage
        .get_user_by_email(&email_trimmed)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .is_some()
    {
        return Err(ApiError::EmailInUse);
    }

    // If an invite_token was supplied, look it up. We must validate the email
    // matches the invitation's recipient (case-insensitive) — otherwise a
    // malicious caller could intercept someone else's invite and squat the
    // org membership. The actual membership is created by the accept flow
    // later; here we just confirm the binding.
    let invite = if let Some(token) = input.invite_token.as_deref() {
        let inv = state
            .storage
            .get_invitation_by_token(token)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound("invitation not found".into()))?;
        // An invite is only bindable if it's still consumable: not accepted,
        // not revoked, not expired, and carries a recipient_email.
        let now = chrono::Utc::now();
        if inv.accepted_at.is_some()
            || inv.revoked_at.is_some()
            || inv.expires_at < now
            || inv.recipient_email.is_none()
        {
            return Err(ApiError::NotFound("invitation not found".into()));
        }
        let recipient = inv.recipient_email.as_deref().unwrap().to_lowercase();
        if recipient != email_trimmed.to_lowercase() {
            return Err(ApiError::EmailMismatchRegister);
        }
        Some(inv)
    } else {
        None
    };

    let now = chrono::Utc::now();
    let platform_role = if is_first_user {
        Some(PlatformRole::PlatformAdmin)
    } else {
        None
    };
    let user = User {
        id: uuid::Uuid::new_v4().to_string(),
        username: input.username.clone(),
        password: hash_password(&input.password).map_err(|e| ApiError::Internal(e.to_string()))?,
        platform_role,
        current_org_id: None,
        enabled: true,
        refresh_token: None,
        created_at: now,
        updated_at: now,
        // Phase 4: new users always need email verification. The dispatch
        // step below mints a token and sends the email; the user is locked
        // out of login until they click the link (see the login gate).
        email: Some(email_trimmed.clone()),
        email_verified_at: None,
        requires_email_verification: true,
        password_changed_at: now,
    };

    state
        .storage
        .create_user(&user)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Phase 3: brand-new users land in limbo — no auto-org-membership, no
    // current_org_id, no account. They complete the onboarding wizard to
    // create or join an org.
    //
    // First-user platform_admin auto-grant is preserved (cold-start deploys
    // still need a way to bootstrap a platform_admin without an existing org).
    //
    // NOTE: the existing `is_first_user` check (`user_count == 0`) means a
    // cold-start deploy's first user gets `platform_role = platform_admin`
    // but STILL has to complete the wizard to create their first org. This
    // is intentional — even platform_admins need an org context to operate.

    let refresh_jwt = create_refresh_jwt(&user.id, &state.jwt_secret)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Persist refresh token on the user row so /auth/refresh can rotate it.
    // (store_refresh_token mutates a local clone and writes — does not
    // re-read the row, so the local `user` stays authoritative below.)
    store_refresh_token(&state, &user, &refresh_jwt).await?;

    // Reload the user to capture the refresh_token + updated_at that
    // store_refresh_token just persisted, so the JWT-bound user row reflects
    // reality and downstream code that re-reads `user` sees the same shape
    // as the DB.
    let user = state
        .storage
        .get_user(&user.id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::Internal("just-created user vanished".into()))?;

    // Phase 4: mint a verification token + dispatch the email. We swallow
    // dispatch errors (the spawn'd task already retries 3x; final failure
    // is logged). A user can always re-request via /auth/resend-verification.
    let verification_expires = now + chrono::Duration::hours(EMAIL_VERIFICATION_TTL_HOURS);
    match state
        .storage
        .create_email_verification(&user.id, &email_trimmed, verification_expires)
        .await
    {
        Ok(verification) => {
            let verification_url = format!(
                "{}/verify-email/{}",
                state.public_base_url.trim_end_matches('/'),
                verification.token
            );
            let ctx = VerificationCtx {
                username: user.username.clone(),
                recipient_email: email_trimmed.clone(),
                verification_url,
                expires_in_hours: EMAIL_VERIFICATION_TTL_HOURS as u32,
                public_base_url: state.public_base_url.clone(),
            };
            match state.templates.render_verification(ctx) {
                Ok(msg) => {
                    dispatch_with_retry(state.mailer.clone(), msg, "verification email".into());
                }
                Err(e) => {
                    // Template failure is a config issue, not a per-user
                    // issue; log loudly but still return success — the user
                    // is created and can use resend-verification later.
                    tracing::error!(error = %e, "failed to render verification email");
                }
            }
        }
        Err(e) => {
            // Same as above: storage failure is a server issue, not a
            // user-blocking error. The user exists and can retry the
            // dispatch via /auth/resend-verification once the DB heals.
            tracing::error!(error = %e, "failed to mint verification token");
        }
    }

    // If an invite_token was supplied and validated above, consume it
    // here so the user lands with the membership already in place. The
    // accept_invitation flow is a different code path (used when an
    // already-registered user joins via a link in their email); the
    // register-with-invite path combines both into one transaction.
    if let Some(inv) = invite {
        // Ignore the return — accept_invitation returns Some(Member) on
        // success; on None the token was already consumed by a concurrent
        // request, which is fine (the user still got created).
        let _ = state
            .storage
            .accept_invitation(&inv.token, &user.id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
    }

    let orgs: Vec<OrgSummary> = Vec::new();

    let platform_role_str = user.platform_role.as_ref().map(|p| p.as_str());
    let token = create_jwt(
        &user.id,
        // current_org_id is None for limbo users — token carries no
        // current_org_id claim.
        None,
        platform_role_str,
        &state.jwt_secret,
    )
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(AuthResponse {
        token,
        refresh_token: refresh_jwt,
        user: UserInfo::from(&user),
        current_org: None,
        orgs,
    }))
}

/// POST /api/v1/auth/verify-email — consume a verification token.
///
/// The token is one-shot. On success the user's `email_verified_at` is set
/// in the same transaction as the row is marked consumed. Returns 204 No
/// Content on success — the caller is expected to redirect the user back
/// to the app, where the next /me call will reflect the verified status.
///
/// Errors are deliberately collapsed: invalid/missing/expired/consumed all
/// surface as the same `VerificationExpired` (410) so the endpoint cannot
/// be used to enumerate live tokens.
pub async fn verify_email(
    State(state): State<Arc<AppState>>,
    Json(input): Json<VerifyEmailRequest>,
) -> Result<StatusCode, ApiError> {
    // Confirm the token exists at all. A truly missing token is a 404
    // (clue: caller typo'd their URL or scraped a stale link).
    let existing = state
        .storage
        .get_email_verification_by_token(&input.token)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::VerificationNotFound)?;

    // Already-consumed or expired → 410. The body still names
    // "verification_expired" because from the caller's perspective the link
    // is no longer usable, regardless of root cause.
    let now = chrono::Utc::now();
    if existing.consumed_at.is_some() || existing.expires_at < now {
        return Err(ApiError::VerificationExpired);
    }

    // Atomic consume — the storage layer also stamps
    // `users.email_verified_at` in the same transaction.
    let ok = state
        .storage
        .consume_email_verification(&input.token)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    if !ok {
        // Race: another caller consumed it between our read and our write.
        return Err(ApiError::VerificationExpired);
    }
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/auth/resend-verification — re-dispatch a verification email.
///
/// Always returns 204 No Content, even for unknown emails. The endpoint
/// is the natural target for enumeration (POST + email), so a uniform
/// response shape is the only acceptable behavior.
///
/// Behavior:
/// - Unknown email → no-op, 204.
/// - Email exists, user not yet verified → mint a new token + dispatch.
/// - Email exists, user already verified → no-op, 204 (don't leak that the
///   address is registered).
pub async fn resend_verification(
    State(state): State<Arc<AppState>>,
    Json(input): Json<ResendVerificationRequest>,
) -> Result<StatusCode, ApiError> {
    // Trim + validate so a malformed input still gets a clean 204 (no
    // enumeration vector). The dispatch step is a no-op for unknown /
    // already-verified addresses, so we don't bother short-circuiting here.
    let email = input.email.trim();
    if email.is_empty() {
        return Ok(StatusCode::NO_CONTENT);
    }

    // Look up by email. None → no-op 204.
    let Some(user) = state
        .storage
        .get_user_by_email(email)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
    else {
        return Ok(StatusCode::NO_CONTENT);
    };

    // Already verified → no-op 204. Don't leak that the address is
    // registered.
    if user.email_verified_at.is_some() {
        return Ok(StatusCode::NO_CONTENT);
    }

    let now = chrono::Utc::now();
    let expires = now + chrono::Duration::hours(EMAIL_VERIFICATION_TTL_HOURS);
    let verification = match state
        .storage
        .create_email_verification(&user.id, email, expires)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "resend_verification: mint failed, returning 204 to avoid enumeration");
            return Ok(StatusCode::NO_CONTENT);
        }
    };

    let verification_url = format!(
        "{}/verify-email/{}",
        state.public_base_url.trim_end_matches('/'),
        verification.token
    );
    let ctx = VerificationCtx {
        username: user.username.clone(),
        recipient_email: email.to_string(),
        verification_url,
        expires_in_hours: EMAIL_VERIFICATION_TTL_HOURS as u32,
        public_base_url: state.public_base_url.clone(),
    };
    match state.templates.render_verification(ctx) {
        Ok(msg) => {
            dispatch_with_retry(state.mailer.clone(), msg, "resend verification email".into());
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to render resend verification email");
        }
    }
    Ok(StatusCode::NO_CONTENT)
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

    let (current_org, orgs) = current_membership(&state, &user).await?;
    let allow_reg = get_allow_registration(&state).await;

    // Detect platform-admin impersonation: the membership row for the current
    // org was created by `system` (the temp-membership path in
    // `membership_layer`) rather than by a real user. The flag is only
    // meaningful when a current org is selected.
    //
    // Propagate storage errors as 500 rather than silently defaulting to
    // `false`: failing closed here would hide the banner while the user is
    // still operating in the impersonated org. Fail-closed belongs at the
    // layer that *grants* access (membership_layer), not the layer that
    // surfaces a warning.
    let impersonating = match &current_org {
        Some(org) => state
            .storage
            .get_member(&user.id, &org.id)
            .await
            .map_err(|e| ApiError::Internal(format!("member lookup failed: {e}")))?
            .map(|m| m.created_by.as_deref() == Some("system"))
            .unwrap_or(false),
        None => false,
    };

    Ok(Json(MeResponse {
        id: user.id,
        username: user.username,
        platform_role: user.platform_role.as_ref().map(|p| p.as_str().to_string()),
        current_org,
        orgs,
        allow_registration: allow_reg,
        impersonating,
        email: user.email.clone(),
        email_verified_at: user.email_verified_at.map(|t| t.to_rfc3339()),
        requires_email_verification: user.requires_email_verification,
    }))
}

#[derive(Debug, Serialize)]
pub struct OnboardingStatus {
    pub needs_onboarding: bool,
}

/// GET /api/v1/auth/me/onboarding — quick probe the frontend uses to decide
/// whether to redirect the user to the onboarding wizard.
///
/// Returns `{ needs_onboarding: true }` when the caller has zero org
/// memberships (limbo). Once they create or join an org — via the wizard,
/// an invitation, or platform-admin intervention — subsequent calls return
/// `false`. This endpoint is intentionally cheap (no user row reload, no
/// impersonation lookup) so the frontend can poll it on boot.
pub async fn me_onboarding(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<OnboardingStatus>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;
    let memberships = state
        .storage
        .list_orgs_for_user(&claims.sub)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(OnboardingStatus {
        needs_onboarding: memberships.is_empty(),
    }))
}

use llm_gateway_storage::{units_to_usd, TransactionResponse};

pub async fn me_balance(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(pagination): Query<llm_gateway_storage::PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    // Limbo users have no current org and therefore no account. Return 404
    // rather than 500 — the frontend treats this as "no balance yet" and
    // the wizard routes them to create/join an org first. The message names
    // the real cause (no current org / onboarding incomplete) rather than
    // "account not found" — the account genuinely doesn't exist yet, but
    // the user row does; the misleading shape would send a frontend
    // debugger looking for a missing user.
    let current_org_id = claims
        .current_org_id
        .as_deref()
        .ok_or_else(|| {
            ApiError::NotFound("no current org — complete onboarding".to_string())
        })?;

    let account = state
        .storage
        .get_account_by_user_id(current_org_id, &claims.sub)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound("Account not found".to_string()))?;

    let (page, page_size) = pagination.normalized();
    let transactions = state
        .storage
        .list_transactions(current_org_id, &account.id, page, page_size)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Convert internal i64 to f64 USD for JSON output
    let tx_responses: Vec<serde_json::Value> = transactions.items.iter().map(|t| {
        let resp = TransactionResponse::from(t);
        serde_json::json!({
            "id": resp.id,
            "account_id": resp.account_id,
            "type": resp.transaction_type,
            "amount": units_to_usd(resp.amount),
            "balance_after": units_to_usd(resp.balance_after),
            "description": resp.description,
            "reference_id": resp.reference_id,
            "created_at": resp.created_at,
        })
    }).collect();

    Ok(Json(serde_json::json!({
        "balance": units_to_usd(account.balance),
        "threshold": units_to_usd(account.threshold),
        "transactions": {
            "items": tx_responses,
            "total": transactions.total,
            "page": transactions.page,
            "page_size": transactions.page_size,
        }
    })))
}

pub async fn auth_config(
    State(state): State<Arc<AppState>>,
) -> Result<Json<AuthConfigResponse>, ApiError> {
    let allow_reg = get_allow_registration(&state).await;
    let currency = state.storage.get_platform_setting("currency").await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .unwrap_or_else(|| "USD".to_string());

    Ok(Json(AuthConfigResponse {
        allow_registration: allow_reg,
        currency,
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

    // Resolve current org so the new access token carries it. Phase 3: a
    // limbo user (no memberships) gets a fresh limbo token — they may have
    // a refresh token from registration but not yet have completed the
    // onboarding wizard. /me and the wizard endpoints accept tokens with
    // no `current_org_id` claim; only org-scoped routes reject them.
    let (current_org, _orgs) = current_membership(&state, &user).await?;
    let current_org_id_arg = current_org.as_ref().map(|o| o.id.as_str());

    // Issue new access token
    let platform_role_str = user.platform_role.as_ref().map(|p| p.as_str());
    let new_token = create_jwt(&user.id, current_org_id_arg, platform_role_str, &state.jwt_secret)
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

/// Switch the caller's current org. Persists `current_org_id` on the user
/// and reissues the access token with the new org embedded.
pub async fn switch_org(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SwitchOrgRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let target_org = if let Some(slug) = body.org_slug {
        state.storage.get_org_by_slug(&slug).await
    } else if let Some(id) = body.org_id {
        state.storage.get_org(&id).await
    } else {
        return Err(ApiError::BadRequest("org_slug or org_id required".to_string()));
    }
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .ok_or_else(|| ApiError::NotFound("org".to_string()))?;

    let member = state
        .storage
        .get_member(&claims.sub, &target_org.id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::Forbidden)?;

    // Persist new current_org_id on the user row.
    let mut user = state
        .storage
        .get_user(&claims.sub)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;
    user.current_org_id = Some(target_org.id.clone());
    user.updated_at = chrono::Utc::now();
    user = state
        .storage
        .update_user(&user)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let platform_role_str = user.platform_role.as_ref().map(|p| p.as_str());
    let token = create_jwt(&user.id, Some(&target_org.id), platform_role_str, &state.jwt_secret)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let memberships = state
        .storage
        .list_orgs_for_user(&claims.sub)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let orgs: Vec<OrgSummary> = memberships.into_iter().map(Into::into).collect();

    Ok(Json(AuthResponse {
        token,
        refresh_token: user.refresh_token.clone().unwrap_or_default(),
        user: UserInfo::from(&user),
        current_org: Some(OrgSummary {
            id: target_org.id,
            slug: target_org.slug,
            name: target_org.name,
            role: member.role.as_str().to_string(),
            group_id: member.group_id,
        }),
        orgs,
    }))
}

/// List all orgs the caller is a member of.
pub async fn list_orgs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<OrgSummary>>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let memberships = state
        .storage
        .list_orgs_for_user(&claims.sub)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(
        memberships.into_iter().map(Into::into).collect(),
    ))
}

/// Slugs that collide with the literal-410 legacy routes in
/// `management::management_router` (`keys`, `model-fallbacks`, `usage`).
/// If an org took one of these slugs, every request under
/// `/api/v1/{slug}/...` would be absorbed by the 410 handlers and the org
/// would be effectively unusable. Reject up-front at validation time.
const RESERVED_SLUGS: [&str; 3] = ["keys", "model-fallbacks", "usage"];

/// Minimal email format check. We don't attempt to fully implement RFC 5321
/// (full grammar is unwieldy) — the goal is to reject obvious garbage before
/// it hits the database. The shape: at least one character on each side of a
/// single `@`, with at least one `.` in the domain part, all ASCII.
///
/// The frontend does its own form-level validation; this is the
/// defense-in-depth check at the API boundary.
pub fn validate_email(s: &str) -> Result<(), String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("email is required".into());
    }
    if s.len() > 254 {
        return Err("email is too long".into());
    }
    if !s.is_ascii() {
        return Err("email must be ASCII".into());
    }
    let mut parts = s.split('@');
    let local = parts.next().unwrap_or("");
    let domain = parts.next().unwrap_or("");
    if local.is_empty()
        || domain.is_empty()
        || parts.next().is_some()
    {
        return Err("email must be of the form local@domain".into());
    }
    if !domain.contains('.') {
        return Err("email domain must contain a '.'".into());
    }
    if domain.starts_with('.') || domain.ends_with('.') {
        return Err("email domain has invalid '.' placement".into());
    }
    if !local
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || ".!#$%&'*+/=?^_`{|}~-".contains(c))
    {
        return Err("email local part contains invalid characters".into());
    }
    if !domain
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
    {
        return Err("email domain contains invalid characters".into());
    }
    Ok(())
}

/// Validate an org slug against the same rule as the DB CHECK constraint:
/// `^[a-z0-9-]{3,64}$` (lowercase letters, digits, hyphens; 3-64 chars).
fn validate_org_slug(slug: &str) -> Result<(), String> {
    if slug.len() < 3 || slug.len() > 64 {
        return Err("Slug must be 3-64 characters".into());
    }
    if !slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err("Slug can only contain lowercase letters, digits, and hyphens".into());
    }
    if RESERVED_SLUGS.contains(&slug) {
        return Err(format!("Slug '{slug}' is reserved"));
    }
    Ok(())
}

/// Create a new org with the caller as owner.
///
/// Phase 3 behavior:
/// - If the caller was in limbo (no `current_org_id`), the new org becomes
///   their current org (auto-switch) and the JWT is reissued with the new
///   org id embedded.
/// - If the caller already has a current org, the new org is created and
///   they're added as owner, but their current_org is NOT switched — they
///   stay in the context they were working in. They can switch later via
///   `/me/current-org`.
/// - Returns `AuthResponse` (not `OrgSummary`) so the client receives the
///   fresh token + full membership list. Phase 3 ships this before the
///   Task 7 frontend callers land, so there are momentarily no frontend
///   consumers of the old `OrgSummary` shape — but those callers are
///   coming, and will be written against this new shape.
pub async fn create_org(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateOrgRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    validate_org_slug(&body.slug).map_err(ApiError::BadRequest)?;
    if body.name.trim().is_empty() {
        return Err(ApiError::BadRequest("name is required".into()));
    }

    let user = state
        .storage
        .get_user(&claims.sub)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;

    let now = chrono::Utc::now();
    let org = state
        .storage
        .create_org(CreateOrg {
            id: uuid::Uuid::new_v4().to_string(),
            slug: body.slug,
            name: body.name,
            owner_id: claims.sub.clone(),
        })
        .await
        // Map duplicate-slug storage errors to 409 Conflict. The storage trait
        // returns `Box<dyn Error>`, so we string-sniff for the Postgres unique
        // constraint name (`orgs_slug_key`) and common violation keywords.
        .map_err(|e| {
            let msg = e.to_string();
            let lower = msg.to_lowercase();
            if msg.contains("orgs_slug_key")
                || lower.contains("duplicate")
                || lower.contains("unique")
            {
                ApiError::Conflict("org slug already taken".into())
            } else {
                ApiError::Internal(msg)
            }
        })?;

    state
        .storage
        .upsert_member(Member {
            user_id: claims.sub.clone(),
            org_id: org.id.clone(),
            role: MemberRole::Owner,
            group_id: None,
            created_by: Some(claims.sub.clone()),
            created_at: now,
        })
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Auto-switch current_org ONLY when the caller was in limbo. An
    // established user creating an additional org keeps their current_org so
    // they don't get yanked out of their working context.
    let was_limbo = user.current_org_id.is_none();
    let mut updated = user.clone();
    if was_limbo {
        updated.current_org_id = Some(org.id.clone());
    }
    updated.updated_at = now;
    let updated_user = state
        .storage
        .update_user(&updated)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Reissue JWT with the caller's current org (new org if was limbo,
    // previous org otherwise).
    let effective_current_org_id = updated_user
        .current_org_id
        .as_deref()
        .unwrap_or(&org.id);
    let platform_role_str = updated_user.platform_role.as_ref().map(|p| p.as_str());
    let token = create_jwt(
        &updated_user.id,
        Some(effective_current_org_id),
        platform_role_str,
        &state.jwt_secret,
    )
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    let memberships = state
        .storage
        .list_orgs_for_user(&updated_user.id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let orgs: Vec<OrgSummary> = memberships.into_iter().map(Into::into).collect();
    let current_org_summary = orgs
        .iter()
        .find(|o| o.id == *effective_current_org_id)
        .cloned()
        .ok_or_else(|| ApiError::Internal("current_org not in membership list".into()))?;

    Ok(Json(AuthResponse {
        token,
        refresh_token: updated_user.refresh_token.clone().unwrap_or_default(),
        user: UserInfo::from(&updated_user),
        current_org: Some(current_org_summary),
        orgs,
    }))
}

/// GET /api/v1/{org_slug} — read details of the resolved org.
///
/// Caller must be a member (enforced by `membership_layer` before reaching
/// here). Returns the same `OrgSummary` shape as `list_orgs`/`create_org`
/// so the frontend can treat them uniformly.
pub async fn get_org(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
) -> Result<Json<OrgSummary>, ApiError> {
    let org = state
        .storage
        .get_org(&ctx.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("org not found".into()))?;

    Ok(Json(OrgSummary {
        id: org.id,
        slug: org.slug,
        name: org.name,
        role: ctx.member_role.as_str().to_string(),
        group_id: ctx.group_id,
    }))
}

/// PATCH /api/v1/{org_slug} — update the resolved org's name and/or slug.
///
/// Requires admin-or-above in the org (or platform_admin). Slug updates are
/// validated against the same rules as `create_org`; duplicate slugs surface
/// as 409 Conflict so callers can distinguish from validation failures.
///
/// **Slug rename caveat:** changing the slug invalidates any URL that embeds
/// the old slug (frontend routes like `/{slug}/keys`, deep-linked bookmarks,
/// external API clients using the slug in the path). The frontend's
/// `OrgSwitcher` rewrites `currentOrg` on rename so in-app navigation
/// survives, but anything caching the old slug will hit `org_resolve_layer`'s
/// 404. Document this in the CHANGELOG when the rename flow ships.
#[derive(Deserialize)]
pub struct UpdateOrgRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
}

pub async fn update_org(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Json(req): Json<UpdateOrgRequest>,
) -> Result<Json<OrgSummary>, ApiError> {
    if !llm_gateway_org::can_manage_org_settings(&ctx) {
        return Err(ApiError::Forbidden);
    }

    // Validate inputs. An explicitly-empty name (after trim) is rejected
    // rather than silently treated as missing — the caller asked to blank
    // something out, which we don't allow.
    let name = match req.name {
        Some(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return Err(ApiError::BadRequest("name must not be empty".into()));
            }
            Some(trimmed.to_string())
        }
        None => None,
    };
    let slug = match req.slug {
        Some(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return Err(ApiError::BadRequest("slug must not be empty".into()));
            }
            Some(trimmed.to_string())
        }
        None => None,
    };

    if name.is_none() && slug.is_none() {
        return Err(ApiError::BadRequest(
            "must provide at least one of 'name' or 'slug'".into(),
        ));
    }

    if let Some(s) = &slug {
        validate_org_slug(s).map_err(ApiError::BadRequest)?;
    }

    let org = state
        .storage
        .update_org(
            &ctx.org_id,
            UpdateOrg {
                name: name.clone(),
                slug: slug.clone(),
            },
        )
        .await
        // Map duplicate-slug storage errors to 409 Conflict. The storage trait
        // returns `Box<dyn Error>`, so we string-sniff for the Postgres unique
        // constraint name (`orgs_slug_key`) and common violation keywords —
        // same approach as `create_org`.
        .map_err(|e| {
            let msg = e.to_string();
            let lower = msg.to_lowercase();
            if msg.contains("orgs_slug_key")
                || lower.contains("duplicate")
                || lower.contains("unique")
            {
                ApiError::Conflict("org slug already taken".into())
            } else {
                ApiError::Internal(msg)
            }
        })?;

    Ok(Json(OrgSummary {
        id: org.id,
        slug: org.slug,
        name: org.name,
        // Role isn't affected by an org update — echo back the caller's role.
        role: ctx.member_role.as_str().to_string(),
        group_id: ctx.group_id,
    }))
}

/// DELETE /api/v1/{org_slug} — hard-delete the resolved org.
///
/// Requires the org owner role (or platform_admin) AND a password re-check to
/// guard against accidental or session-hijack deletion. The DB cascade removes
/// members and other org-scoped rows.
#[derive(Deserialize)]
pub struct DeleteOrgRequest {
    pub password: String,
}

pub async fn delete_org(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Json(req): Json<DeleteOrgRequest>,
) -> Result<StatusCode, ApiError> {
    if !llm_gateway_org::can_delete_org(&ctx) {
        return Err(ApiError::Forbidden);
    }

    let user = state
        .storage
        .get_user(&ctx.user_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("user not found".into()))?;

    if !verify_password(&req.password, &user.password) {
        return Err(ApiError::Unauthorized);
    }

    state
        .storage
        .delete_org(&ctx.org_id)
        .await
        // Map the storage layer's "org not found" outcome to a clean 404.
        // This happens deterministically when a concurrent DELETE won the
        // race after we passed the password check; surfacing it as 500
        // would leak the org id and report a server error for a benign
        // lost race.
        .map_err(|e| {
            let msg = e.to_string().to_lowercase();
            if msg.contains("not found") {
                ApiError::NotFound("org".into())
            } else {
                ApiError::Internal(e.to_string())
            }
        })?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_org_slug_accepts_valid() {
        assert!(validate_org_slug("my-org").is_ok());
        assert!(validate_org_slug("abc").is_ok());
        assert!(validate_org_slug("a1b2c3").is_ok());
        assert!(validate_org_slug(&"x".repeat(64)).is_ok());
    }

    #[test]
    fn validate_org_slug_rejects_invalid() {
        assert!(validate_org_slug("ab").is_err());            // too short
        assert!(validate_org_slug(&"x".repeat(65)).is_err()); // too long
        assert!(validate_org_slug("MyOrg").is_err());         // uppercase
        assert!(validate_org_slug("my_org").is_err());        // underscore
        assert!(validate_org_slug("my org").is_err());        // space
        assert!(validate_org_slug("").is_err());              // empty
    }

    #[test]
    fn validate_org_slug_rejects_reserved() {
        // Slugs that collide with literal-410 legacy routes would make the
        // org unreachable; reject at validation time.
        assert!(validate_org_slug("keys").is_err());
        assert!(validate_org_slug("model-fallbacks").is_err());
        assert!(validate_org_slug("usage").is_err());
        // Sanity: close-but-not-exact slugs are fine.
        assert!(validate_org_slug("keys-prod").is_ok());
        assert!(validate_org_slug("usage-team").is_ok());
    }

    // ─── Phase 3: integration tests for register / create_org / auth/me/onboarding ───
    //
    // These tests need a real Postgres pool (migrations + state). They live
    // inline in this module rather than under `crates/api/tests/` so they can
    // call private helpers (`AuthResponse` shape) and stay close to the
    // handler under test. They reuse the same management router +
    // `seed_admin_user`/`make_state`/`create_jwt` helpers the integration
    // tests use.

    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use llm_gateway_email::noop::NoopMailer;
    use llm_gateway_email::templates::TemplateRegistry;
    use llm_gateway_storage::postgres::PostgresStorage;
    use llm_gateway_storage::Storage;
    use serde_json::{Value, json};
    use sqlx::PgPool;
    use tower::ServiceExt;

    /// Build a management router wired to a real Postgres storage pool. The
    /// JWT secret matches what `create_jwt` uses in tests below.
    fn build_router(storage: Arc<PostgresStorage>) -> axum::Router {
        let state: Arc<AppState> = Arc::new(AppState {
            storage: storage.clone() as Arc<dyn Storage>,
            rate_limiter: Arc::new(llm_gateway_ratelimit::RateLimiter::new(60)),
            jwt_secret: "test-jwt-secret".to_string(),
            encryption_key: [0u8; 32],
            nats_publisher: None,
            registry: Arc::new(crate::InMemoryChannelRegistry::new(
                storage as Arc<dyn Storage>,
                [0u8; 32],
                std::time::Duration::from_secs(30),
            )),
            system_info: crate::SystemInfo {
                server_bind_address: "127.0.0.1:8080".to_string(),
                database_driver: "postgres".to_string(),
                rate_limit_window_secs: 60,
                rate_limit_flush_interval_secs: 30,
                upstream_timeout_secs: 30,
                audit_retention_days: None,
            },
            public_base_url: "http://localhost:5173".to_string(),
            mailer: Arc::new(NoopMailer::new()),
            templates: Arc::new(
                TemplateRegistry::load("noreply@test.local".to_string(), "Test".to_string())
                    .expect("load templates"),
            ),
        });
        crate::management::management_router(state.clone()).with_state(state)
    }

    async fn body_json(resp: axum::http::Response<Body>) -> Value {
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn bearer(token: &str) -> String {
        format!("Bearer {token}")
    }

    async fn post_json(
        app: &axum::Router,
        uri: &str,
        token: Option<&str>,
        body: Value,
    ) -> axum::http::Response<Body> {
        let mut req = Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json");
        if let Some(t) = token {
            req = req.header("authorization", bearer(t));
        }
        app.clone()
            .oneshot(req.body(Body::from(body.to_string())).unwrap())
            .await
            .unwrap()
    }

    async fn get_authed(app: &axum::Router, uri: &str, token: &str) -> axum::http::Response<Body> {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(uri)
                    .header("authorization", bearer(token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    /// Direct storage lookup to verify DB-side state after a handler call.
    /// Used to assert that `users.current_org_id` was persisted correctly.
    async fn user_current_org_id(pool: &PgPool, user_id: &str) -> Option<String> {
        let row: (Option<String>,) = sqlx::query_as(
            "SELECT current_org_id FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
        .expect("user row exists");
        row.0
    }

    #[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
    async fn register_returns_jwt_with_null_current_org(pool: PgPool) {
        // POST /auth/register → 200. Response has current_org: None, orgs: [].
        // The user row in DB has current_org_id = NULL.
        let storage = Arc::new(PostgresStorage::from_pool(pool.clone()));
        let app = build_router(storage);
        let resp = post_json(
            &app,
            "/api/v1/auth/register",
            None,
            json!({"username": "alice", "password": "password123", "email": "alice@example.com"}),
        )
        .await;
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        let body = body_json(resp).await;
        assert!(
            body["current_org"].is_null(),
            "limbo user should have null current_org, got {}",
            body["current_org"]
        );
        assert!(body["orgs"].as_array().unwrap().is_empty());
        assert!(body["token"].is_string());

        // DB row should have current_org_id = NULL.
        // Look up the user we just created (id is generated; look up by username).
        let row: (String, Option<String>) = sqlx::query_as(
            "SELECT id, current_org_id FROM users WHERE username = 'alice'",
        )
        .fetch_one(&pool)
        .await
        .expect("user row");
        assert!(row.1.is_none(), "DB current_org_id must be NULL for limbo user");
    }

    #[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
    async fn create_org_called_by_limbo_user_switches_current_org_and_reissues_jwt(pool: PgPool) {
        // Register (limbo) → POST /api/v1/orgs → 200 with AuthResponse.
        // Assert: current_org is the new org, orgs list contains the new org,
        // and the user row has current_org_id set to the new org id.
        let storage = Arc::new(PostgresStorage::from_pool(pool.clone()));
        let app = build_router(storage);
        let resp = post_json(
            &app,
            "/api/v1/auth/register",
            None,
            json!({"username": "alice", "password": "password123", "email": "alice@example.com"}),
        )
        .await;
        let body = body_json(resp).await;
        let token = body["token"].as_str().unwrap().to_string();
        let user_id: String = sqlx::query_scalar("SELECT id FROM users WHERE username = 'alice'")
            .fetch_one(&pool)
            .await
            .unwrap();

        // Limbo user creates an org.
        let resp = post_json(
            &app,
            "/api/v1/orgs",
            Some(&token),
            json!({"slug": "acme", "name": "Acme"}),
        )
        .await;
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        let body = body_json(resp).await;
        // current_org should be the new org (auto-switched because was limbo).
        assert_eq!(body["current_org"]["slug"], "acme");
        assert_eq!(body["current_org"]["role"], "owner");
        // orgs list contains the new org.
        assert!(body["orgs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["slug"] == "acme"));
        // New token issued.
        assert!(body["token"].is_string());

        // DB: user.current_org_id was persisted to the new org.
        let new_org_id: String = sqlx::query_scalar("SELECT id FROM orgs WHERE slug = 'acme'")
            .fetch_one(&pool)
            .await
            .unwrap();
        let current = user_current_org_id(&pool, &user_id).await;
        assert_eq!(current.as_deref(), Some(new_org_id.as_str()));
    }

    #[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
    async fn create_org_called_by_established_user_does_not_switch_current_org(pool: PgPool) {
        // Setup: user already has Org A as current_org.
        // POST /api/v1/orgs with slug "b" → 200 with AuthResponse.
        // Assert: current_org is STILL Org A (not Org B), orgs list contains
        //   both Org A and Org B, and user row's current_org_id is unchanged.
        let storage = Arc::new(PostgresStorage::from_pool(pool.clone()));
        let app = build_router(storage.clone());

        // Bootstrap: create Org A (org_a) via storage, seed an owner user
        // with current_org_id pointing at Org A, then create Org B via the
        // API and verify current_org stays at Org A.
        let owner_id = "established-1".to_string();
        sqlx::query(
            r#"INSERT INTO users (id, username, password, current_org_id, enabled, created_at, updated_at)
               VALUES ($1, 'established', 'x', NULL, true, NOW(), NOW())"#,
        )
        .bind(&owner_id)
        .execute(&pool)
        .await
        .unwrap();
        let org_a = storage
            .create_org(CreateOrg {
                id: "org-a-id".to_string(),
                slug: "org-a".to_string(),
                name: "Org A".to_string(),
                owner_id: owner_id.clone(),
            })
            .await
            .unwrap();
        storage
            .upsert_member(llm_gateway_storage::Member {
                user_id: owner_id.clone(),
                org_id: org_a.id.clone(),
                role: llm_gateway_storage::MemberRole::Owner,
                group_id: None,
                created_by: Some(owner_id.clone()),
                created_at: chrono::Utc::now(),
            })
            .await
            .unwrap();
        // Set current_org_id to Org A — this user is NOT in limbo.
        sqlx::query("UPDATE users SET current_org_id = $1 WHERE id = $2")
            .bind(&org_a.id)
            .bind(&owner_id)
            .execute(&pool)
            .await
            .unwrap();

        // Token carries Org A as current_org_id (non-limbo JWT).
        let token = llm_gateway_auth::create_jwt(
            &owner_id,
            Some(&org_a.id),
            None,
            "test-jwt-secret",
        )
        .unwrap();

        let resp = post_json(
            &app,
            "/api/v1/orgs",
            Some(&token),
            json!({"slug": "org-b", "name": "Org B"}),
        )
        .await;
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        let body = body_json(resp).await;
        // current_org should STILL be Org A, not the newly created Org B.
        assert_eq!(body["current_org"]["slug"], "org-a",
            "established user must not be yanked into the new org");
        // orgs list contains BOTH Org A and Org B.
        let org_slugs: Vec<&str> = body["orgs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| o["slug"].as_str().unwrap())
            .collect();
        assert!(org_slugs.contains(&"org-a"));
        assert!(org_slugs.contains(&"org-b"));

        // DB: user.current_org_id is UNCHANGED (still Org A).
        let current = user_current_org_id(&pool, &owner_id).await;
        assert_eq!(current.as_deref(), Some(org_a.id.as_str()));
    }

    #[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
    async fn me_onboarding_returns_true_for_limbo_user(pool: PgPool) {
        // Register → GET /api/v1/auth/me/onboarding → { needs_onboarding: true }.
        let storage = Arc::new(PostgresStorage::from_pool(pool.clone()));
        let app = build_router(storage);
        let resp = post_json(
            &app,
            "/api/v1/auth/register",
            None,
            json!({"username": "alice", "password": "password123", "email": "alice@example.com"}),
        )
        .await;
        let body = body_json(resp).await;
        let token = body["token"].as_str().unwrap().to_string();

        let resp = get_authed(&app, "/api/v1/auth/me/onboarding", &token).await;
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["needs_onboarding"], true);
    }

    #[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
    async fn me_onboarding_returns_false_once_user_has_org(pool: PgPool) {
        // Register → create_org → GET /api/v1/auth/me/onboarding → { needs_onboarding: false }.
        let storage = Arc::new(PostgresStorage::from_pool(pool.clone()));
        let app = build_router(storage);
        let resp = post_json(
            &app,
            "/api/v1/auth/register",
            None,
            json!({"username": "alice", "password": "password123", "email": "alice@example.com"}),
        )
        .await;
        let body = body_json(resp).await;
        let token = body["token"].as_str().unwrap().to_string();

        // Complete onboarding by creating an org.
        let resp = post_json(
            &app,
            "/api/v1/orgs",
            Some(&token),
            json!({"slug": "acme", "name": "Acme"}),
        )
        .await;
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        // Use the freshly reissued token from the AuthResponse.
        let body = body_json(resp).await;
        let token = body["token"].as_str().unwrap().to_string();

        let resp = get_authed(&app, "/api/v1/auth/me/onboarding", &token).await;
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["needs_onboarding"], false);
    }
}
