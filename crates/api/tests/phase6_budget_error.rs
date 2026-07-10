//! Integration test for ApiError::BudgetExceeded response shape.

mod common;

use axum::body::to_bytes;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use llm_gateway_api::error::ApiError;
use serde_json::Value;

#[tokio::test]
async fn budget_exceeded_renders_429_with_body() {
    let err = ApiError::BudgetExceeded {
        key_id: "key_abc".into(),
        month_bucket: "2026-07".into(),
        limit_units: 5_000_000_000,    // $50
        accrued_units: 5_230_000_000,  // $52.30
    };
    let resp = err.into_response();
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);

    // No Retry-After header (budget exceeded is not a wait scenario).
    assert!(resp.headers().get("retry-after").is_none());

    let body = to_bytes(resp.into_body(), 1024 * 16).await.unwrap();
    let v: Value = serde_json::from_slice(&body).unwrap();
    let err_obj = &v["error"];

    assert_eq!(err_obj["type"], "budget_exceeded");
    assert_eq!(err_obj["key_id"], "key_abc");
    assert_eq!(err_obj["month_bucket"], "2026-07");
    assert_eq!(err_obj["limit"], 50.0);
    assert_eq!(err_obj["accrued"], 52.3);
    assert!(err_obj["message"].as_str().unwrap().contains("budget exceeded"));
}
