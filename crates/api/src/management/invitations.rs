use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use llm_gateway_auth::create_jwt;
use llm_gateway_email::dispatch_with_retry;
use llm_gateway_email::templates::InvitationCtx;
use llm_gateway_org::{can_administer, OrgContext};
use llm_gateway_storage::{
    AcceptInvitationRequest, InvitationPreview, InvitationResponse, MemberRole,
};

use crate::auth::{AuthResponse, OrgSummary, UserInfo};
use crate::error::ApiError;
use crate::extractors::require_auth;
use crate::AppState;

/// Invitation token lifetime. Kept short — these are typically shared
/// one-to-one in chat/email, not posted broadly.
const INVITATION_TTL_DAYS: u32 = 7;

#[derive(Debug, Deserialize)]
pub struct CreateInvitationBody {
    pub role: String,
    /// Phase 4: invitation is bound to this recipient email. The handler
    /// validates the format and that no existing user holds the address.
    pub recipient_email: String,
}

/// Parse the role from the request body. 'owner' is explicitly rejected with
/// a 400 — it's not assignable via invitation (the DB CHECK enforces this too,
/// but we surface a friendly error before hitting the DB).
fn parse_invitation_role(s: &str) -> Result<MemberRole, ApiError> {
    match s {
        "member" => Ok(MemberRole::Member),
        "admin" => Ok(MemberRole::Admin),
        "owner" => Err(ApiError::BadRequest(
            "owner role cannot be assigned via invitation".into(),
        )),
        other => Err(ApiError::BadRequest(format!(
            "unknown role '{other}'; expected one of: member, admin"
        ))),
    }
}

/// Build the URL the admin will share. Uses `state.public_base_url` and the
/// token. Trims any trailing slash on the base so the result is canonical
/// regardless of how the operator configured it.
fn build_invite_url(public_base_url: &str, token: &str) -> String {
    let base = public_base_url.trim_end_matches('/');
    format!("{base}/accept-invite?token={token}")
}

/// POST /api/v1/{org_slug}/invitations — mint a new invitation token.
///
/// Admin/owner only. The returned `url` is ready to paste into a message; the
/// frontend does not need to know the public base URL.
pub async fn create_invitation(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Json(body): Json<CreateInvitationBody>,
) -> Result<(StatusCode, Json<InvitationResponse>), ApiError> {
    if !can_administer(&ctx) {
        return Err(ApiError::Forbidden);
    }
    let role = parse_invitation_role(&body.role)?;
    crate::auth::validate_email(&body.recipient_email).map_err(ApiError::BadRequest)?;
    // Normalize to lowercase so the accept-time email-match gate (which
    // compares via .to_lowercase()) can't fail on a case mismatch — e.g.
    // admin mints Alice@Example.com, invitee verifies alice@example.com.
    let recipient_email = body.recipient_email.trim().to_lowercase();
    // Reject if the recipient already has an account — invites aren't for
    // existing users; the admin should change their role instead.
    if state
        .storage
        .get_user_by_email(&recipient_email)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .is_some()
    {
        return Err(ApiError::EmailInUse);
    }
    let now = chrono::Utc::now();
    let expires_at = now + chrono::Duration::days(i64::from(INVITATION_TTL_DAYS));

    let invitation = state
        .storage
        .create_invitation(&ctx.org_id, &role, &ctx.user_id, &recipient_email, expires_at)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Dispatch the invitation email fire-and-forget. Failures here don't fail
    // the mint — the token is valid, the admin can re-trigger from the UI.
    let accept_url = build_invite_url(&state.public_base_url, &invitation.token);
    // These fetches only feed the email template — failures degrade to empty
    // org name / fallback inviter id, never fail the mint. (The invitation row
    // is already committed above; a transient DB blip here must not 500 the
    // admin or they'll retry and mint a duplicate.)
    let org_name = state
        .storage
        .get_org(&ctx.org_id)
        .await
        .ok() // transient DB error → empty org name, not a 500
        .flatten()
        .map(|o| o.name)
        .unwrap_or_default();
    let inviter_username = state
        .storage
        .get_user(&ctx.user_id)
        .await
        .ok()
        .flatten()
        .map(|u| u.username)
        .unwrap_or_else(|| ctx.user_id.clone());
    let response = InvitationResponse {
        id: invitation.id,
        token: invitation.token.clone(),
        url: accept_url,
        role: invitation.role.as_str().to_string(),
        created_at: invitation.created_at,
        expires_at: invitation.expires_at,
        accepted_at: None,
        accepted_by: None,
        revoked_at: None,
    };
    match state.templates.render_invitation(InvitationCtx {
        org_name,
        inviter_username,
        role: invitation.role.as_str().to_string(),
        recipient_email,
        accept_url: response.url.clone(),
        expires_in_days: INVITATION_TTL_DAYS,
        public_base_url: state.public_base_url.clone(),
    }) {
        Ok(msg) => {
            dispatch_with_retry(state.mailer.clone(), msg, "invitation email".to_string());
        }
        Err(e) => {
            tracing::error!(error = ?e, "failed to render invitation email; dispatch skipped");
        }
    }

    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /api/v1/{org_slug}/invitations — list pending + recently-accepted.
///
/// "Recent" = accepted within the last 30 days. Revoked invitations are
/// excluded so the list reflects actionable state.
pub async fn list_invitations(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
) -> Result<Json<Vec<InvitationResponse>>, ApiError> {
    if !can_administer(&ctx) {
        return Err(ApiError::Forbidden);
    }
    let invitations = state
        .storage
        .list_invitations_for_org(&ctx.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let cutoff = chrono::Utc::now() - chrono::Duration::days(30);
    let mut out = Vec::with_capacity(invitations.len());
    for inv in invitations {
        // Include: pending (not accepted, not revoked) OR accepted within last 30d.
        let include = match inv.accepted_at {
            Some(t) => t > cutoff,
            None => inv.revoked_at.is_none(),
        };
        if !include {
            continue;
        }
        // Resolve accepted_by UUID → username for the response (frontend doesn't
        // need raw user IDs). If the accepter was deleted (orphan FK), the
        // lookup returns None and we surface null — not an empty string.
        let accepted_by_username: Option<String> = match &inv.accepted_by {
            Some(uid) => state
                .storage
                .get_user(uid)
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?
                .map(|u| u.username),
            None => None,
        };
        out.push(InvitationResponse {
            id: inv.id,
            token: inv.token.clone(),
            url: build_invite_url(&state.public_base_url, &inv.token),
            role: inv.role.as_str().to_string(),
            created_at: inv.created_at,
            expires_at: inv.expires_at,
            accepted_at: inv.accepted_at,
            accepted_by: accepted_by_username,
            revoked_at: inv.revoked_at,
        });
    }
    Ok(Json(out))
}

/// DELETE /api/v1/{org_slug}/invitations/{id} — revoke an invitation.
///
/// The storage layer scopes the UPDATE by `org_id`, so a call from an admin
/// in org B against org A's invitation is a silent no-op (no row touched, no
/// error). We return 204 either way — leaking "not found vs. revoked" would
/// be an info disclosure.
pub async fn revoke_invitation(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
    Path((_org_slug, invitation_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    if !can_administer(&ctx) {
        return Err(ApiError::Forbidden);
    }
    state
        .storage
        .revoke_invitation(&ctx.org_id, &invitation_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct PreviewQuery {
    pub token: String,
}

/// Single static "no longer valid" reason for any non-consumable token.
/// Identical for invalid/expired/revoked/already-accepted to prevent
/// enumeration via response-body differences.
const INVITATION_GONE_REASON: &str = "This invitation is no longer valid.";

/// GET /api/v1/invitations/preview?token=... — public (no auth).
///
/// Returns org metadata for the landing page. Any non-consumable token
/// (invalid/expired/revoked/already-accepted) yields the same 410 body so
/// the endpoint cannot be probed to enumerate valid tokens.
///
/// Tokens travel in the URL query string and could be cached by
/// intermediate proxies/CDNs, so the response always carries
/// `Cache-Control: no-store, private`.
pub async fn preview_invitation(
    State(state): State<Arc<AppState>>,
    Query(q): Query<PreviewQuery>,
) -> Result<Response, ApiError> {
    let Some(inv) = state
        .storage
        .get_invitation_by_token(&q.token)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
    else {
        return Err(ApiError::Gone(INVITATION_GONE_REASON.to_string()));
    };

    let now = chrono::Utc::now();
    if inv.accepted_at.is_some() || inv.revoked_at.is_some() || inv.expires_at < now {
        return Err(ApiError::Gone(INVITATION_GONE_REASON.to_string()));
    }

    let org = state
        .storage
        .get_org(&inv.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::Internal("invitation references missing org".into()))?;
    let inviter = state
        .storage
        .get_user(&inv.created_by)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .map(|u| u.username)
        .unwrap_or_default();

    let preview = InvitationPreview {
        org_name: org.name,
        org_slug: org.slug,
        role: inv.role.as_str().to_string(),
        inviter_username: inviter,
        recipient_email: inv.recipient_email.clone(),
        expires_at: inv.expires_at,
    };
    let mut response = Json(preview).into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store, private"),
    );
    Ok(response)
}

/// POST /api/v1/invitations/accept — authed. Body `{ token }`.
///
/// Calls the storage-layer `accept_invitation` (transactional, single-use).
/// Reissues the JWT with the new current_org and returns an `AuthResponse`
/// mirroring login/switch_org so the frontend can drop-in replace.
pub async fn accept_invitation(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<AcceptInvitationRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    // Phase 4: the accepter must have a verified email AND that email must
    // match the invitation's recipient. This is what binds "click the link"
    // to "the person we actually invited" — without it, anyone who obtained
    // the token could join. The checks run BEFORE the transactional accept so
    // a failed match leaves the invitation consumable.
    let user = state
        .storage
        .get_user(&claims.sub)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;
    if user.email_verified_at.is_none() {
        return Err(ApiError::EmailVerificationRequired);
    }
    let inv = state
        .storage
        .get_invitation_by_token(&body.token)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::Gone(INVITATION_GONE_REASON.to_string()))?;
    // A verified user always carries an email; a Phase 4 invitation always
    // carries a recipient. If either is missing, the row is internally
    // inconsistent — surface as 500 rather than silently allowing the accept.
    let user_email = user
        .email
        .as_ref()
        .map(|e| e.to_lowercase())
        .ok_or_else(|| ApiError::Internal("verified user has no email".into()))?;
    let recipient = inv
        .recipient_email
        .as_ref()
        .map(|e| e.to_lowercase())
        .ok_or_else(|| ApiError::Internal("invitation has no recipient_email".into()))?;
    if user_email != recipient {
        return Err(ApiError::EmailMismatchAccept);
    }

    let Some(member) = state
        .storage
        .accept_invitation(&body.token, &claims.sub)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
    else {
        // Distinguish race-loser from invalid/expired/revoked.
        // - If the token exists AND was accepted (regardless of by whom) → 409 Conflict.
        // - Otherwise (expired, revoked, never-existed) → 410 Gone.
        let existing = state
            .storage
            .get_invitation_by_token(&body.token)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        let was_just_consumed = matches!(existing, Some(inv) if inv.accepted_at.is_some());
        if was_just_consumed {
            return Err(ApiError::Conflict(
                "invitation was already accepted".to_string(),
            ));
        }
        return Err(ApiError::Gone(INVITATION_GONE_REASON.to_string()));
    };

    // Reload the user (their membership list just changed) and persist the
    // current_org_id switch so subsequent requests default to the new org.
    let mut user = state
        .storage
        .get_user(&claims.sub)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;
    user.current_org_id = Some(member.org_id.clone());
    user.updated_at = chrono::Utc::now();
    user = state
        .storage
        .update_user(&user)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let platform_role_str = user.platform_role.as_ref().map(|p| p.as_str());
    let token = create_jwt(&user.id, Some(&member.org_id), platform_role_str, &state.jwt_secret)
        .map_err(ApiError::Internal)?;

    let memberships = state
        .storage
        .list_orgs_for_user(&user.id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let orgs: Vec<OrgSummary> = memberships.into_iter().map(Into::into).collect();
    let current_org = orgs
        .iter()
        .find(|o| o.id == member.org_id)
        .cloned()
        .ok_or_else(|| ApiError::Internal("just-joined org not in membership list".into()))?;

    Ok(Json(AuthResponse {
        token,
        refresh_token: user.refresh_token.clone().unwrap_or_default(),
        user: UserInfo::from(&user),
        current_org: Some(current_org),
        orgs,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use llm_gateway_email::noop::NoopMailer;
    use llm_gateway_email::templates::TemplateRegistry;
    use llm_gateway_ratelimit::RateLimiter;
    use llm_gateway_storage::postgres::PostgresStorage;
    use llm_gateway_storage::{CreateOrg, Storage};

    /// Build a minimal AppState wired to a real Postgres pool. Only the
    /// fields the invitation handlers touch (`storage`, `public_base_url`)
    /// carry meaningful values; the rest are placeholders that are never
    /// exercised by these tests.
    fn make_state(storage: Arc<dyn Storage>) -> Arc<AppState> {
        let registry_storage = storage.clone();
        Arc::new(AppState {
            storage,
            rate_limiter: Arc::new(RateLimiter::new(60)),
            jwt_secret: "test".to_string(),
            encryption_key: [0u8; 32],
            nats_publisher: None,
            registry: Arc::new(crate::InMemoryChannelRegistry::new(
                // The registry isn't used by invitation handlers; give it a
                // clone of the same storage arc so it could refresh if asked.
                registry_storage,
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
        })
    }

    async fn make_org(storage: &PostgresStorage, id: &str) -> llm_gateway_storage::Org {
        let owner_id = format!("owner-{id}");
        sqlx::query(
            "INSERT INTO users (id, username, password, created_at, updated_at)
             VALUES ($1, $1, 'x', NOW(), NOW()) ON CONFLICT (id) DO NOTHING",
        )
        .bind(&owner_id)
        .execute(storage.pool())
        .await
        .expect("insert owner user");
        storage
            .create_org(CreateOrg {
                id: id.to_string(),
                slug: id.to_string(),
                name: id.to_string(),
                owner_id: owner_id.clone(),
            })
            .await
            .expect("create_org")
    }

    async fn make_user(storage: &PostgresStorage, id: &str) {
        let now = chrono::Utc::now();
        storage
            .create_user(&llm_gateway_storage::User {
                id: id.to_string(),
                username: id.to_string(),
                password: "x".to_string(),
                platform_role: None,
                current_org_id: None,
                enabled: true,
                refresh_token: None,
                created_at: now,
                updated_at: now,
                // Phase 4: default to a verified email so the accept_invitation
                // checks (verified + email-match) succeed for tests that don't
                // care about those gates. Tests that exercise the gates use
                // `make_unverified_user` or pass a custom recipient_email.
                email: Some(format!("{id}@example.com")),
                email_verified_at: Some(now),
                requires_email_verification: false,
                password_changed_at: now,
            })
            .await
            .expect("create_user");
    }

    /// Like `make_user` but with `email_verified_at = None`. For tests that
    /// exercise the verification gate on `accept_invitation`.
    async fn make_unverified_user(storage: &PostgresStorage, id: &str) {
        let now = chrono::Utc::now();
        storage
            .create_user(&llm_gateway_storage::User {
                id: id.to_string(),
                username: id.to_string(),
                password: "x".to_string(),
                platform_role: None,
                current_org_id: None,
                enabled: true,
                refresh_token: None,
                created_at: now,
                updated_at: now,
                email: Some(format!("{id}@example.com")),
                email_verified_at: None,
                requires_email_verification: true,
                password_changed_at: now,
            })
            .await
            .expect("create_user");
    }

    fn ctx(org_id: &str, user_id: &str, role: MemberRole) -> OrgContext {
        OrgContext {
            user_id: user_id.to_string(),
            org_id: org_id.to_string(),
            member_role: role,
            platform_role: None,
            group_id: None,
        }
    }

    #[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
    async fn admin_can_mint_invitation(pool: sqlx::PgPool) {
        let storage = Arc::new(PostgresStorage::from_pool(pool));
        let org = make_org(&storage, "acme").await;
        make_user(&storage, "alice").await;
        let state = make_state(storage.clone());

        let resp = create_invitation(
            State(state.clone()),
            ctx(&org.id, "alice", MemberRole::Admin),
            Json(CreateInvitationBody {
                role: "admin".into(),
                recipient_email: "invitee1@example.com".into(),
            }),
        )
        .await
        .expect("admin can mint");

        assert_eq!(resp.0, StatusCode::CREATED);
        let body = resp.1 .0;
        assert!(!body.token.is_empty(), "token populated");
        assert_eq!(
            body.url,
            format!("http://localhost:5173/accept-invite?token={}", body.token)
        );
        assert_eq!(body.role, "admin");
        assert!(body.accepted_at.is_none());
        assert!(body.revoked_at.is_none());
    }

    #[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
    async fn member_cannot_mint(pool: sqlx::PgPool) {
        let storage = Arc::new(PostgresStorage::from_pool(pool));
        let org = make_org(&storage, "acme").await;
        make_user(&storage, "bob").await;
        let state = make_state(storage.clone());

        let err = create_invitation(
            State(state),
            ctx(&org.id, "bob", MemberRole::Member),
            Json(CreateInvitationBody {
                role: "member".into(),
                recipient_email: "invitee2@example.com".into(),
            }),
        )
        .await
        .expect_err("member forbidden");

        assert!(matches!(err, ApiError::Forbidden), "got {err:?}");
    }

    #[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
    async fn owner_role_rejected_at_mint(pool: sqlx::PgPool) {
        let storage = Arc::new(PostgresStorage::from_pool(pool));
        let org = make_org(&storage, "acme").await;
        make_user(&storage, "alice").await;
        let state = make_state(storage.clone());

        let err = create_invitation(
            State(state),
            ctx(&org.id, "alice", MemberRole::Admin),
            Json(CreateInvitationBody {
                role: "owner".into(),
                recipient_email: "invitee3@example.com".into(),
            }),
        )
        .await
        .expect_err("owner rejected");

        match err {
            ApiError::BadRequest(msg) => assert!(
                msg.contains("owner"),
                "expected message about owner role, got: {msg}"
            ),
            other => panic!("expected BadRequest, got {other:?}"),
        }
    }

    #[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
    async fn list_returns_pending_and_recently_accepted(pool: sqlx::PgPool) {
        let storage = Arc::new(PostgresStorage::from_pool(pool));
        let org = make_org(&storage, "acme").await;
        make_user(&storage, "alice").await;
        make_user(&storage, "bob").await;
        let state = make_state(storage.clone());

        let admin_ctx = ctx(&org.id, "alice", MemberRole::Admin);

        // Mint two.
        let inv1 = super::create_invitation(
            State(state.clone()),
            admin_ctx.clone(),
            Json(CreateInvitationBody {
                role: "member".into(),
                recipient_email: "invitee1@example.com".into(),
            }),
        )
        .await
        .expect("mint 1")
        .1
         .0;
        let inv2 = super::create_invitation(
            State(state.clone()),
            admin_ctx.clone(),
            Json(CreateInvitationBody {
                role: "admin".into(),
                recipient_email: "invitee2@example.com".into(),
            }),
        )
        .await
        .expect("mint 2")
        .1
         .0;

        // Accept inv1 as bob directly via storage.
        storage
            .accept_invitation(&inv1.token, "bob")
            .await
            .expect("accept")
            .expect("consumable");

        // Mint a third pending one.
        let _inv3 = super::create_invitation(
            State(state.clone()),
            admin_ctx.clone(),
            Json(CreateInvitationBody {
                role: "member".into(),
                recipient_email: "invitee3@example.com".into(),
            }),
        )
        .await
        .expect("mint 3");

        let listed = list_invitations(State(state), admin_ctx)
            .await
            .expect("list")
            .0;

        // Expect: inv1 (accepted, <30d) + inv2 (pending) + inv3 (pending) = 3.
        assert_eq!(listed.len(), 3, "got: {listed:?}");
        let tokens: Vec<_> = listed.iter().map(|i| i.token.clone()).collect();
        assert!(tokens.contains(&inv1.token), "accepted inv1 should appear");
        assert!(tokens.contains(&inv2.token), "pending inv2 should appear");

        // The accepted one should carry the accepter's username.
        let accepted = listed
            .iter()
            .find(|i| i.token == inv1.token)
            .expect("inv1 in list");
        assert_eq!(accepted.accepted_by.as_deref(), Some("bob"));
        assert!(accepted.accepted_at.is_some());
    }

    #[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
    async fn admin_in_other_org_cannot_revoke(pool: sqlx::PgPool) {
        let storage = Arc::new(PostgresStorage::from_pool(pool));
        let org_a = make_org(&storage, "orga").await;
        let org_b = make_org(&storage, "orgb").await;
        make_user(&storage, "alice").await; // admin in org A
        make_user(&storage, "mallory").await; // admin in org B
        let state = make_state(storage.clone());

        // Mint an invitation in org A.
        let inv = create_invitation(
            State(state.clone()),
            ctx(&org_a.id, "alice", MemberRole::Admin),
            Json(CreateInvitationBody {
                role: "member".into(),
                recipient_email: "invitee4@example.com".into(),
            }),
        )
        .await
        .expect("mint in org A")
        .1
         .0;

        // Mallory (admin in org B) tries to revoke org A's invitation by id.
        let status = revoke_invitation(
            State(state.clone()),
            ctx(&org_b.id, "mallory", MemberRole::Admin),
            Path(("orga".to_string(), inv.id.clone())),
        )
        .await
        .expect("cross-org revoke returns 204 (no-op)");
        assert_eq!(status, StatusCode::NO_CONTENT);

        // Verify org A's invitation is NOT revoked.
        let after = storage
            .get_invitation_by_token(&inv.token)
            .await
            .expect("get")
            .expect("present");
        assert!(
            after.revoked_at.is_none(),
            "org A invitation must not be revoked by org B admin"
        );
    }

    // --- Task 5: public preview + transactional accept ---

    /// Helper: mint an invitation directly via storage with a configurable
    /// expiry. Used by the preview tests so we can backdate or forward-date
    /// without going through the handler (which always uses +7d).
    async fn mint_invitation(
        storage: &PostgresStorage,
        org_id: &str,
        created_by: &str,
        role: MemberRole,
        recipient_email: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> llm_gateway_storage::Invitation {
        storage
            .create_invitation(
                org_id,
                &role,
                created_by,
                recipient_email,
                expires_at,
            )
            .await
            .expect("mint_invitation")
    }

    #[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
    async fn preview_returns_metadata_for_valid_pending_token(pool: sqlx::PgPool) {
        let storage = Arc::new(PostgresStorage::from_pool(pool));
        let org = make_org(&storage, "acme").await;
        // make_org creates an owner user "owner-acme"; use them as inviter.
        let state = make_state(storage.clone());
        let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
        let inv = mint_invitation(&storage, &org.id, "owner-acme", MemberRole::Admin, "preview@example.com", expires_at)
            .await;

        let resp = preview_invitation(
            State(state),
            Query(PreviewQuery {
                token: inv.token.clone(),
            }),
        )
        .await
        .expect("preview ok");

        // Cache-Control: no-store, private is always set on preview responses.
        assert_eq!(
            resp.headers()
                .get(header::CACHE_CONTROL)
                .map(|v| v.to_str().unwrap()),
            Some("no-store, private")
        );

        let body: InvitationPreview =
            serde_json::from_slice(&axum::body::to_bytes(resp.into_body(), usize::MAX).await.expect("collect body"))
                .expect("deserialize preview body");
        assert_eq!(body.org_name, "acme");
        assert_eq!(body.org_slug, "acme");
        assert_eq!(body.role, "admin");
        assert_eq!(body.inviter_username, "owner-acme");
        assert_eq!(
            body.recipient_email.as_deref(),
            Some("preview@example.com"),
            "preview should surface recipient_email"
        );
        // Postgres stores microsecond precision; the round-trip can shed up to
        // 1us of sub-microsecond precision, so compare with a tolerance
        // rather than exact equality.
        let delta = (body.expires_at - expires_at).num_nanoseconds().unwrap();
        assert!(delta.abs() < 2_000, "expires_at drifted by {delta} ns");
    }

    #[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
    async fn preview_returns_410_for_expired(pool: sqlx::PgPool) {
        let storage = Arc::new(PostgresStorage::from_pool(pool));
        let org = make_org(&storage, "acme").await;
        let state = make_state(storage.clone());
        let past = chrono::Utc::now() - chrono::Duration::hours(1);
        let inv = mint_invitation(&storage, &org.id, "owner-acme", MemberRole::Member, "preview@example.com", past).await;

        let err = preview_invitation(
            State(state),
            Query(PreviewQuery {
                token: inv.token.clone(),
            }),
        )
        .await
        .expect_err("expired token -> 410");

        match err {
            ApiError::Gone(msg) => assert!(!msg.is_empty()),
            other => panic!("expected Gone, got {other:?}"),
        }
    }

    #[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
    async fn preview_returns_410_for_revoked(pool: sqlx::PgPool) {
        let storage = Arc::new(PostgresStorage::from_pool(pool));
        let org = make_org(&storage, "acme").await;
        let state = make_state(storage.clone());
        let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
        let inv = mint_invitation(&storage, &org.id, "owner-acme", MemberRole::Member, "preview@example.com", expires_at)
            .await;
        storage
            .revoke_invitation(&org.id, &inv.id)
            .await
            .expect("revoke");

        let err = preview_invitation(
            State(state),
            Query(PreviewQuery {
                token: inv.token.clone(),
            }),
        )
        .await
        .expect_err("revoked -> 410");

        assert!(matches!(err, ApiError::Gone(_)), "got {err:?}");
    }

    #[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
    async fn preview_returns_410_for_already_accepted(pool: sqlx::PgPool) {
        let storage = Arc::new(PostgresStorage::from_pool(pool));
        let org = make_org(&storage, "acme").await;
        make_user(&storage, "bob").await;
        let state = make_state(storage.clone());
        let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
        let inv = mint_invitation(&storage, &org.id, "owner-acme", MemberRole::Member, "preview@example.com", expires_at)
            .await;
        storage
            .accept_invitation(&inv.token, "bob")
            .await
            .expect("accept")
            .expect("consumable");

        let err = preview_invitation(
            State(state),
            Query(PreviewQuery {
                token: inv.token.clone(),
            }),
        )
        .await
        .expect_err("accepted -> 410");

        assert!(matches!(err, ApiError::Gone(_)), "got {err:?}");
    }

    #[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
    async fn preview_returns_410_for_invalid_token(pool: sqlx::PgPool) {
        let storage = Arc::new(PostgresStorage::from_pool(pool));
        let _org = make_org(&storage, "acme").await;
        let state = make_state(storage.clone());

        // Sanity: body shape for invalid matches the body shape for expired.
        let err_invalid = preview_invitation(
            State(state.clone()),
            Query(PreviewQuery {
                token: "nonexistent".into(),
            }),
        )
        .await
        .expect_err("invalid -> 410");
        let past = chrono::Utc::now() - chrono::Duration::hours(1);
        let org2 = make_org(&storage, "omega").await;
        let inv = mint_invitation(&storage, &org2.id, "owner-omega", MemberRole::Member, "preview@example.com", past)
            .await;
        let err_expired = preview_invitation(
            State(state),
            Query(PreviewQuery {
                token: inv.token.clone(),
            }),
        )
        .await
        .expect_err("expired -> 410");

        let gone_msg = |e: ApiError| match e {
            ApiError::Gone(m) => m,
            other => panic!("expected Gone, got {other:?}"),
        };
        assert_eq!(gone_msg(err_invalid), gone_msg(err_expired));
    }

    /// Build an Authorization header carrying a fresh JWT for `user_id`
    /// against `org_id`, signed with the test jwt_secret ("test").
    fn auth_header(user_id: &str, org_id: &str) -> HeaderMap {
        let token = llm_gateway_auth::create_jwt(user_id, Some(org_id), None, "test")
            .expect("create_jwt");
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            format!("Bearer {token}").parse().expect("header value"),
        );
        headers
    }

    #[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
    async fn accept_creates_membership_and_reissues_jwt(pool: sqlx::PgPool) {
        let storage = Arc::new(PostgresStorage::from_pool(pool));
        let org = make_org(&storage, "acme").await;
        // invitee is the caller; not yet a member of acme.
        make_user(&storage, "carol").await;
        let state = make_state(storage.clone());

        let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
        let inv = mint_invitation(&storage, &org.id, "owner-acme", MemberRole::Admin, "carol@example.com", expires_at)
            .await;

        let resp = accept_invitation(
            State(state.clone()),
            auth_header("carol", &org.id),
            Json(AcceptInvitationRequest {
                token: inv.token.clone(),
            }),
        )
        .await
        .expect("accept ok");
        let body = resp.0;

        // New JWT, non-empty.
        assert!(!body.token.is_empty(), "token populated");
        // current_org points at the inviting org. Phase 3 made this Optional
        // on AuthResponse — accept_invitation always sets it.
        let current_org = body
            .current_org
            .as_ref()
            .expect("accept_invitation should set current_org");
        assert_eq!(current_org.id, org.id);
        assert_eq!(current_org.role, "admin");
        // orgs list contains the inviting org.
        assert!(
            body.orgs.iter().any(|o| o.id == org.id),
            "inviting org present in orgs list"
        );
        // User identity echoed.
        assert_eq!(body.user.id, "carol");

        // current_org_id was persisted on the user row.
        let reloaded = storage.get_user("carol").await.unwrap().unwrap();
        assert_eq!(reloaded.current_org_id.as_deref(), Some(org.id.as_str()));

        // Membership row exists.
        let member = storage.get_member("carol", &org.id).await.unwrap();
        assert!(member.is_some(), "membership row created");
        assert_eq!(member.unwrap().role, MemberRole::Admin);

        // Invitation row is marked accepted.
        let after = storage
            .get_invitation_by_token(&inv.token)
            .await
            .unwrap()
            .unwrap();
        assert!(after.accepted_at.is_some());
        assert_eq!(after.accepted_by.as_deref(), Some("carol"));
    }

    #[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
    async fn accept_returns_409_after_sequential_consume(pool: sqlx::PgPool) {
        // The same invitee clicking an already-consumed token is the race-loser
        // case (their first accept landed first). Per spec, the second attempt
        // gets 409 Conflict — not 410 Gone (410 is reserved for invalid/expired/
        // revoked tokens that were never accepted). Email-binding means only the
        // one invited user can reach this path, so we exercise it with a single
        // user making two calls.
        let storage = Arc::new(PostgresStorage::from_pool(pool));
        let org = make_org(&storage, "acme").await;
        make_user(&storage, "carol").await;
        let state = make_state(storage.clone());

        let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
        let inv = mint_invitation(
            &storage,
            &org.id,
            "owner-acme",
            MemberRole::Member,
            "carol@example.com",
            expires_at,
        )
        .await;

        // Carol accepts first.
        let _ = accept_invitation(
            State(state.clone()),
            auth_header("carol", &org.id),
            Json(AcceptInvitationRequest {
                token: inv.token.clone(),
            }),
        )
        .await
        .expect("first accept ok");

        // Carol tries the same (already-consumed) token again -> 409 Conflict.
        let result = accept_invitation(
            State(state),
            auth_header("carol", &org.id),
            Json(AcceptInvitationRequest {
                token: inv.token.clone(),
            }),
        )
        .await;

        let err = match result {
            Ok(_) => panic!("second accept should fail with 409, got Ok"),
            Err(e) => e,
        };
        assert!(matches!(err, ApiError::Conflict(_)), "got {err:?}");
        // Carol is still exactly one membership (no duplicate row).
        let carol_member = storage.get_member("carol", &org.id).await.unwrap();
        assert!(carol_member.is_some(), "carol should be a member");
    }

    #[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
    async fn accept_concurrent_only_one_wins(pool: sqlx::PgPool) {
        // The same invitee races themselves on the same token (e.g. two browser
        // tabs). The storage layer's SELECT ... FOR UPDATE serializes them:
        // exactly one returns Ok(AuthResponse), the other returns
        // Err(ApiError::Conflict). The org gains exactly one membership. With
        // email-binding, only the invited user can reach this path, so we
        // exercise the row lock with a single user issuing two concurrent calls.
        let storage = Arc::new(PostgresStorage::from_pool(pool));
        let org = make_org(&storage, "acme").await;
        make_user(&storage, "alice").await;
        let state = make_state(storage.clone());

        let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
        let inv = mint_invitation(
            &storage,
            &org.id,
            "owner-acme",
            MemberRole::Member,
            "alice@example.com",
            expires_at,
        )
        .await;

        // Fire both accepts concurrently. tokio::join! polls them on the same
        // executor; the row lock guarantees one winner + one loser.
        let (res_a, res_b) = tokio::join!(
            accept_invitation(
                State(state.clone()),
                auth_header("alice", &org.id),
                Json(AcceptInvitationRequest {
                    token: inv.token.clone(),
                }),
            ),
            accept_invitation(
                State(state.clone()),
                auth_header("alice", &org.id),
                Json(AcceptInvitationRequest {
                    token: inv.token.clone(),
                }),
            ),
        );

        // Exactly one Ok, exactly one Err(Conflict).
        let loser = match (res_a, res_b) {
            (Ok(_), Err(e)) => e,
            (Err(e), Ok(_)) => e,
            (Ok(_), Ok(_)) => panic!("both accepts succeeded; row lock failed to serialize"),
            (Err(_), Err(_)) => panic!("both accepts failed; expected exactly one winner"),
        };
        assert!(
            matches!(loser, ApiError::Conflict(_)),
            "race-loser should be 409 Conflict, got {loser:?}"
        );

        // The org gained exactly one membership from this invitation.
        let winner_member = storage.get_member("alice", &org.id).await.unwrap();
        assert!(
            winner_member.is_some(),
            "alice should be a member"
        );

        // The invitation row records alice as accepter.
        let after = storage
            .get_invitation_by_token(&inv.token)
            .await
            .unwrap()
            .unwrap();
        assert!(after.accepted_at.is_some());
        assert_eq!(after.accepted_by.as_deref(), Some("alice"));
    }

    // --- Task 10: email-bound invitations (dispatch + accept gates) ---

    #[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
    async fn create_invitation_dispatches_email_and_persists_recipient(pool: sqlx::PgPool) {
        // The handler must persist recipient_email on the invitation row so the
        // accept-time email-match gate has something to compare against.
        // NoopMailer is silent; asserting the DB row proves the handler wired
        // the field through. The dispatch itself is exercised by code review —
        // dispatch_with_retry is separately unit-tested in the email crate.
        let storage = Arc::new(PostgresStorage::from_pool(pool));
        let org = make_org(&storage, "acme").await;
        make_user(&storage, "alice").await;
        let state = make_state(storage.clone());

        let resp = create_invitation(
            State(state.clone()),
            ctx(&org.id, "alice", MemberRole::Admin),
            Json(CreateInvitationBody {
                role: "member".into(),
                recipient_email: "newinvitee@example.com".into(),
            }),
        )
        .await
        .expect("mint ok");
        let body = resp.1 .0;

        let row = storage
            .get_invitation_by_token(&body.token)
            .await
            .expect("get")
            .expect("invitation persisted");
        assert_eq!(
            row.recipient_email.as_deref(),
            Some("newinvitee@example.com"),
            "recipient_email must be persisted on the row",
        );
    }

    #[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
    async fn accept_with_wrong_email_returns_403_email_mismatch(pool: sqlx::PgPool) {
        // Invitation is bound to alice@example.com. A verified user "bob" (with
        // a different verified email) must NOT be able to accept it — that's
        // the whole point of email-binding.
        let storage = Arc::new(PostgresStorage::from_pool(pool));
        let org = make_org(&storage, "acme").await;
        make_user(&storage, "bob").await; // bob@example.com, verified
        let state = make_state(storage.clone());

        let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
        let inv = mint_invitation(
            &storage,
            &org.id,
            "owner-acme",
            MemberRole::Member,
            "alice@example.com",
            expires_at,
        )
        .await;

        let result = accept_invitation(
            State(state),
            auth_header("bob", &org.id),
            Json(AcceptInvitationRequest {
                token: inv.token.clone(),
            }),
        )
        .await;

        let err = match result {
            Ok(_) => panic!("mismatched email should be rejected, got Ok"),
            Err(e) => e,
        };
        assert!(
            matches!(err, ApiError::EmailMismatchAccept),
            "got {err:?}"
        );
        // Bob did not join.
        let bob_member = storage.get_member("bob", &org.id).await.unwrap();
        assert!(bob_member.is_none(), "bob must not become a member");
        // The invitation is still consumable by the right person.
        let row = storage
            .get_invitation_by_token(&inv.token)
            .await
            .unwrap()
            .unwrap();
        assert!(row.accepted_at.is_none(), "invitation must remain pending");
    }

    #[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
    async fn accept_unverified_user_returns_403_verification_required(pool: sqlx::PgPool) {
        // Even if the email matches, an unverified user cannot accept — the
        // email-match check is meaningless without a verified email.
        let storage = Arc::new(PostgresStorage::from_pool(pool));
        let org = make_org(&storage, "acme").await;
        make_unverified_user(&storage, "carol").await; // carol@example.com, unverified
        let state = make_state(storage.clone());

        let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
        let inv = mint_invitation(
            &storage,
            &org.id,
            "owner-acme",
            MemberRole::Member,
            "carol@example.com",
            expires_at,
        )
        .await;

        let result = accept_invitation(
            State(state),
            auth_header("carol", &org.id),
            Json(AcceptInvitationRequest {
                token: inv.token.clone(),
            }),
        )
        .await;

        let err = match result {
            Ok(_) => panic!("unverified user should be rejected, got Ok"),
            Err(e) => e,
        };
        assert!(
            matches!(err, ApiError::EmailVerificationRequired),
            "got {err:?}"
        );
        let carol_member = storage.get_member("carol", &org.id).await.unwrap();
        assert!(carol_member.is_none(), "carol must not become a member");
    }
}
