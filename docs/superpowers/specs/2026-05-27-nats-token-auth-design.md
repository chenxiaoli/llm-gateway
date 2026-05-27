# NATS Token Authentication Design

## Problem

NATS connections have no authentication support. If the NATS server requires token auth, the gateway and workers cannot connect.

## Design

Add optional `token` field to NATS config. When present, use `async_nats::ConnectOptions::with_token()` for authentication.

### Config

```toml
[nats]
url = "nats://nats:4222"
token = "my-secret-token"  # optional, default: absent
```

### NatsConfig

```rust
pub struct NatsConfig {
    pub url: String,
    pub token: Option<String>,  // new
}
```

### NatsPublisher::new()

Signature changes to `new(url: &str, token: Option<String>)`.

```rust
let client = if let Some(token) = token {
    async_nats::connect_with_options(
        url,
        async_nats::ConnectOptions::new().token(token.into()),
    ).await?
} else {
    async_nats::connect(url).await?
};
```

### Call sites

All three (gateway, audit-worker, usage-worker) pass `nats_cfg.token.clone()`.

## Changes

| File | Change |
|---|---|
| `crates/storage/src/types.rs` | Add `token: Option<String>` to `NatsConfig` |
| `crates/nats-publisher/src/lib.rs` | Update `new()` signature and auth logic |
| `crates/gateway/src/main.rs` | Pass `nats_cfg.token.clone()` |
| `crates/audit-worker/src/main.rs` | Pass `nats_cfg.token.clone()` |
| `crates/usage-worker/src/main.rs` | Pass `nats_cfg.token.clone()` |
