use axum::middleware;
use axum::routing::{get, post};
use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use llm_gateway_api::{self as api, AppState, InMemoryChannelRegistry};
use llm_gateway_audit::AuditLogger;
use llm_gateway_ratelimit::RateLimiter;
use llm_gateway_storage::{AppConfig, Storage};
use llm_gateway_storage::sqlite::SqliteStorage;
use rust_embed::Embed;
use sha2::Digest;
use std::sync::Arc;

#[derive(Embed)]
#[folder = "../../web/dist"]
struct Frontend;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Init tracing
    tracing_subscriber::fmt::init();

    // Load config
    let config_str = std::fs::read_to_string("config.toml")?;
    let config: AppConfig = toml::from_str(&config_str)?;

    // Init storage
    let db_path = config.database.url.as_deref().unwrap_or("./data/gateway.db");
    let storage = SqliteStorage::new(db_path).await?;
    storage.run_migrations().await?;
    storage.seed_data().await?;
    let storage: Arc<dyn Storage> = Arc::new(storage);

    // Init channel registry with in-memory cache
    let refresh_interval = std::time::Duration::from_secs(30); // 默认 30 秒
    let registry: Arc<dyn llm_gateway_api::ChannelRegistry> = Arc::new(
        InMemoryChannelRegistry::new(
            storage.clone(),
            {
                use sha2::Sha256;
                let key = config.server.encryption_key.as_bytes();
                let mut hasher = Sha256::new();
                hasher.update(key);
                let result = hasher.finalize();
                let mut bytes = [0u8; 32];
                bytes.copy_from_slice(&result);
                bytes
            },
            refresh_interval,
        )
    );
    // Start background refresh loop
    let registry_for_loop = registry.clone();
    tokio::spawn(async move {
        registry_for_loop.start_refresh_loop().await;
    });

    // Init rate limiter
    let rate_limiter = Arc::new(RateLimiter::new(config.rate_limit.window_size_secs));

    // Init audit logger
    let audit_logger = Arc::new(AuditLogger::new(storage.clone()));

    // Create MPSC channel for async audit logging
    let (audit_tx, audit_rx) = tokio::sync::mpsc::channel::<llm_gateway_api::AuditTask>(100);
    // Spawn background audit worker
    tokio::spawn(llm_gateway_api::workers::start_audit_worker(storage.clone(), audit_rx));

    // App state
    let state = Arc::new(AppState {
        storage,
        rate_limiter,
        audit_logger,
        jwt_secret: config.auth.jwt_secret.clone(),
        encryption_key: {
            use sha2::Sha256;
            let key = config.server.encryption_key.as_bytes();
            // Derive exactly 32 bytes using SHA256
            let mut hasher = Sha256::new();
            hasher.update(key);
            let result = hasher.finalize();
            let mut bytes = [0u8; 32];
            bytes.copy_from_slice(&result);
            bytes
        },
        audit_tx,
        registry,
    });

    // Build router
    let app = axum::Router::new()
        // OpenAI compatible endpoints (now unified through proxy)
        .route("/v1/chat/completions", post(api::proxy::proxy_with_protocol))
        .route("/v1/models", get(api::models::list_models))
        .route("/v1/messages", post(api::proxy::messages))
        // Management API
        .merge(api::management::management_router())
        // Frontend static files (fallback for SPA)
        .fallback(get(serve_frontend))
        // State + middleware
        .with_state(state)
        .layer(middleware::from_fn(trace_middleware));

    // Start server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    tracing::info!("Starting LLM Gateway on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn serve_frontend(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // Try exact file first
    if let Some(content) = Frontend::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return Response::builder()
            .header(header::CONTENT_TYPE, mime.as_ref())
            .body(content.data.to_vec().into())
            .unwrap();
    }

    // SPA fallback: only for paths without a file extension (HTML5 routes)
    if !path.contains('.') {
        if let Some(content) = Frontend::get("index.html") {
            return Response::builder()
                .header(header::CONTENT_TYPE, "text/html")
                .body(content.data.to_vec().into())
                .unwrap();
        }
    }

    StatusCode::NOT_FOUND.into_response()
}

async fn trace_middleware(
    req: axum::extract::Request,
    next: middleware::Next,
) -> axum::response::Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    tracing::info!(method = %method, path = %path, "request");
    let resp = next.run(req).await;
    tracing::info!(method = %method, path = %path, status = resp.status().as_u16(), "response");
    resp
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
