use axum::http::StatusCode;
use axum::response::IntoResponse;
use llm_gateway_api::error::ApiError;

#[tokio::test]
async fn rate_limited_emits_retry_after_header() {
    let resp = ApiError::RateLimited {
        retry_after_secs: 60,
    }
    .into_response();
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    let retry_after = resp
        .headers()
        .get("retry-after")
        .expect("Retry-After header missing")
        .to_str()
        .unwrap();
    assert_eq!(retry_after, "60");

    // Body still carries the standard error envelope.
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["error"]["type"], 429);
}
