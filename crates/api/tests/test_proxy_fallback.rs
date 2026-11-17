//! Proxy tests for the model-fallback chain.
//!
//! The upstream is a real HTTP server on localhost (the proxy talks to it over
//! reqwest, exactly as it would to a provider), the storage is an in-memory
//! double, and the event publisher records the audit events the proxy emits —
//! so the whole request path runs without NATS or PostgreSQL.

mod common;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::any;
use axum::Router;

use llm_gateway_api::proxy::proxy_with_protocol;
use llm_gateway_api::{AppState, ChannelRegistry, ResolvedChannel, SystemInfo};
use llm_gateway_auth::hash_api_key;
use llm_gateway_nats_publisher::{AuditEvent, EventPublisher, StreamStatusInfo, UsageEvent};
use llm_gateway_ratelimit::RateLimiter;
use llm_gateway_storage::*;

use common::mock_storage::MockStorage;

// ── Test doubles ───────────────────────────────────────────────────────────

/// Registry that never resolves anything, so the proxy takes its database
/// routing path (models → channel_models → channels).
struct EmptyRegistry;

#[async_trait::async_trait]
impl ChannelRegistry for EmptyRegistry {
    async fn resolve_by_model(&self, _model: &str) -> Vec<ResolvedChannel> {
        Vec::new()
    }
    async fn resolve(&self, _channel_id: &str) -> Option<ResolvedChannel> {
        None
    }
    async fn reload(&self) {}
    fn disable_channel_model(&self, _channel_id: &str, _model_name: &str, _until: Instant) {}
    fn is_circuit_broken(&self, _channel_id: &str, _model_name: &str) -> bool {
        false
    }
}

/// Publisher that records what the proxy emits.
#[derive(Default)]
struct RecordingPublisher {
    audits: Mutex<Vec<AuditEvent>>,
    usages: Mutex<Vec<UsageEvent>>,
}

#[async_trait::async_trait]
impl EventPublisher for RecordingPublisher {
    async fn publish_usage(
        &self,
        event: &UsageEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.usages.lock().unwrap().push(event.clone());
        Ok(())
    }

    async fn publish_audit(
        &self,
        event: &AuditEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.audits.lock().unwrap().push(event.clone());
        Ok(())
    }

    async fn stream_info(&self, _name: &str) -> Result<StreamStatusInfo, String> {
        Err("no streams in tests".to_string())
    }
}

/// The fake upstream: records the model of every request it receives and
/// answers with whatever the test configured for that model.
#[derive(Clone)]
struct FakeUpstream {
    base_url: String,
    hits: Arc<Mutex<Vec<String>>>,
}

impl FakeUpstream {
    async fn start(responses: HashMap<String, (u16, String)>) -> Self {
        let hits: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let state = (hits.clone(), Arc::new(responses));

        let app = Router::new()
            .fallback(any(upstream_handler))
            .with_state(state);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        FakeUpstream {
            base_url: format!("http://127.0.0.1:{}", port),
            hits,
        }
    }

    /// Models the upstream was asked for, in order.
    fn hits(&self) -> Vec<String> {
        self.hits.lock().unwrap().clone()
    }
}

async fn upstream_handler(
    State((hits, responses)): State<(Arc<Mutex<Vec<String>>>, Arc<HashMap<String, (u16, String)>>)>,
    body: Bytes,
) -> Response {
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
    let model = parsed
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("")
        .to_string();
    hits.lock().unwrap().push(model.clone());

    let (status, payload) = responses
        .get(&model)
        .cloned()
        .unwrap_or((200, format!("{{\"served_by\":\"{}\"}}", model)));

    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(payload))
        .unwrap()
}

// ── Fixture ────────────────────────────────────────────────────────────────

const RAW_KEY: &str = "sk-test-key";

struct Fixture {
    state: Arc<AppState>,
    publisher: Arc<RecordingPublisher>,
    upstream: FakeUpstream,
}

/// Builds a gateway with one provider, two channels for `glm-5.1` and one for
/// `minimax-m3.1-flash`, and (optionally) a fallback config binding the two
/// models together.
async fn fixture(
    responses: HashMap<String, (u16, String)>,
    with_fallback: bool,
) -> Fixture {
    let upstream = FakeUpstream::start(responses).await;

    let storage = MockStorage::default();

    storage.keys.lock().unwrap().push(ApiKey {
        id: "key-1".to_string(),
        name: "test".to_string(),
        key_hash: hash_api_key(RAW_KEY),
        key_prefix: None,
        rate_limit: None,
        budget_monthly: None,
        enabled: true,
        created_by: None,
        model_fallback_id: if with_fallback {
            Some("fb-1".to_string())
        } else {
            None
        },
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    });

    storage.models.lock().unwrap().push(Model {
        id: "m-glm".to_string(),
        name: "glm-5.1".to_string(),
        model_type: None,
        pricing_policy_id: None,
        created_at: chrono::Utc::now(),
    });
    storage.models.lock().unwrap().push(Model {
        id: "m-minimax".to_string(),
        name: "minimax-m3.1-flash".to_string(),
        model_type: None,
        pricing_policy_id: None,
        created_at: chrono::Utc::now(),
    });

    storage.providers.lock().unwrap().push(Provider {
        id: "p-1".to_string(),
        name: "fake".to_string(),
        slug: "fake".to_string(),
        endpoints: Some(
            serde_json::json!({ "openai": upstream.base_url }).to_string(),
        ),
        proxy_url: None,
        enabled: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    });

    for (id, name, priority) in [("c-1", "channel-1", 1), ("c-2", "channel-2", 2)] {
        storage.channels.lock().unwrap().push(Channel {
            id: id.to_string(),
            provider_id: "p-1".to_string(),
            name: name.to_string(),
            api_key: "upstream-key".to_string(),
            priority,
            pricing_policy_id: None,
            markup_ratio: 0,
            rpm_limit: None,
            tpm_limit: None,
            balance: None,
            weight: None,
            enabled: true,
            disabled_until: None,
            available_hours: None,
            created_by: None,
            group_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
    }

    // glm-5.1 is served by both channels; minimax-m3.1-flash by the first.
    for (id, model_id, channel_id) in [
        ("cm-1", "m-glm", "c-1"),
        ("cm-2", "m-glm", "c-2"),
        ("cm-3", "m-minimax", "c-1"),
    ] {
        storage.channel_models.lock().unwrap().push(ChannelModel {
            id: id.to_string(),
            channel_id: channel_id.to_string(),
            model_id: model_id.to_string(),
            upstream_model_name: None,
            priority_override: None,
            pricing_policy_id: None,
            markup_ratio: 0,
            enabled: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
    }

    if with_fallback {
        storage.fallbacks.lock().unwrap().push(ModelFallbackConfig {
            id: "fb-1".to_string(),
            name: "glm ↔ minimax".to_string(),
            config: vec![ModelFallbackGroup {
                models: vec!["glm-5.1".to_string(), "minimax-m3.1-flash".to_string()],
                priorities: vec![1, 2],
            }],
            created_by: None,
            created_at: chrono::Utc::now(),
        });
    }

    let publisher = Arc::new(RecordingPublisher::default());

    let state = Arc::new(AppState {
        storage: Arc::new(storage) as Arc<dyn Storage>,
        rate_limiter: Arc::new(RateLimiter::new(60)),
        jwt_secret: "test".to_string(),
        encryption_key: [0u8; 32],
        nats_publisher: Some(publisher.clone() as Arc<dyn EventPublisher>),
        registry: Arc::new(EmptyRegistry),
        system_info: SystemInfo {
            server_bind_address: "127.0.0.1:0".to_string(),
            database_driver: "memory".to_string(),
            rate_limit_window_secs: 60,
            rate_limit_flush_interval_secs: 30,
            upstream_timeout_secs: 30,
            audit_retention_days: None,
        },
    });

    Fixture {
        state,
        publisher,
        upstream,
    }
}

async fn proxy_request(state: Arc<AppState>, model: &str) -> (u16, String) {
    let mut headers = HeaderMap::new();
    headers.insert(
        "authorization",
        format!("Bearer {}", RAW_KEY).parse().unwrap(),
    );
    let body = format!("{{\"model\":\"{}\",\"messages\":[{{\"role\":\"user\",\"content\":\"hi\"}}]}}", model);

    match proxy_with_protocol(axum::extract::State(state), headers, body).await {
        Ok(response) => {
            let status = response.status().as_u16();
            let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            (status, String::from_utf8_lossy(&bytes).to_string())
        }
        // The axum layer turns an ApiError into the same status/body the
        // client sees; the proxy itself returns it as an error value.
        Err(err) => (err.status_code(), err.message()),
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn upstream_4xx_exhausts_the_models_channels_before_the_fallback_runs() {
    let fixture = fixture(
        HashMap::from([
            ("glm-5.1".to_string(), (404u16, "{\"error\":\"no such model\"}".to_string())),
            (
                "minimax-m3.1-flash".to_string(),
                (200u16, "{\"served_by\":\"minimax\"}".to_string()),
            ),
        ]),
        true,
    )
    .await;

    let (status, body) = proxy_request(fixture.state.clone(), "glm-5.1").await;

    // The fallback served the request.
    assert_eq!(status, 200);
    assert!(body.contains("minimax"), "body: {}", body);

    // Both channels of glm-5.1 were tried first, then the fallback model.
    assert_eq!(
        fixture.upstream.hits(),
        vec!["glm-5.1", "glm-5.1", "minimax-m3.1-flash"],
        "the model's channels must be exhausted before the fallback runs"
    );

    // Exactly one audit event, carrying the whole chain.
    let audits = fixture.publisher.audits.lock().unwrap();
    assert_eq!(audits.len(), 1, "one request must produce one audit event");
    let audit = &audits[0];
    assert_eq!(audit.status_code, 200);
    assert_eq!(audit.original_model.as_deref(), Some("glm-5.1"));
    assert_eq!(audit.model_name, "minimax-m3.1-flash");
    assert_eq!(
        audit.model_override_reason.as_deref(),
        Some("model_fallback")
    );

    let routes = audit.routes.as_ref().expect("routes must be recorded");
    assert_eq!(routes.len(), 3, "routes: {:?}", routes);
    assert_eq!(routes[0].model, "glm-5.1");
    assert_eq!(routes[0].status_code, 404);
    assert_eq!(routes[1].model, "glm-5.1");
    assert_eq!(routes[1].status_code, 404);
    assert_eq!(routes[2].model, "minimax-m3.1-flash");
    assert_eq!(routes[2].status_code, 200);
}

#[tokio::test]
async fn without_a_fallback_the_upstream_error_is_returned_verbatim() {
    let fixture = fixture(
        HashMap::from([(
            "glm-5.1".to_string(),
            (404u16, "{\"error\":\"no such model\"}".to_string()),
        )]),
        false,
    )
    .await;

    let (status, body) = proxy_request(fixture.state.clone(), "glm-5.1").await;

    assert_eq!(status, 404);
    assert!(body.contains("no such model"), "body: {}", body);

    // Both channels were tried, and no fallback was.
    assert_eq!(fixture.upstream.hits(), vec!["glm-5.1", "glm-5.1"]);

    let audits = fixture.publisher.audits.lock().unwrap();
    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0].status_code, 404);
    let routes = audits[0].routes.as_ref().unwrap();
    assert_eq!(routes.len(), 2);
    assert!(routes.iter().all(|r| r.model == "glm-5.1"));
}

#[tokio::test]
async fn a_failing_fallback_still_returns_the_original_upstream_error() {
    let fixture = fixture(
        HashMap::from([
            ("glm-5.1".to_string(), (503u16, "{\"error\":\"unavailable\"}".to_string())),
            (
                "minimax-m3.1-flash".to_string(),
                (500u16, "{\"error\":\"also down\"}".to_string()),
            ),
        ]),
        true,
    )
    .await;

    let (status, body) = proxy_request(fixture.state.clone(), "glm-5.1").await;

    // A pure 5xx chain has no upstream error to hand back — the client gets
    // the gateway's own "all channels failed" error (the 4xx path is the one
    // that passes the upstream body through).
    assert_eq!(status, 502);
    assert!(body.contains("All channels failed"), "body: {}", body);

    // Both channels of glm-5.1, then the fallback model.
    assert_eq!(
        fixture.upstream.hits(),
        vec!["glm-5.1", "glm-5.1", "minimax-m3.1-flash"]
    );

    let audits = fixture.publisher.audits.lock().unwrap();
    assert_eq!(audits.len(), 1, "the chain must not produce a row per attempt");
    assert_eq!(audits[0].status_code, 503);
    let routes = audits[0].routes.as_ref().unwrap();
    assert_eq!(routes.len(), 3);
}

#[tokio::test]
async fn a_fallback_candidate_that_errors_does_not_end_the_chain() {
    let fixture = fixture(
        HashMap::from([
            ("glm-5.1".to_string(), (404u16, "{\"error\":\"no such model\"}".to_string())),
            (
                "minimax-m3.1-flash".to_string(),
                (404u16, "{\"error\":\"not here either\"}".to_string()),
            ),
        ]),
        true,
    )
    .await;

    let (status, _body) = proxy_request(fixture.state.clone(), "glm-5.1").await;

    assert_eq!(status, 404);
    // The failing fallback candidate was tried, and the chain continued.
    assert_eq!(
        fixture.upstream.hits(),
        vec!["glm-5.1", "glm-5.1", "minimax-m3.1-flash"]
    );

    let audits = fixture.publisher.audits.lock().unwrap();
    assert_eq!(audits.len(), 1);
    let routes = audits[0].routes.as_ref().unwrap();
    assert_eq!(routes.len(), 3);
    assert_eq!(routes[2].model, "minimax-m3.1-flash");
    assert_eq!(routes[2].status_code, 404);
}

#[tokio::test]
async fn an_unknown_model_never_reaches_an_upstream() {
    let fixture = fixture(
        HashMap::from([(
            "minimax-m3.1-flash".to_string(),
            (200u16, "{\"served_by\":\"minimax\"}".to_string()),
        )]),
        true,
    )
    .await;

    let (status, body) = proxy_request(fixture.state.clone(), "not-a-model").await;

    // The model is not in the registry → the fallback chain has no group for
    // it → the client gets a 404 and no upstream was contacted.
    assert_eq!(status, 404);
    assert!(body.contains("not found"), "body: {}", body);
    assert!(fixture.upstream.hits().is_empty());

    // The routing failure is audited now (it used to vanish entirely).
    let audits = fixture.publisher.audits.lock().unwrap();
    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0].status_code, 404);
    let routes = audits[0].routes.as_ref().unwrap();
    assert_eq!(routes.len(), 1);
    assert!(routes[0]
        .error_message
        .as_ref()
        .unwrap()
        .contains("not in registry"));
}
