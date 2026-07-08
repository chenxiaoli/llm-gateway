mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::routing::get;
use axum::Router;
use llm_gateway_api::middleware::{auth_layer, membership_layer, org_resolve_layer};
use llm_gateway_org::{OrgContext, ResolvedOrg};
use sqlx::PgPool;
use tower::ServiceExt;

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn auth_layer_rejects_missing_header(pool: PgPool) {
    let state = common::make_state(pool);
    let app = Router::new()
        .route("/ok", get(|| async { "ok" }))
        .layer(from_fn_with_state(state, auth_layer));

    let resp = app
        .oneshot(Request::builder().uri("/ok").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn auth_layer_accepts_valid_token(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let state = common::make_state(pool);
    let app = Router::new()
        .route("/ok", get(|| async { "ok" }))
        .layer(from_fn_with_state(state, auth_layer));

    let token = common::make_admin_token().token;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/ok")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn org_resolve_layer_404s_unknown_slug(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let state = common::make_state(pool);
    // Stack auth_layer → org_resolve_layer so the JWT in extensions is
    // populated before org_resolve_layer reads path params. The 404 must
    // come from org_resolve_layer, not auth_layer (token is valid).
    let app = Router::new()
        .route("/{org_slug}/probe", get(|| async { "ok" }))
        .layer(from_fn_with_state(state.clone(), org_resolve_layer))
        .layer(from_fn_with_state(state, auth_layer));

    let token = common::make_admin_token().token;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/ghost-org/probe")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn org_resolve_layer_injects_resolved_org_extension(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let state = common::make_state(pool);
    let app = Router::new()
        .route(
            "/{org_slug}/probe",
            get(|axum::Extension(org): axum::Extension<ResolvedOrg>| async move {
                format!("{}/{}", org.id, org.slug)
            }),
        )
        .layer(from_fn_with_state(state.clone(), org_resolve_layer))
        .layer(from_fn_with_state(state, auth_layer));

    let token = common::make_admin_token().token;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/default/probe")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&body[..], b"org_default/default");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn membership_layer_injects_org_context_for_member(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let state = common::make_state(pool);
    // Stack auth_layer → org_resolve_layer → membership_layer so the request
    // mirrors real flow: JWT → ResolvedOrg → OrgContext.
    let app = Router::new()
        .route(
            "/{org_slug}/probe",
            get(|req: axum::extract::Request| async move {
                let ctx = req.extensions().get::<OrgContext>().cloned().unwrap();
                format!("{}:{:?}", ctx.org_id, ctx.member_role)
            }),
        )
        .layer(from_fn_with_state(state.clone(), membership_layer))
        .layer(from_fn_with_state(state.clone(), org_resolve_layer))
        .layer(from_fn_with_state(state, auth_layer));

    let token = common::make_admin_token().token;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/default/probe")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&body[..], b"org_default:Owner");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn membership_layer_403s_non_member(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    // Insert a user with NO member row in org_default.
    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ('outsider-1', 'outsider', 'x', NULL, $1, true, NOW(), NOW())"#,
    )
    .bind(common::TEST_ORG)
    .execute(&pool)
    .await
    .unwrap();

    let state = common::make_state(pool);
    let app = Router::new()
        .route("/{org_slug}/probe", get(|| async { "ok" }))
        .layer(from_fn_with_state(state.clone(), membership_layer))
        .layer(from_fn_with_state(state.clone(), org_resolve_layer))
        .layer(from_fn_with_state(state, auth_layer));

    let token =
        llm_gateway_auth::create_jwt("outsider-1", common::TEST_ORG, None, common::TEST_JWT_SECRET)
            .unwrap();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/default/probe")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn membership_layer_updates_last_seen(pool: PgPool) {
    common::seed_admin_user(&pool).await;
    let state = common::make_state(pool.clone());

    // Capture last_seen before the request. seed_admin_user inserts admin-1
    // with last_seen=NOW() (default from the migration), so we sleep briefly
    // to guarantee the post-request timestamp will differ.
    let before: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "SELECT last_seen FROM members WHERE user_id = 'admin-1' AND org_id = $1",
    )
    .bind(common::TEST_ORG)
    .fetch_one(&pool)
    .await
    .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let app = Router::new()
        .route("/{org_slug}/probe", get(|| async { "ok" }))
        .layer(from_fn_with_state(state.clone(), membership_layer))
        .layer(from_fn_with_state(state.clone(), org_resolve_layer))
        .layer(from_fn_with_state(state, auth_layer));

    let token = common::make_admin_token().token;
    let _ = app
        .oneshot(
            Request::builder()
                .uri("/default/probe")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let after: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "SELECT last_seen FROM members WHERE user_id = 'admin-1' AND org_id = $1",
    )
    .bind(common::TEST_ORG)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(after > before, "last_seen should advance: before={before:?} after={after:?}");
}
