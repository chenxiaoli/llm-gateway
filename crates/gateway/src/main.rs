use axum::middleware;
use axum::routing::{get, post};
use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use llm_gateway_api::{self as api, AppState, SystemInfo, InMemoryChannelRegistry, spawn_registry_refresh};
use llm_gateway_ratelimit::RateLimiter;
use llm_gateway_storage::{AppConfig, Storage};
use llm_gateway_storage::postgres::PostgresStorage;
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

    // Bootstrap: create data directory and default config.toml if missing
    bootstrap().await?;

    // Load config with environment variable expansion
    let config_str = std::fs::read_to_string("config.toml")?;
    let config_str = shellexpand::env(&config_str)?.to_string();
    let config: AppConfig = toml::from_str(&config_str)?;

    // Init storage
    let storage: Arc<dyn Storage> = {
        if config.database.driver.as_str() != "postgres" {
            return Err(format!("Unsupported database driver '{}'. Only 'postgres' is supported", config.database.driver).into());
        }
        let url = config.database.url.as_deref().ok_or("database.url is required")?;
        tracing::info!("Using PostgreSQL: {}", url.split('@').last().unwrap_or("***"));
        let db = PostgresStorage::new(url).await?;
        db.run_migrations().await?;
        db.seed_data().await?;
        Arc::new(db)
    };

    // Init NATS publisher (required)
    let nats_cfg = config.nats.as_ref().ok_or("[nats] section is required in config.toml")?;
    let nats_publisher: Arc<llm_gateway_nats_publisher::NatsPublisher> =
        match llm_gateway_nats_publisher::NatsPublisher::new(&nats_cfg.url, nats_cfg.token.clone(), nats_cfg.credentials_file.clone()).await {
            Ok(pub_) => {
                tracing::info!("Connected to NATS: {}", nats_cfg.url);
                Arc::new(pub_)
            }
            Err(e) => {
                tracing::error!("Failed to connect to NATS: {}", e);
                return Err(format!("NATS connection failed: {}", e).into());
            }
        };

    // Derive encryption key (32 bytes via SHA256)
    let encryption_key: [u8; 32] = {
        use sha2::Sha256;
        let key = config.server.encryption_key.as_bytes();
        let mut hasher = Sha256::new();
        hasher.update(key);
        let result = hasher.finalize();
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&result);
        bytes
    };

    // Init channel registry with in-memory cache
    let refresh_interval = std::time::Duration::from_secs(30); // default 30 seconds
    let registry_inner: Arc<InMemoryChannelRegistry> = Arc::new(
        InMemoryChannelRegistry::new(storage.clone(), encryption_key, refresh_interval)
    );
    // Start background refresh loop
    spawn_registry_refresh(registry_inner.clone());
    let registry: Arc<dyn llm_gateway_api::ChannelRegistry> = registry_inner;

    // Init rate limiter
    let rate_limiter = Arc::new(RateLimiter::new(config.rate_limit.window_size_secs));

    // App state
    let system_info = SystemInfo {
        server_bind_address: format!("{}:{}", config.server.host, config.server.port),
        database_driver: config.database.driver.clone(),
        rate_limit_window_secs: config.rate_limit.window_size_secs,
        rate_limit_flush_interval_secs: config.rate_limit.flush_interval_secs,
        upstream_timeout_secs: config.upstream.timeout_secs,
        audit_retention_days: config.audit.retention_days,
    };
    let state = Arc::new(AppState {
        storage,
        rate_limiter,
        jwt_secret: config.auth.jwt_secret.clone(),
        encryption_key,
        nats_publisher: Some(nats_publisher),
        registry,
        system_info,
    });

    // Spawn platform-admin impersonation janitor — reaps stale temp member
    // rows (created_by='system') left behind by platform_admin visits. The
    // 1-hour threshold is generous on purpose: the janitor is a safety net,
    // not the primary exit signal — a stale row is an inconvenience, not a
    // bug. Tick is 5 minutes.
    {
        let janitor_state = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
            interval.tick().await; // skip the immediate first tick
            loop {
                interval.tick().await;
                match llm_gateway_api::janitor::cleanup_stale_impersonations(
                    &janitor_state,
                    chrono::Duration::hours(1),
                )
                .await
                {
                    Ok(0) => {}
                    Ok(n) => tracing::info!(
                        rows_removed = n,
                        "janitor: reaped stale platform-admin temp member rows"
                    ),
                    Err(e) => tracing::warn!(
                        error = %e,
                        "janitor: cleanup_stale_impersonations failed"
                    ),
                }
            }
        });
    }

    // Build router
    let app = axum::Router::new()
        // OpenAI compatible endpoints (now unified through proxy)
        // Use /*path to capture the remaining path after /v1
        .route("/v1/chat/completions", post(api::proxy::proxy_with_protocol))
        .route("/v1/models", get(api::models::list_models))
        .route("/api/v1/user/models", get(api::user_models::list_user_models))
        .route("/v1/messages", post(api::proxy::messages))
        .route("/v1/responses", post(api::proxy::responses))
        // Management API
        .merge(api::management::management_router(state.clone()))
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

/// Bootstrap: create ./data/ directory and default config.toml if missing.
/// Safe to call repeatedly — skips creation if already present.
async fn bootstrap() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config_path = std::path::Path::new("config.toml");
    if config_path.exists() {
        return Ok(());
    }

    tracing::info!("config.toml not found, creating ./data/ and default config.toml");

    // Create data directory
    tokio::fs::create_dir_all("./data").await?;

    // Write default config.toml
    let default_config = r#"# LLM Gateway Configuration
# Generated automatically on first startup.

[server]
host = "0.0.0.0"
port = 8080
# IMPORTANT: Change this to a random 32-byte secret in production
encryption_key = "change-me-32-byte-secret-here!"

[auth]
# IMPORTANT: Change this to a random JWT secret in production
jwt_secret = "change-me-jwt-secret!"
allow_registration = true

[database]
driver = "postgres"
url = "postgresql://user:password@localhost/llm_gateway"

[nats]
url = "nats://localhost:4222"

[rate_limit]
flush_interval_secs = 30
window_size_secs = 60

[upstream]
timeout_secs = 30

[audit]
retention_days = 90
"#;
    tokio::fs::write(config_path, default_config).await?;
    tracing::info!("Created config.toml — please edit it with your secrets before restarting");
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
