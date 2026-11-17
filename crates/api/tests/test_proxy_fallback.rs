//! Proxy tests for routing, failover and the model-fallback chain.
//!
//! The upstream is a real HTTP server on localhost (the proxy talks to it over
//! reqwest, exactly as it would to a provider), the storage is an in-memory
//! double, and the event publisher records the audit events the proxy emits —
//! so the whole request path runs without NATS or PostgreSQL.

mod common;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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
/// routing path (models → channel_models → channels), but keeps the circuit
/// breaker the 429 handler populates.
#[derive(Default)]
struct EmptyRegistry {
    broken: Mutex<HashMap<(String, String), Instant>>,
}

#[async_trait::async_trait]
impl ChannelRegistry for EmptyRegistry {
    async fn resolve_by_model(&self, _model: &str) -> Vec<ResolvedChannel> {
        Vec::new()
    }
    async fn resolve(&self, _channel_id: &str) -> Option<ResolvedChannel> {
        None
    }
    async fn reload(&self) {}
    fn disable_channel_model(&self, channel_id: &str, model_name: &str, until: Instant) {
        self.broken
            .lock()
            .unwrap()
            .insert((channel_id.to_string(), model_name.to_lowercase()), until);
    }
    fn is_circuit_broken(&self, channel_id: &str, model_name: &str) -> bool {
        let key = (channel_id.to_string(), model_name.to_lowercase());
        match self.broken.lock().unwrap().get(&key) {
            Some(until) => Instant::now() < *until,
            None => false,
        }
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

/// What the fake upstream answers with.
#[derive(Clone)]
struct FakeResponse {
    status: u16,
    body: String,
    content_type: String,
    /// Delay before answering — used to hang the upstream on purpose.
    delay: Duration,
}

impl FakeResponse {
    fn new(status: u16, body: &str) -> Self {
        FakeResponse {
            status,
            body: body.to_string(),
            content_type: "application/json".to_string(),
            delay: Duration::ZERO,
        }
    }

    fn slow(status: u16, body: &str, delay_ms: u64) -> Self {
        FakeResponse {
            status,
            body: body.to_string(),
            content_type: "application/json".to_string(),
            delay: Duration::from_millis(delay_ms),
        }
    }

    /// A server-sent-events response.
    fn sse(body: &str) -> Self {
        FakeResponse {
            status: 200,
            body: body.to_string(),
            content_type: "text/event-stream".to_string(),
            delay: Duration::ZERO,
        }
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
    async fn start(responses: HashMap<String, FakeResponse>) -> Self {
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

    /// Models this upstream was asked for, in order.
    fn hits(&self) -> Vec<String> {
        self.hits.lock().unwrap().clone()
    }
}

async fn upstream_handler(
    State((hits, responses)): State<(Arc<Mutex<Vec<String>>>, Arc<HashMap<String, FakeResponse>>)>,
    body: Bytes,
) -> Response {
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
    let model = parsed
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("")
        .to_string();
    hits.lock().unwrap().push(model.clone());

    let response = responses.get(&model).cloned().unwrap_or(FakeResponse {
        status: 200,
        body: format!("{{\"served_by\":\"{}\"}}", model),
        content_type: "application/json".to_string(),
        delay: Duration::ZERO,
    });

    if !response.delay.is_zero() {
        tokio::time::sleep(response.delay).await;
    }

    Response::builder()
        .status(response.status)
        .header("content-type", response.content_type.clone())
        .body(axum::body::Body::from(response.body))
        .unwrap()
}

// ── Fixture ────────────────────────────────────────────────────────────────

const RAW_KEY: &str = "sk-test-key";

/// Channel ids are UUIDs in production — the proxy parses them into a `Uuid`,
/// and the circuit breaker is keyed by that parsed id.
const CH1: &str = "11111111-1111-1111-1111-111111111111";
const CH2: &str = "22222222-2222-2222-2222-222222222222";

/// How the fixture gateway is wired up.
#[derive(Default)]
struct FixtureOptions {
    /// What channel-1's upstream answers per model.
    responses_c1: HashMap<String, FakeResponse>,
    /// What channel-2's upstream answers per model.
    responses_c2: HashMap<String, FakeResponse>,
    /// (channel, model) pairs: which channels serve which model.
    bindings: Vec<(&'static str, &'static str)>,
    /// Bind a fallback config to the key (glm-5.1 → minimax-m3.1-flash).
    with_fallback: bool,
    /// Models present in the gateway's registry.
    registered_models: Vec<String>,
    /// Channel that points at an unreachable endpoint (connection refused).
    dead_channel: Option<&'static str>,
    /// `[upstream] timeout_secs`.
    upstream_timeout_secs: u64,
}

impl FixtureOptions {
    fn new() -> Self {
        FixtureOptions {
            responses_c1: HashMap::new(),
            responses_c2: HashMap::new(),
            bindings: vec![
                (CH1, "glm-5.1"),
                (CH2, "glm-5.1"),
                (CH1, "minimax-m3.1-flash"),
            ],
            with_fallback: true,
            registered_models: vec!["glm-5.1".to_string(), "minimax-m3.1-flash".to_string()],
            dead_channel: None,
            upstream_timeout_secs: 30,
        }
    }
}

struct Fixture {
    state: Arc<AppState>,
    publisher: Arc<RecordingPublisher>,
    /// The upstream behind channel-1.
    upstream_c1: FakeUpstream,
    /// The upstream behind channel-2.
    upstream_c2: FakeUpstream,
}

/// Builds a gateway with one provider, two channels for `glm-5.1` and one for
/// `minimax-m3.1-flash`, and (optionally) a fallback config binding the two
/// models together.
async fn fixture(options: FixtureOptions) -> Fixture {
    let upstream_c1 = FakeUpstream::start(options.responses_c1.clone()).await;
    let upstream_c2 = FakeUpstream::start(options.responses_c2.clone()).await;

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
        model_fallback_id: if options.with_fallback {
            Some("fb-1".to_string())
        } else {
            None
        },
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    });

    for name in &options.registered_models {
        storage.models.lock().unwrap().push(Model {
            id: format!("m-{}", name),
            name: name.clone(),
            model_type: None,
            pricing_policy_id: None,
            created_at: chrono::Utc::now(),
        });
    }

    for (id, name, base_url) in [
        ("p-1", "fake-1", upstream_c1.base_url.clone()),
        ("p-2", "fake-2", upstream_c2.base_url.clone()),
    ] {
        storage.providers.lock().unwrap().push(Provider {
            id: id.to_string(),
            name: name.to_string(),
            slug: name.to_string(),
            endpoints: Some(serde_json::json!({ "openai": base_url }).to_string()),
            proxy_url: None,
            enabled: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
    }
    // A provider whose endpoint refuses connections.
    storage.providers.lock().unwrap().push(Provider {
        id: "p-dead".to_string(),
        name: "dead".to_string(),
        slug: "dead".to_string(),
        endpoints: Some(serde_json::json!({ "openai": "http://127.0.0.1:1" }).to_string()),
        proxy_url: None,
        enabled: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    });

    for (id, name, priority) in [(CH1, "channel-1", 1), (CH2, "channel-2", 2)] {
        let provider_id = if options.dead_channel == Some(id) {
            "p-dead"
        } else if id == CH1 {
            "p-1"
        } else {
            "p-2"
        };
        storage.channels.lock().unwrap().push(Channel {
            id: id.to_string(),
            provider_id: provider_id.to_string(),
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

    for (i, (channel_id, model)) in options.bindings.iter().enumerate() {
        if !options
            .registered_models
            .iter()
            .any(|m| m == model)
        {
            continue;
        }
        storage.channel_models.lock().unwrap().push(ChannelModel {
            id: format!("cm-{}", i),
            channel_id: channel_id.to_string(),
            model_id: format!("m-{}", model),
            upstream_model_name: None,
            priority_override: None,
            pricing_policy_id: None,
            markup_ratio: 0,
            enabled: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
    }

    if options.with_fallback {
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
        registry: Arc::new(EmptyRegistry::default()),
        system_info: SystemInfo {
            server_bind_address: "127.0.0.1:0".to_string(),
            database_driver: "memory".to_string(),
            rate_limit_window_secs: 60,
            rate_limit_flush_interval_secs: 30,
            upstream_timeout_secs: options.upstream_timeout_secs,
            audit_retention_days: None,
        },
    });

    Fixture {
        state,
        publisher,
        upstream_c1,
        upstream_c2,
    }
}

async fn proxy_request(state: Arc<AppState>, model: &str) -> (u16, String) {
    let mut headers = HeaderMap::new();
    headers.insert(
        "authorization",
        format!("Bearer {}", RAW_KEY).parse().unwrap(),
    );
    let body = format!(
        "{{\"model\":\"{}\",\"messages\":[{{\"role\":\"user\",\"content\":\"hi\"}}]}}",
        model
    );

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

/// A streaming request (`"stream": true`).
async fn proxy_stream_request(state: Arc<AppState>, model: &str) -> (u16, String) {
    let mut headers = HeaderMap::new();
    headers.insert(
        "authorization",
        format!("Bearer {}", RAW_KEY).parse().unwrap(),
    );
    let body = format!(
        "{{\"model\":\"{}\",\"stream\":true,\"messages\":[{{\"role\":\"user\",\"content\":\"hi\"}}]}}",
        model
    );

    match proxy_with_protocol(axum::extract::State(state), headers, body).await {
        Ok(response) => {
            let status = response.status().as_u16();
            let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            (status, String::from_utf8_lossy(&bytes).to_string())
        }
        Err(err) => (err.status_code(), err.message()),
    }
}

fn audits(fixture: &Fixture) -> Vec<AuditEvent> {
    fixture.publisher.audits.lock().unwrap().clone()
}

// ── Model fallback ─────────────────────────────────────────────────────────

#[tokio::test]
async fn upstream_4xx_exhausts_the_models_channels_before_the_fallback_runs() {
    let fixture = fixture(FixtureOptions {
        responses_c1: HashMap::from([
            (
                "glm-5.1".to_string(),
                FakeResponse::new(404, "{\"error\":\"no such model\"}"),
            ),
            (
                "minimax-m3.1-flash".to_string(),
                FakeResponse::new(200, "{\"served_by\":\"minimax\"}"),
            ),
        ]),
        responses_c2: HashMap::from([(
            "glm-5.1".to_string(),
            FakeResponse::new(404, "{\"error\":\"no such model\"}"),
        )]),
        ..FixtureOptions::new()
    })
    .await;

    let (status, body) = proxy_request(fixture.state.clone(), "glm-5.1").await;

    // The fallback served the request.
    assert_eq!(status, 200);
    assert!(body.contains("minimax"), "body: {}", body);

    // Both channels of glm-5.1 were contacted, then the fallback model.
    assert_eq!(fixture.upstream_c1.hits(), vec!["glm-5.1", "minimax-m3.1-flash"]);
    assert_eq!(fixture.upstream_c2.hits(), vec!["glm-5.1"]);

    // Exactly one audit event, carrying the whole chain — and the chain shows
    // the order the channels were tried in.
    let audits = audits(&fixture);
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
    assert_eq!((routes[0].model.as_str(), routes[0].channel_id.as_str()), ("glm-5.1", CH1));
    assert_eq!(routes[0].status_code, 404);
    assert_eq!((routes[1].model.as_str(), routes[1].channel_id.as_str()), ("glm-5.1", CH2));
    assert_eq!(routes[1].status_code, 404);
    assert_eq!((routes[2].model.as_str(), routes[2].channel_id.as_str()), ("minimax-m3.1-flash", CH1));
    assert_eq!(routes[2].status_code, 200);
}

#[tokio::test]
async fn without_a_fallback_the_upstream_error_is_returned_verbatim() {
    let fixture = fixture(FixtureOptions {
        responses_c1: HashMap::from([(
            "glm-5.1".to_string(),
            FakeResponse::new(404, "{\"error\":\"no such model\"}"),
        )]),
        responses_c2: HashMap::from([(
            "glm-5.1".to_string(),
            FakeResponse::new(404, "{\"error\":\"no such model\"}"),
        )]),
        with_fallback: false,
        ..FixtureOptions::new()
    })
    .await;

    let (status, body) = proxy_request(fixture.state.clone(), "glm-5.1").await;

    assert_eq!(status, 404);
    assert!(body.contains("no such model"), "body: {}", body);

    // Both channels were tried, and no fallback was.
    assert_eq!(fixture.upstream_c1.hits(), vec!["glm-5.1"]);
    assert_eq!(fixture.upstream_c2.hits(), vec!["glm-5.1"]);

    let audits = audits(&fixture);
    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0].status_code, 404);
    let routes = audits[0].routes.as_ref().unwrap();
    assert_eq!(routes.len(), 2);
    assert!(routes.iter().all(|r| r.model == "glm-5.1"));
}

#[tokio::test]
async fn a_failing_fallback_still_returns_the_original_upstream_error() {
    let fixture = fixture(FixtureOptions {
        responses_c1: HashMap::from([
            (
                "glm-5.1".to_string(),
                FakeResponse::new(503, "{\"error\":\"unavailable\"}"),
            ),
            (
                "minimax-m3.1-flash".to_string(),
                FakeResponse::new(500, "{\"error\":\"also down\"}"),
            ),
        ]),
        responses_c2: HashMap::from([(
            "glm-5.1".to_string(),
            FakeResponse::new(503, "{\"error\":\"unavailable\"}"),
        )]),
        ..FixtureOptions::new()
    })
    .await;

    let (status, body) = proxy_request(fixture.state.clone(), "glm-5.1").await;

    // A pure 5xx chain has no upstream error to hand back — the client gets
    // the gateway's own "all channels failed" error (the 4xx path is the one
    // that passes the upstream body through).
    assert_eq!(status, 502);
    assert!(body.contains("All channels failed"), "body: {}", body);

    // Both channels of glm-5.1, then the fallback model.
    assert_eq!(fixture.upstream_c1.hits(), vec!["glm-5.1", "minimax-m3.1-flash"]);
    assert_eq!(fixture.upstream_c2.hits(), vec!["glm-5.1"]);

    let audits = audits(&fixture);
    assert_eq!(audits.len(), 1, "the chain must not produce a row per attempt");
    assert_eq!(audits[0].status_code, 503);
    let routes = audits[0].routes.as_ref().unwrap();
    assert_eq!(routes.len(), 3);
}

#[tokio::test]
async fn a_fallback_candidate_that_errors_does_not_end_the_chain() {
    let fixture = fixture(FixtureOptions {
        responses_c1: HashMap::from([
            (
                "glm-5.1".to_string(),
                FakeResponse::new(404, "{\"error\":\"no such model\"}"),
            ),
            (
                "minimax-m3.1-flash".to_string(),
                FakeResponse::new(404, "{\"error\":\"not here either\"}"),
            ),
        ]),
        responses_c2: HashMap::from([(
            "glm-5.1".to_string(),
            FakeResponse::new(404, "{\"error\":\"no such model\"}"),
        )]),
        ..FixtureOptions::new()
    })
    .await;

    let (status, _body) = proxy_request(fixture.state.clone(), "glm-5.1").await;

    assert_eq!(status, 404);
    // The failing fallback candidate was tried, and the chain continued.
    assert_eq!(fixture.upstream_c1.hits(), vec!["glm-5.1", "minimax-m3.1-flash"]);
    assert_eq!(fixture.upstream_c2.hits(), vec!["glm-5.1"]);

    let audits = audits(&fixture);
    assert_eq!(audits.len(), 1);
    let routes = audits[0].routes.as_ref().unwrap();
    assert_eq!(routes.len(), 3);
    assert_eq!(routes[2].model, "minimax-m3.1-flash");
    assert_eq!(routes[2].status_code, 404);
}

#[tokio::test]
async fn a_model_missing_from_the_registry_is_served_by_the_fallback() {
    // The requested model is not registered at all — the fallback chain is the
    // only way the request can be served.
    let fixture = fixture(FixtureOptions {
        responses_c1: HashMap::from([(
            "minimax-m3.1-flash".to_string(),
            FakeResponse::new(200, "{\"served_by\":\"minimax\"}"),
        )]),
        registered_models: vec!["minimax-m3.1-flash".to_string()],
        ..FixtureOptions::new()
    })
    .await;

    let (status, body) = proxy_request(fixture.state.clone(), "glm-5.1").await;

    assert_eq!(status, 200);
    assert!(body.contains("minimax"), "body: {}", body);
    assert_eq!(fixture.upstream_c1.hits(), vec!["minimax-m3.1-flash"]);
    assert!(fixture.upstream_c2.hits().is_empty());

    let audits = audits(&fixture);
    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0].original_model.as_deref(), Some("glm-5.1"));
    assert_eq!(audits[0].model_name, "minimax-m3.1-flash");
}

#[tokio::test]
async fn an_unknown_model_never_reaches_an_upstream() {
    let fixture = fixture(FixtureOptions {
        responses_c1: HashMap::from([(
            "minimax-m3.1-flash".to_string(),
            FakeResponse::new(200, "{\"served_by\":\"minimax\"}"),
        )]),
        ..FixtureOptions::new()
    })
    .await;

    let (status, body) = proxy_request(fixture.state.clone(), "not-a-model").await;

    // The model is not in the registry and no fallback group contains it →
    // the client gets a 404 and no upstream was contacted.
    assert_eq!(status, 404);
    assert!(body.contains("not found"), "body: {}", body);
    assert!(fixture.upstream_c1.hits().is_empty());
    assert!(fixture.upstream_c2.hits().is_empty());

    // The routing failure is audited now (it used to vanish entirely).
    let audits = audits(&fixture);
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

// ── Failover ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn upstream_5xx_fails_over_to_the_next_channel() {
    let fixture = fixture(FixtureOptions {
        responses_c1: HashMap::from([(
            "glm-5.1".to_string(),
            FakeResponse::new(500, "{\"error\":\"boom\"}"),
        )]),
        responses_c2: HashMap::from([(
            "glm-5.1".to_string(),
            FakeResponse::new(200, "{\"served_by\":\"glm-via-c2\"}"),
        )]),
        ..FixtureOptions::new()
    })
    .await;

    let (status, body) = proxy_request(fixture.state.clone(), "glm-5.1").await;

    // The second channel of the same model served it — the fallback was not
    // needed.
    assert_eq!(status, 200);
    assert!(body.contains("glm-via-c2"), "body: {}", body);
    assert_eq!(fixture.upstream_c1.hits(), vec!["glm-5.1"]);
    assert_eq!(fixture.upstream_c2.hits(), vec!["glm-5.1"]);

    let audits = audits(&fixture);
    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0].status_code, 200);
    let routes = audits[0].routes.as_ref().unwrap();
    assert_eq!(routes.len(), 2);
    assert_eq!(routes[0].status_code, 500);
    assert_eq!(routes[0].channel_id, CH1);
    assert_eq!(routes[1].status_code, 200);
    assert_eq!(routes[1].channel_id, CH2);
}

#[tokio::test]
async fn upstream_429_circuit_breaks_the_channel_for_later_requests() {
    let fixture = fixture(FixtureOptions {
        responses_c1: HashMap::from([(
            "glm-5.1".to_string(),
            FakeResponse::new(429, "{\"error\":\"slow down\"}"),
        )]),
        responses_c2: HashMap::from([(
            "glm-5.1".to_string(),
            FakeResponse::new(200, "{\"served_by\":\"glm-via-c2\"}"),
        )]),
        ..FixtureOptions::new()
    })
    .await;

    let (status, _body) = proxy_request(fixture.state.clone(), "glm-5.1").await;
    assert_eq!(status, 200);
    // The rate-limited channel was tried, then the next one served the request.
    assert_eq!(fixture.upstream_c1.hits(), vec!["glm-5.1"]);
    assert_eq!(fixture.upstream_c2.hits(), vec!["glm-5.1"]);

    // The (channel, model) pair is now circuit-broken: the next request skips
    // the rate-limited channel entirely.
    let (status, _body) = proxy_request(fixture.state.clone(), "glm-5.1").await;
    assert_eq!(status, 200);
    assert_eq!(
        fixture.upstream_c1.hits(),
        vec!["glm-5.1"],
        "the circuit-broken channel must be skipped"
    );
    assert_eq!(fixture.upstream_c2.hits(), vec!["glm-5.1", "glm-5.1"]);
}

#[tokio::test]
async fn a_connection_error_fails_over_to_the_next_channel() {
    let fixture = fixture(FixtureOptions {
        responses_c2: HashMap::from([(
            "glm-5.1".to_string(),
            FakeResponse::new(200, "{\"served_by\":\"glm-via-c2\"}"),
        )]),
        dead_channel: Some(CH1),
        ..FixtureOptions::new()
    })
    .await;

    let (status, body) = proxy_request(fixture.state.clone(), "glm-5.1").await;

    assert_eq!(status, 200);
    assert!(body.contains("glm-via-c2"), "body: {}", body);
    // The dead channel never reached its upstream; the live one served it.
    assert!(fixture.upstream_c1.hits().is_empty());
    assert_eq!(fixture.upstream_c2.hits(), vec!["glm-5.1"]);

    let audits = audits(&fixture);
    assert_eq!(audits.len(), 1);
    let routes = audits[0].routes.as_ref().unwrap();
    assert_eq!(routes.len(), 2);
    assert_eq!(routes[0].status_code, 0, "a connection error has no status");
    assert!(routes[0]
        .error_message
        .as_ref()
        .unwrap()
        .contains("Connection error"));
}

#[tokio::test]
async fn a_hanging_upstream_times_out_and_fails_over() {
    let fixture = fixture(FixtureOptions {
        responses_c1: HashMap::from([(
            "glm-5.1".to_string(),
            FakeResponse::slow(200, "{\"served_by\":\"slow\"}", 3_000),
        )]),
        responses_c2: HashMap::from([
            (
                "glm-5.1".to_string(),
                FakeResponse::slow(200, "{\"served_by\":\"slow\"}", 3_000),
            ),
            (
                "minimax-m3.1-flash".to_string(),
                FakeResponse::new(200, "{\"served_by\":\"minimax\"}"),
            ),
        ]),
        // The fallback model is served by the second channel.
        bindings: vec![
            (CH1, "glm-5.1"),
            (CH2, "glm-5.1"),
            (CH2, "minimax-m3.1-flash"),
        ],
        upstream_timeout_secs: 1,
        ..FixtureOptions::new()
    })
    .await;

    let (status, body) = proxy_request(fixture.state.clone(), "glm-5.1").await;

    // Both glm-5.1 channels hung and timed out; the fallback served it.
    assert_eq!(status, 200);
    assert!(body.contains("minimax"), "body: {}", body);
    assert_eq!(fixture.upstream_c1.hits(), vec!["glm-5.1"]);
    assert_eq!(
        fixture.upstream_c2.hits(),
        vec!["glm-5.1", "minimax-m3.1-flash"]
    );

    let audits = audits(&fixture);
    assert_eq!(audits.len(), 1);
    let routes = audits[0].routes.as_ref().unwrap();
    assert_eq!(routes.len(), 3);
    assert_eq!(routes[0].status_code, 0);
    let error = routes[0].error_message.clone().unwrap().to_lowercase();
    assert!(
        error.contains("timed out") || error.contains("timeout"),
        "the audit must show that the upstream timed out, not just that the \
         request failed: {}",
        error
    );
}

#[tokio::test]
async fn a_successful_request_records_one_audit_row() {
    let fixture = fixture(FixtureOptions {
        responses_c1: HashMap::from([(
            "glm-5.1".to_string(),
            FakeResponse::new(200, "{\"served_by\":\"glm\"}"),
        )]),
        ..FixtureOptions::new()
    })
    .await;

    let (status, body) = proxy_request(fixture.state.clone(), "glm-5.1").await;

    assert_eq!(status, 200);
    assert!(body.contains("glm"), "body: {}", body);
    assert_eq!(fixture.upstream_c1.hits(), vec!["glm-5.1"]);

    let audits = audits(&fixture);
    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0].status_code, 200);
    assert_eq!(audits[0].model_name, "glm-5.1");
    assert_eq!(audits[0].original_model.as_deref(), Some("glm-5.1"));
    assert_eq!(audits[0].model_override_reason, None);
    let routes = audits[0].routes.as_ref().unwrap();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].status_code, 200);
}

// ── Streaming (SSE) ────────────────────────────────────────────────────────

#[tokio::test]
async fn a_streaming_request_forwards_the_events_and_audits_them() {
    let fixture = fixture(FixtureOptions {
        responses_c1: HashMap::from([(
            "glm-5.1".to_string(),
            FakeResponse::sse(
                "data: {\"choices\":[{\"delta\":{\"content\":\"he\"}}]}\n\n\
                 data: {\"choices\":[{\"delta\":{\"content\":\"llo\"}}]}\n\n\
                 data: [DONE]\n\n",
            ),
        )]),
        ..FixtureOptions::new()
    })
    .await;

    let (status, body) = proxy_stream_request(fixture.state.clone(), "glm-5.1").await;

    assert_eq!(status, 200);
    assert!(body.contains("hello") || body.contains("he"), "body: {}", body);

    let audits = audits(&fixture);
    assert_eq!(audits.len(), 1);
    assert!(audits[0].stream, "the audit must be marked as a stream");
    assert_eq!(audits[0].status_code, 200);
    assert!(
        audits[0].response_body.contains("hello")
            || audits[0].response_body.contains("he"),
        "the audit must contain the streamed events: {}",
        audits[0].response_body
    );
}

#[tokio::test]
async fn an_sse_error_event_circuit_breaks_the_channel() {
    let fixture = fixture(FixtureOptions {
        responses_c1: HashMap::from([(
            "glm-5.1".to_string(),
            FakeResponse::sse(
                "event: error\ndata: {\"error\":{\"message\":\"upstream exploded\"}}\n\n",
            ),
        )]),
        responses_c2: HashMap::from([(
            "glm-5.1".to_string(),
            FakeResponse::sse("data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\ndata: [DONE]\n\n"),
        )]),
        ..FixtureOptions::new()
    })
    .await;

    // The stream is already committed to the client, so the error event is
    // forwarded rather than turned into another model.
    let (status, body) = proxy_stream_request(fixture.state.clone(), "glm-5.1").await;
    assert_eq!(status, 200);
    assert!(body.contains("upstream exploded"), "body: {}", body);
    assert_eq!(fixture.upstream_c1.hits(), vec!["glm-5.1"]);

    // The (channel, model) pair is circuit-broken now: the next request skips
    // the channel that produced the error event.
    let (status, body) = proxy_stream_request(fixture.state.clone(), "glm-5.1").await;
    assert_eq!(status, 200);
    assert!(body.contains("ok"), "body: {}", body);
    assert_eq!(
        fixture.upstream_c1.hits(),
        vec!["glm-5.1"],
        "the circuit-broken channel must be skipped"
    );
    assert_eq!(fixture.upstream_c2.hits(), vec!["glm-5.1"]);
}

#[tokio::test]
async fn a_streaming_request_fails_over_to_the_next_channel_on_5xx() {
    let fixture = fixture(FixtureOptions {
        responses_c1: HashMap::from([(
            "glm-5.1".to_string(),
            FakeResponse::new(500, "{\"error\":\"boom\"}"),
        )]),
        responses_c2: HashMap::from([(
            "glm-5.1".to_string(),
            FakeResponse::sse("data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\ndata: [DONE]\n\n"),
        )]),
        ..FixtureOptions::new()
    })
    .await;

    let (status, body) = proxy_stream_request(fixture.state.clone(), "glm-5.1").await;

    assert_eq!(status, 200);
    assert!(body.contains("ok"), "body: {}", body);
    assert_eq!(fixture.upstream_c1.hits(), vec!["glm-5.1"]);
    assert_eq!(fixture.upstream_c2.hits(), vec!["glm-5.1"]);

    let audits = audits(&fixture);
    assert_eq!(audits.len(), 1);
    assert!(audits[0].stream);
    let routes = audits[0].routes.as_ref().unwrap();
    assert_eq!(routes.len(), 2);
    assert_eq!(routes[0].status_code, 500);
    assert_eq!(routes[1].status_code, 200);
}

#[tokio::test]
async fn a_streaming_request_falls_back_when_every_channel_fails() {
    let fixture = fixture(FixtureOptions {
        responses_c1: HashMap::from([
            (
                "glm-5.1".to_string(),
                FakeResponse::new(404, "{\"error\":\"no such model\"}"),
            ),
            (
                "minimax-m3.1-flash".to_string(),
                FakeResponse::sse("data: {\"choices\":[{\"delta\":{\"content\":\"from-minimax\"}}]}\n\ndata: [DONE]\n\n"),
            ),
        ]),
        responses_c2: HashMap::from([(
            "glm-5.1".to_string(),
            FakeResponse::new(404, "{\"error\":\"no such model\"}"),
        )]),
        ..FixtureOptions::new()
    })
    .await;

    let (status, body) = proxy_stream_request(fixture.state.clone(), "glm-5.1").await;

    // The fallback served the stream.
    assert_eq!(status, 200);
    assert!(body.contains("from-minimax"), "body: {}", body);
    assert_eq!(
        fixture.upstream_c1.hits(),
        vec!["glm-5.1", "minimax-m3.1-flash"]
    );
    assert_eq!(fixture.upstream_c2.hits(), vec!["glm-5.1"]);

    let audits = audits(&fixture);
    assert_eq!(audits.len(), 1, "the chain must not produce a row per attempt");
    assert!(audits[0].stream);
    assert_eq!(audits[0].model_name, "minimax-m3.1-flash");
    assert_eq!(audits[0].original_model.as_deref(), Some("glm-5.1"));
    assert_eq!(
        audits[0].model_override_reason.as_deref(),
        Some("model_fallback")
    );
    let routes = audits[0].routes.as_ref().unwrap();
    assert_eq!(routes.len(), 3);
}
