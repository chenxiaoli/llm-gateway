mod common;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use llm_gateway_api::management;
use llm_gateway_api::AppState;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

fn build_app(state: Arc<AppState>) -> axum::Router {
    management::management_router().with_state(state)
}

fn bearer_token(token: &str) -> String {
    format!("Bearer {}", token)
}

/// Helper: register an admin user via the auth endpoint and return the parsed response body.
async fn register_admin(app: &axum::Router) -> Value {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "admin", "password": "password123"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap()
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_create_group_succeeds(pool: PgPool) {
    let app = build_app(common::make_state(pool));

    let admin_body = register_admin(&app).await;
    let admin_token = admin_body["token"].as_str().unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/groups")
                .header("authorization", bearer_token(admin_token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "engineering"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &to_bytes(resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(body["name"], "engineering");
    assert!(body["id"].as_str().is_some());
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_duplicate_group_name_returns_409(pool: PgPool) {
    let app = build_app(common::make_state(pool));

    let admin_body = register_admin(&app).await;
    let admin_token = admin_body["token"].as_str().unwrap();

    // First create succeeds
    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/groups")
                .header("authorization", bearer_token(admin_token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "engineering"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);

    // Second create with same name returns 409
    let second = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/groups")
                .header("authorization", bearer_token(admin_token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "engineering"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::CONFLICT);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_list_groups_returns_all(pool: PgPool) {
    let app = build_app(common::make_state(pool));

    let admin_body = register_admin(&app).await;
    let admin_token = admin_body["token"].as_str().unwrap();

    // Create three groups
    for name in ["engineering", "marketing", "finance"] {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/groups")
                    .header("authorization", bearer_token(admin_token))
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"name": name}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // List returns all three
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/groups?page=1&page_size=20")
                .header("authorization", bearer_token(admin_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &to_bytes(resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(body["total"], 3);
    assert_eq!(body["items"].as_array().unwrap().len(), 3);
    assert_eq!(body["items"][0]["name"], "engineering");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_delete_group_clears_user_channel_references(pool: PgPool) {
    let app = build_app(common::make_state(pool));

    let admin_body = register_admin(&app).await;
    let admin_token = admin_body["token"].as_str().unwrap();

    // Create a group
    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/groups")
                .header("authorization", bearer_token(admin_token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "engineering"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::OK);
    let group_body: Value = serde_json::from_slice(
        &to_bytes(create_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    let group_id = group_body["id"].as_str().unwrap();

    // Delete the group — no users/channels reference it, so counts are 0
    let delete_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/v1/admin/groups/{}", group_id))
                .header("authorization", bearer_token(admin_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &to_bytes(delete_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(body["cleared_users"], 0);
    assert_eq!(body["cleared_channels"], 0);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_update_user_group_id_assigns_group(pool: PgPool) {
    let app = build_app(common::make_state(pool));
    let admin_body = register_admin(&app).await;
    let admin_token = admin_body["token"].as_str().unwrap();

    // Register a regular user
    let user_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(json!({"username": "regular", "password": "password123"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(user_resp.status(), StatusCode::OK);
    let user_body: Value = serde_json::from_slice(&to_bytes(user_resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    let user_id = user_body["user"]["id"].as_str().unwrap();

    // Create a group
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/groups")
                .header("authorization", bearer_token(admin_token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "engineering"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    let group_id = body["id"].as_str().unwrap();

    // Assign user to group
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/v1/admin/users/{}", user_id))
                .header("authorization", bearer_token(admin_token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"group_id": group_id}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["group_id"], group_id);
    assert_eq!(body["group_name"], "engineering");
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_update_user_group_id_nonexistent_returns_400(pool: PgPool) {
    let app = build_app(common::make_state(pool));
    let admin_body = register_admin(&app).await;
    let admin_token = admin_body["token"].as_str().unwrap();

    // Register a regular user
    let user_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(json!({"username": "regular", "password": "password123"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(user_resp.status(), StatusCode::OK);
    let user_body: Value = serde_json::from_slice(&to_bytes(user_resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    let user_id = user_body["user"]["id"].as_str().unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/v1/admin/users/{}", user_id))
                .header("authorization", bearer_token(admin_token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"group_id": "nonexistent-group-id"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn test_update_user_clear_group_id_with_null(pool: PgPool) {
    let app = build_app(common::make_state(pool));
    let admin_body = register_admin(&app).await;
    let admin_token = admin_body["token"].as_str().unwrap();

    // Register a regular user
    let user_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(json!({"username": "regular", "password": "password123"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(user_resp.status(), StatusCode::OK);
    let user_body: Value = serde_json::from_slice(&to_bytes(user_resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    let user_id = user_body["user"]["id"].as_str().unwrap();

    // Clear group_id (no prior assignment)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/v1/admin/users/{}", user_id))
                .header("authorization", bearer_token(admin_token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"group_id": null}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert!(body["group_id"].is_null());
}