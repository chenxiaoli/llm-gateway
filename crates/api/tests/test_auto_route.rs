//! Integration tests for `model=auto` proxy routing (Task 14).
//!
//! These tests exercise the full proxy pipeline end-to-end: API-key auth →
//! model=auto detection → capability filter → registry resolve_by_pool →
//! upstream forwarding → failover → typed error responses. The upstream
//! provider is a `wiremock` MockServer whose URI is wired into the channel's
//! provider endpoints, so we can assert exactly which model the gateway
//! forwarded to and how many times.
//!
//! Tests use the real `InMemoryChannelRegistry` (NOT the `MockChannelRegistry`
//! from `common`) because `model=auto` depends on `resolve_by_pool`, which
//! the mock returns empty for. After seeding DB rows we trigger a registry
//! reload so the cache reflects the test fixtures.
//!
//! Harness shape mirrors `phase5_enforcement.rs` (build_full_app + oneshot)
//! with the additions: real registry + wiremock upstream + auto_route_config
//! seeding.

mod common;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use axum::routing::post;
use axum::Router;
use llm_gateway_api::management;
use llm_gateway_api::proxy::{self, InMemoryChannelRegistry};
use llm_gateway_api::{AppState, ChannelRegistry};
use llm_gateway_storage::{postgres::PostgresStorage, AutoRouteConfig, AutoRouteConfigData, Storage};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceExt;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Same shape as `phase5_enforcement::build_full_app`.
fn build_full_app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(proxy::proxy_with_protocol))
        .route("/v1/messages", post(proxy::messages))
        .route("/v1/responses", post(proxy::responses))
        .merge(management::management_router(state.clone()))
        .with_state(state)
}

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

async fn body_text(resp: axum::http::Response<Body>) -> String {
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    String::from_utf8_lossy(&bytes).into_owned()
}

async fn body_json(resp: axum::http::Response<Body>) -> Value {
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// Build AppState with the REAL InMemoryChannelRegistry instead of the
/// common::MockChannelRegistry. Required because `model=auto` relies on
/// `resolve_by_pool`, which the mock returns empty for.
async fn make_state_with_real_registry(pool: PgPool) -> (Arc<AppState>, Arc<InMemoryChannelRegistry>) {
    let db = Arc::new(PostgresStorage::from_pool(pool.clone())) as Arc<dyn Storage>;
    let registry = Arc::new(InMemoryChannelRegistry::new(
        db.clone(),
        [0u8; 32],
        Duration::from_secs(3600),
    ));
    let state = Arc::new(AppState {
        storage: db.clone(),
        rate_limiter: Arc::new(llm_gateway_ratelimit::RateLimiter::new(60)),
        jwt_secret: common::TEST_JWT_SECRET.to_string(),
        auth_config: Arc::new(llm_gateway_storage::AuthConfig {
            jwt_secret: common::TEST_JWT_SECRET.to_string(),
            allow_registration: Some(true),
            first_user_is_admin: true,
        }),
        encryption_key: [0u8; 32],
        nats_publisher: None,
        registry: registry.clone() as Arc<dyn llm_gateway_api::ChannelRegistry>,
        system_info: llm_gateway_api::SystemInfo {
            server_bind_address: "0.0.0.0:8080".to_string(),
            database_driver: "postgres".to_string(),
            rate_limit_window_secs: 60,
            rate_limit_flush_interval_secs: 30,
            upstream_timeout_secs: 30,
            audit_retention_days: Some(90),
        },
        public_base_url: "http://localhost:5173".to_string(),
        mailer: Arc::new(llm_gateway_email::noop::NoopMailer::new()),
        templates: Arc::new(
            llm_gateway_email::templates::TemplateRegistry::load(
                "noreply@test.local".to_string(),
                "Test".to_string(),
            )
            .expect("load templates"),
        ),
    });
    (state, registry)
}

/// POST `/v1/chat/completions` with the given JSON body and `model=auto`.
async fn chat_auto(app: &Router, api_key: &str, body: Value) -> axum::http::Response<Body> {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .header("authorization", bearer(api_key))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Seed a provider that points at the mock upstream. The registry reads
/// `endpoints.openai` to find the OpenAI base URL.
async fn seed_provider(pool: &PgPool, org_id: &str, id: &str, openai_endpoint: &str) {
    let endpoints = serde_json::json!({ "openai": openai_endpoint }).to_string();
    sqlx::query(
        r#"INSERT INTO providers (id, owner_org_id, name, slug, base_url, endpoints, proxy_url, enabled, created_at, updated_at)
           VALUES ($1, $2, $3, $4, NULL, $5, NULL, true, NOW(), NOW())"#,
    )
    .bind(id)
    .bind(org_id)
    .bind(format!("Provider {id}"))
    .bind(format!("slug-{id}"))
    .bind(&endpoints)
    .execute(pool)
    .await
    .expect("seed provider");
}

/// Seed an enabled channel that uses the seeded provider and an org-wide
/// encryption-zero key (encryption_key=[0u8;32] means encrypted==plaintext).
async fn seed_channel(pool: &PgPool, org_id: &str, channel_id: &str, provider_id: &str, priority: i32) {
    // Encryption key in AppState is all-zeros, so the channel's stored
    // api_key is decrypted with a no-op; we store a placeholder plaintext.
    let encrypted_api_key = llm_gateway_encryption::encrypt("sk-test-upstream", &[0u8; 32])
        .expect("encrypt upstream key");
    sqlx::query(
        r#"INSERT INTO channels
             (id, org_id, provider_id, name, api_key, base_url, priority, pricing_policy_id,
              markup_ratio, enabled, rpm_limit, tpm_limit, balance, weight, available_hours,
              created_by, group_id, disabled_until, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, NULL, $6, NULL, 0, true, NULL, NULL, NULL, 100, NULL,
                   NULL, NULL, NULL, NOW(), NOW())"#,
    )
    .bind(channel_id)
    .bind(org_id)
    .bind(provider_id)
    .bind(format!("Channel {channel_id}"))
    .bind(&encrypted_api_key)
    .bind(priority)
    .execute(pool)
    .await
    .expect("seed channel");
}

/// Bind a model to a channel via channel_models (the junction table).
async fn seed_channel_model(pool: &PgPool, org_id: &str, channel_id: &str, model_id: &str) {
    sqlx::query(
        r#"INSERT INTO channel_models
             (id, org_id, channel_id, model_id, upstream_model_name, priority_override,
              pricing_policy_id, markup_ratio, enabled, created_at, updated_at)
           VALUES ($1, $2, $3, $4, NULL, NULL, NULL, 0, true, NOW(), NOW())"#,
    )
    .bind(format!("cm-{channel_id}-{model_id}"))
    .bind(org_id)
    .bind(channel_id)
    .bind(model_id)
    .execute(pool)
    .await
    .expect("seed channel_model");
}

/// Insert a model row directly. Bypasses the storage facade because we
/// need to set supports_vision / supports_tools, which the management
/// endpoint hasn't been wired up for yet (T18 is the frontend task).
async fn seed_model(
    pool: &PgPool,
    org_id: &str,
    id: &str,
    name: &str,
    supports_vision: bool,
    supports_tools: bool,
) {
    sqlx::query(
        r#"INSERT INTO models (id, owner_org_id, name, model_type, pricing_policy_id,
                                supports_vision, supports_tools, created_at)
           VALUES ($1, $2, $3, NULL, NULL, $4, $5, NOW())"#,
    )
    .bind(id)
    .bind(org_id)
    .bind(name)
    .bind(supports_vision)
    .bind(supports_tools)
    .execute(pool)
    .await
    .expect("seed model");
}

/// Seed an auto_route_config and return its id.
async fn seed_auto_route_config(pool: &PgPool, id: &str, name: &str, model_names: Vec<String>) -> String {
    let config = AutoRouteConfig {
        id: id.to_string(),
        name: name.to_string(),
        config: AutoRouteConfigData { model_names },
        created_by: None,
        created_at: chrono::Utc::now(),
    };
    sqlx::query(
        r#"INSERT INTO auto_route_configs (id, name, config, created_by, created_at)
           VALUES ($1, $2, $3, NULL, $4)"#,
    )
    .bind(&config.id)
    .bind(&config.name)
    .bind(serde_json::to_string(&config.config).unwrap())
    .bind(config.created_at)
    .execute(pool)
    .await
    .expect("seed auto_route_config");
    id.to_string()
}

/// Common fixture: seed a fresh org_default-scoped ecosystem (org_default is
/// already created by migrations), an api_key with `auto_route_id`, and
/// return the plaintext key + the org_id ("org_default").
async fn seed_api_key_with_auto_route(
    pool: &PgPool,
    auto_route_id: Option<&str>,
) -> String {
    // The InMemoryChannelRegistry hard-codes org_id = "org_default" (Phase 1
    // single-org assumption). So we must put all fixtures under org_default.
    let org_id = "org_default";
    let tag = uuid::Uuid::new_v4().to_string();
    let user_id = format!("u-{tag}");

    sqlx::query(
        r#"INSERT INTO users (id, username, password, platform_role, current_org_id, enabled, created_at, updated_at)
           VALUES ($1, $2, 'x', NULL, $3, true, NOW(), NOW())"#,
    )
    .bind(&user_id)
    .bind(&user_id)
    .bind(org_id)
    .execute(pool)
    .await
    .expect("seed user");

    // Owner member so the routing filter doesn't apply (request_is_admin=true).
    sqlx::query(
        r#"INSERT INTO members (user_id, org_id, role, created_by, created_at)
           VALUES ($1, $2, 'owner', $1, NOW())"#,
    )
    .bind(&user_id)
    .bind(org_id)
    .execute(pool)
    .await
    .expect("seed member");

    let plaintext = llm_gateway_auth::generate_api_key();
    let key_hash = llm_gateway_auth::hash_api_key(&plaintext);
    sqlx::query(
        r#"INSERT INTO api_keys
             (id, org_id, name, key_hash, key_prefix, rate_limit, budget_monthly,
              enabled, created_by, model_fallback_id, auto_route_id, created_at, updated_at)
           VALUES ($1, $2, 'test-key', $3, NULL, NULL, NULL, true, $4, NULL, $5, NOW(), NOW())"#,
    )
    .bind(format!("key-{tag}"))
    .bind(org_id)
    .bind(&key_hash)
    .bind(&user_id)
    .bind(auto_route_id)
    .execute(pool)
    .await
    .expect("seed api_key");

    plaintext
}

/// Stand up the mock upstream + a per-test DB seed, returning everything the
/// test needs: app, mock server, and the api_key plaintext.
struct AutoFixture {
    app: Router,
    mock_server: MockServer,
    api_key: String,
}

/// Build the full fixture used by most tests:
/// - 2 models (text_only and vision_or_tools depending on flags)
/// - 2 channels, each bound to one model, both pointing at the same mock
/// - 1 provider pointing at the mock_server
/// - 1 auto_route_config with both model names
/// - 1 api_key with auto_route_id set
///
/// `vision_a` / `tools_a` configure model_a; `vision_b` / `tools_b` configure model_b.
/// After seeding we trigger a registry reload so the cache sees the new state.
async fn build_fixture(
    pool: &PgPool,
    name_a: &str,
    vision_a: bool,
    tools_a: bool,
    name_b: &str,
    vision_b: bool,
    tools_b: bool,
    pool_model_names: Vec<String>,
) -> AutoFixture {
    let mock_server = MockServer::start().await;
    let (state, registry) = make_state_with_real_registry(pool.clone()).await;
    let app = build_full_app(state);

    let org_id = "org_default";
    let provider_id = "prov-test";
    seed_provider(pool, org_id, provider_id, &mock_server.uri()).await;

    let model_a_id = format!("model-{name_a}");
    let model_b_id = format!("model-{name_b}");
    seed_model(pool, org_id, &model_a_id, name_a, vision_a, tools_a).await;
    seed_model(pool, org_id, &model_b_id, name_b, vision_b, tools_b).await;

    let channel_a = format!("chan-{name_a}");
    let channel_b = format!("chan-{name_b}");
    seed_channel(pool, org_id, &channel_a, provider_id, 1).await;
    seed_channel(pool, org_id, &channel_b, provider_id, 2).await;
    seed_channel_model(pool, org_id, &channel_a, &model_a_id).await;
    seed_channel_model(pool, org_id, &channel_b, &model_b_id).await;

    let config_id = seed_auto_route_config(pool, "arc-test", "test-pool", pool_model_names).await;
    let api_key = seed_api_key_with_auto_route(pool, Some(&config_id)).await;

    // Refresh the registry cache so it sees the seeded channels.
    registry.reload().await;

    AutoFixture { app, mock_server, api_key }
}

// ─── Tests ────────────────────────────────────────────────────────────────

/// 1. Key has `auto_route_id = NULL` → 400 + body code `auto_not_configured`.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn auto_without_config_returns_400(pool: PgPool) {
    let (state, _registry) = make_state_with_real_registry(pool.clone()).await;
    let app = build_full_app(state);

    let api_key = seed_api_key_with_auto_route(&pool, None).await;

    let resp = chat_auto(
        &app,
        &api_key,
        json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "hi"}],
        }),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = body_text(resp).await;
    assert!(
        body.contains("auto_not_configured"),
        "expected body to contain 'auto_not_configured', got: {}",
        body
    );
}

/// 2. Body has image_url → routes to vision-capable model. Mock records
///    exactly ONE upstream hit, and that hit's model name is the vision one.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn auto_with_vision_routes_to_vision_capable_model(pool: PgPool) {
    let f = build_fixture(
        &pool,
        "text-only", false, false,
        "vision-model", true, false,
        vec!["text-only".to_string(), "vision-model".to_string()],
    )
    .await;

    // Upstream always returns 200; we just count + inspect the recorded model.
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "x", "object": "chat.completion",
            "choices": [{"message": {"role": "assistant", "content": "ok"}, "index": 0}],
            "usage": {"prompt_tokens": 5, "completion_tokens": 3},
        })))
        .expect(1)
        .mount(&f.mock_server)
        .await;

    let resp = chat_auto(
        &f.app,
        &f.api_key,
        json!({
            "model": "auto",
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": "describe this"},
                    {"type": "image_url", "image_url": {"url": "https://example.com/x.png"}}
                ]
            }],
        }),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::OK);
    let recorded = &f.mock_server.received_requests().await.unwrap();
    assert_eq!(recorded.len(), 1, "exactly one upstream call expected");
    let body_str = String::from_utf8_lossy(&recorded[0].body);
    let body: Value = serde_json::from_slice(&recorded[0].body).unwrap_or_else(|_| json!({}));
    let model_hit = body
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or_else(|| panic!("upstream body missing model field: {body_str}"));
    assert_eq!(
        model_hit, "vision-model",
        "vision-required request must hit the vision-capable model; got {model_hit}"
    );
}

/// 3. Body has `tools` array → routes to tools-capable model.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn auto_with_tools_routes_to_tools_capable_model(pool: PgPool) {
    let f = build_fixture(
        &pool,
        "text-only", false, false,
        "tools-model", false, true,
        vec!["text-only".to_string(), "tools-model".to_string()],
    )
    .await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "x", "object": "chat.completion",
            "choices": [{"message": {"role": "assistant", "content": "ok"}, "index": 0}],
            "usage": {"prompt_tokens": 5, "completion_tokens": 3},
        })))
        .expect(1)
        .mount(&f.mock_server)
        .await;

    let resp = chat_auto(
        &f.app,
        &f.api_key,
        json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "use the tool"}],
            "tools": [{"type": "function", "function": {"name": "do_it"}}],
        }),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::OK);
    let recorded = &f.mock_server.received_requests().await.unwrap();
    assert_eq!(recorded.len(), 1);
    let body: Value = serde_json::from_slice(&recorded[0].body).unwrap();
    let model_hit = body.get("model").and_then(|m| m.as_str()).unwrap();
    assert_eq!(model_hit, "tools-model");
}

/// 4. Text-only body, pool of 2 → either is acceptable, exactly 1 upstream call.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn auto_with_no_capabilities_routes_to_any_pool_model(pool: PgPool) {
    let f = build_fixture(
        &pool,
        "alpha", false, false,
        "beta", false, false,
        vec!["alpha".to_string(), "beta".to_string()],
    )
    .await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "x", "object": "chat.completion",
            "choices": [{"message": {"role": "assistant", "content": "ok"}, "index": 0}],
            "usage": {"prompt_tokens": 5, "completion_tokens": 3},
        })))
        .expect(1)
        .mount(&f.mock_server)
        .await;

    let resp = chat_auto(
        &f.app,
        &f.api_key,
        json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "hi"}],
        }),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::OK);
    let recorded = &f.mock_server.received_requests().await.unwrap();
    assert_eq!(recorded.len(), 1, "exactly one upstream call expected");
    let body: Value = serde_json::from_slice(&recorded[0].body).unwrap();
    let model_hit = body.get("model").and_then(|m| m.as_str()).unwrap();
    assert!(
        model_hit == "alpha" || model_hit == "beta",
        "must hit one of the pool models; got {model_hit}"
    );
}

/// 5. Body requires vision, pool has only text-only → 400 + code
///    `auto_no_matching_model` + required_capabilities contains "vision".
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn auto_with_unsatisfiable_capabilities_returns_400(pool: PgPool) {
    let f = build_fixture(
        &pool,
        "only-text", false, false,
        "decoy", false, false,
        vec!["only-text".to_string(), "decoy".to_string()],
    )
    .await;

    // No upstream mock mounted — request must fail BEFORE forwarding.
    let resp = chat_auto(
        &f.app,
        &f.api_key,
        json!({
            "model": "auto",
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": "describe this"},
                    {"type": "image_url", "image_url": {"url": "https://example.com/x.png"}}
                ]
            }],
        }),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = body_json(resp).await;
    assert_eq!(body["error"]["code"], "auto_no_matching_model");
    let required = body["error"]["required_capabilities"]
        .as_array()
        .expect("required_capabilities is an array");
    assert!(
        required.iter().any(|v| v == "vision"),
        "required_capabilities must include 'vision'; got {required:?}"
    );

    // No upstream call should have been made.
    let recorded = f.mock_server.received_requests().await.unwrap();
    assert!(recorded.is_empty(), "no upstream call expected on auto_no_matching_model");
}

/// 6. Failover: first model's channel returns 500, second returns 200.
///    Assert: 200 response, 2 upstream calls, second call's model = model_B.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn auto_failover_across_models_when_first_model_channels_fail(pool: PgPool) {
    // Set up so model_A has priority 1 (always tried first) and model_B has
    // priority 2 (fallback). Both are text-only.
    let mock_server = MockServer::start().await;
    let (state, registry) = make_state_with_real_registry(pool.clone()).await;
    let app = build_full_app(state);

    let org_id = "org_default";
    let provider_id = "prov-failover";
    seed_provider(&pool, org_id, provider_id, &mock_server.uri()).await;

    seed_model(&pool, org_id, "model-A-id", "model-A", false, false).await;
    seed_model(&pool, org_id, "model-B-id", "model-B", false, false).await;

    seed_channel(&pool, org_id, "chan-A", provider_id, 1).await;
    seed_channel(&pool, org_id, "chan-B", provider_id, 2).await;
    seed_channel_model(&pool, org_id, "chan-A", "model-A-id").await;
    seed_channel_model(&pool, org_id, "chan-B", "model-B-id").await;

    let config_id = seed_auto_route_config(
        &pool,
        "arc-failover",
        "failover-pool",
        vec!["model-A".to_string(), "model-B".to_string()],
    )
    .await;
    let api_key = seed_api_key_with_auto_route(&pool, Some(&config_id)).await;

    registry.reload().await;

    // Two expectations: each channel hits a different path because they share
    // the upstream URL. We match by request body's model field instead.
    // wiremock lets us mount multiple mocks; both are matched in order.
    Mock::given(method("POST"))
        .and(wiremock::matchers::body_partial_json(json!({"model": "model-A"})))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({"error": "upstream boom"})))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;
    Mock::given(method("POST"))
        .and(wiremock::matchers::body_partial_json(json!({"model": "model-B"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "x", "object": "chat.completion",
            "choices": [{"message": {"role": "assistant", "content": "ok"}, "index": 0}],
            "usage": {"prompt_tokens": 5, "completion_tokens": 3},
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    let resp = chat_auto(
        &app,
        &api_key,
        json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "hi"}],
        }),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::OK);
    let recorded = &mock_server.received_requests().await.unwrap();
    assert_eq!(recorded.len(), 2, "expected 2 upstream calls (failover), got {}", recorded.len());
    let second_body: Value = serde_json::from_slice(&recorded[1].body).unwrap();
    let second_model = second_body.get("model").and_then(|m| m.as_str()).unwrap();
    assert_eq!(second_model, "model-B", "second (successful) call must be model-B");
}

/// 7. All candidates fail → 502 + code `auto_all_candidates_failed`.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn auto_all_candidates_failed_returns_502(pool: PgPool) {
    let f = build_fixture(
        &pool,
        "doomed-A", false, false,
        "doomed-B", false, false,
        vec!["doomed-A".to_string(), "doomed-B".to_string()],
    )
    .await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({"error": "upstream boom"})))
        .mount(&f.mock_server)
        .await;

    let resp = chat_auto(
        &f.app,
        &f.api_key,
        json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "hi"}],
        }),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    let body = body_text(resp).await;
    assert!(
        body.contains("auto_all_candidates_failed"),
        "expected body to contain 'auto_all_candidates_failed', got: {}",
        body
    );
}

/// 8. Org has 3 vision-capable models; pool includes only 1. Body requires
///    vision. Assert: only the pooled model was hit; the other 2 never
///    received an upstream call.
#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn auto_ignores_models_outside_pool_even_if_capable(pool: PgPool) {
    let mock_server = MockServer::start().await;
    let (state, registry) = make_state_with_real_registry(pool.clone()).await;
    let app = build_full_app(state);

    let org_id = "org_default";
    let provider_id = "prov-pool-filter";
    seed_provider(&pool, org_id, provider_id, &mock_server.uri()).await;

    // 3 vision-capable models. Only "in-pool" is in the auto_route_config.
    seed_model(&pool, org_id, "in-pool-id", "in-pool", true, false).await;
    seed_model(&pool, org_id, "out-pool-1-id", "out-pool-1", true, false).await;
    seed_model(&pool, org_id, "out-pool-2-id", "out-pool-2", true, false).await;

    // 3 channels, each bound to one model, so each model has a wireable route.
    seed_channel(&pool, org_id, "chan-in-pool", provider_id, 1).await;
    seed_channel(&pool, org_id, "chan-out-1", provider_id, 2).await;
    seed_channel(&pool, org_id, "chan-out-2", provider_id, 3).await;
    seed_channel_model(&pool, org_id, "chan-in-pool", "in-pool-id").await;
    seed_channel_model(&pool, org_id, "chan-out-1", "out-pool-1-id").await;
    seed_channel_model(&pool, org_id, "chan-out-2", "out-pool-2-id").await;

    let config_id = seed_auto_route_config(
        &pool,
        "arc-pool-filter",
        "pool-filter",
        vec!["in-pool".to_string()],
    )
    .await;
    let api_key = seed_api_key_with_auto_route(&pool, Some(&config_id)).await;

    registry.reload().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "x", "object": "chat.completion",
            "choices": [{"message": {"role": "assistant", "content": "ok"}, "index": 0}],
            "usage": {"prompt_tokens": 5, "completion_tokens": 3},
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let resp = chat_auto(
        &app,
        &api_key,
        json!({
            "model": "auto",
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": "describe"},
                    {"type": "image_url", "image_url": {"url": "https://example.com/x.png"}}
                ]
            }],
        }),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::OK);
    let recorded = &mock_server.received_requests().await.unwrap();
    assert_eq!(recorded.len(), 1, "only the pooled model should be hit");
    let body: Value = serde_json::from_slice(&recorded[0].body).unwrap();
    let model_hit = body.get("model").and_then(|m| m.as_str()).unwrap();
    assert_eq!(
        model_hit, "in-pool",
        "must hit ONLY the in-pool model; got {model_hit}"
    );
}
