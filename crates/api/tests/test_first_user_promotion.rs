mod common;

use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use llm_gateway_api::{management, AppState};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

async fn register_first_user(state: Arc<AppState>) -> Response<Body> {
    let app = management::management_router(state.clone()).with_state(state.clone());
    app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/api/v1/auth/register")
            .header("content-type", "application/json")
            .body(Body::from(json!({
                "password": "supersecret123",
                "email": "first@test.local"
            }).to_string()))
            .unwrap(),
    )
    .await
    .unwrap()
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn first_user_is_admin_true_promotes(pool: PgPool) {
    let state = common::make_state(pool.clone());
    let resp = register_first_user(state).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let user = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT platform_role FROM users WHERE email = 'first@test.local'"
    )
    .fetch_one(&pool).await.unwrap();
    assert_eq!(user.0, Some("platform_admin".to_string()));
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn first_user_is_admin_false_skips_promotion(pool: PgPool) {
    let state = common::make_state_with_auth(pool.clone(), false);
    let resp = register_first_user(state).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let user = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT platform_role FROM users WHERE email = 'first@test.local'"
    )
    .fetch_one(&pool).await.unwrap();
    assert_eq!(user.0, None);
}
