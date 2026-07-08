use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use llm_gateway_org::{can_administer, OrgContext};
use llm_gateway_storage::{InvitationResponse, MemberRole};

use crate::error::ApiError;
use crate::AppState;

/// Invitation token lifetime. Kept short — these are typically shared
/// one-to-one in chat/email, not posted broadly.
const INVITATION_TTL_DAYS: i64 = 7;

#[derive(Debug, Deserialize)]
pub struct CreateInvitationBody {
    pub role: String,
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
    let now = chrono::Utc::now();
    let expires_at = now + chrono::Duration::days(INVITATION_TTL_DAYS);

    let invitation = state
        .storage
        .create_invitation(&ctx.org_id, &role, &ctx.user_id, expires_at)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(InvitationResponse {
            id: invitation.id,
            token: invitation.token.clone(),
            url: build_invite_url(&state.public_base_url, &invitation.token),
            role: invitation.role.as_str().to_string(),
            created_at: invitation.created_at,
            expires_at: invitation.expires_at,
            accepted_at: None,
            accepted_by: None,
            revoked_at: None,
        }),
    ))
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

#[cfg(test)]
mod tests {
    use super::*;
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
}
