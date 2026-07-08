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
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub refresh_token: String,
    pub user: UserInfo,
    pub current_org: OrgSummary,
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
    pub current_org: OrgSummary,
    pub orgs: Vec<OrgSummary>,
    pub allow_registration: bool,
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
async fn current_membership(
    state: &AppState,
    user: &User,
) -> Result<(OrgSummary, Vec<OrgSummary>), ApiError> {
    let memberships = state
        .storage
        .list_orgs_for_user(&user.id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let current = memberships
        .iter()
        .find(|m| Some(&m.org.id) == user.current_org_id.as_ref())
        .or_else(|| memberships.first())
        .ok_or_else(|| ApiError::Internal("user has no org membership".to_string()))?
        .clone();

    let orgs: Vec<OrgSummary> = memberships.into_iter().map(Into::into).collect();
    Ok((current.into(), orgs))
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

    let (current_org, orgs) = current_membership(&state, &user).await?;

    let platform_role_str = user.platform_role.as_ref().map(|p| p.as_str());
    let token = create_jwt(&user.id, &current_org.id, platform_role_str, &state.jwt_secret)
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
    let platform_role = if is_first_user {
        Some(PlatformRole::PlatformAdmin)
    } else {
        None
    };
    let mut user = User {
        id: uuid::Uuid::new_v4().to_string(),
        username: input.username.clone(),
        password: hash_password(&input.password).map_err(|e| ApiError::Internal(e.to_string()))?,
        platform_role,
        current_org_id: None,
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

    // TODO(Phase 3): replace the default-org-membership below with auto-creation
    // of a personal org whose slug is derived from the username.
    let default_org = state
        .storage
        .get_org_by_slug("default")
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::Internal("default org missing".to_string()))?;

    state
        .storage
        .upsert_member(Member {
            user_id: user.id.clone(),
            org_id: default_org.id.clone(),
            role: if is_first_user {
                MemberRole::Owner
            } else {
                MemberRole::Member
            },
            group_id: None,
            created_by: Some(user.id.clone()),
            created_at: now,
        })
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Persist current_org_id on the user row.
    user.current_org_id = Some(default_org.id.clone());
    user.updated_at = chrono::Utc::now();
    user = state
        .storage
        .update_user(&user)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Auto-create account for new user (Phase 1: scoped to default org).
    let account = llm_gateway_storage::Account {
        id: uuid::Uuid::new_v4().to_string(),
        org_id: default_org.id.clone(),
        user_id: user.id.clone(),
        balance: 0,
        threshold: llm_gateway_storage::usd_to_units(1.0),
        created_at: now,
        updated_at: now,
    };
    state
        .storage
        .create_account(&default_org.id, &account)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let (current_org, orgs) = current_membership(&state, &user).await?;

    let platform_role_str = user.platform_role.as_ref().map(|p| p.as_str());
    let token = create_jwt(&user.id, &current_org.id, platform_role_str, &state.jwt_secret)
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

    Ok(Json(MeResponse {
        id: user.id,
        username: user.username,
        platform_role: user.platform_role.as_ref().map(|p| p.as_str().to_string()),
        current_org,
        orgs,
        allow_registration: allow_reg,
    }))
}

use llm_gateway_storage::{units_to_usd, TransactionResponse};

pub async fn me_balance(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(pagination): Query<llm_gateway_storage::PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let account = state
        .storage
        .get_account_by_user_id(&claims.current_org_id, &claims.sub)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound("Account not found".to_string()))?;

    let (page, page_size) = pagination.normalized();
    let transactions = state
        .storage
        .list_transactions(&claims.current_org_id, &account.id, page, page_size)
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

    // Resolve current org so the new access token carries it.
    let (current_org, _orgs) = current_membership(&state, &user).await?;

    // Issue new access token
    let platform_role_str = user.platform_role.as_ref().map(|p| p.as_str());
    let new_token = create_jwt(&user.id, &current_org.id, platform_role_str, &state.jwt_secret)
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
    let token = create_jwt(&user.id, &target_org.id, platform_role_str, &state.jwt_secret)
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
        current_org: OrgSummary {
            id: target_org.id,
            slug: target_org.slug,
            name: target_org.name,
            role: member.role.as_str().to_string(),
            group_id: member.group_id,
        },
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
    Ok(())
}

/// Create a new org with the caller as owner.
pub async fn create_org(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateOrgRequest>,
) -> Result<Json<OrgSummary>, ApiError> {
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

    // Make the new org the caller's current org so subsequent requests
    // default to it without an explicit switch.
    let mut updated = user.clone();
    updated.current_org_id = Some(org.id.clone());
    updated.updated_at = now;
    state
        .storage
        .update_user(&updated)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(OrgSummary {
        id: org.id,
        slug: org.slug,
        name: org.name,
        role: MemberRole::Owner.as_str().to_string(),
        group_id: None,
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
}
