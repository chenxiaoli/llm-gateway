mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::routing::get;
use axum::Router;
use llm_gateway_api::middleware::auth_layer;
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
