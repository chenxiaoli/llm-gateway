# Channel Enhancement Design Spec

**Date:** 2026-04-14

## Overview

Enhance the Channel model with four new fields (rpm_limit, tpm_limit, balance, weight) and add API key encryption. Channel is the operational unit that describes "which credential, which route, what limits" — it's the most important table in the system.

## Background

A provider can have multiple channels:
- Different API keys for the same OpenAI account
- Primary and backup accounts
- Different Azure deployment endpoints

Each channel should independently manage:
- api_key (encrypted)
- base_url_override (needed for Azure)
- Rate limits (rpm/tpm)
- Balance (remaining quota)
- Weight (for routing strategy)

## Data Model

### Channel Struct Changes

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    pub api_key: String,           // stored encrypted
    pub base_url: Option<String>,
    pub priority: i32,
    pub enabled: bool,
    pub rpm_limit: Option<i64>,    // NEW: requests per minute (None = unlimited)
    pub tpm_limit: Option<i64>,    // NEW: tokens per minute (None = unlimited)
    pub balance: Option<f64>,      // NEW: remaining quota in USD
    pub weight: Option<i32>,       // NEW: routing weight (default 100)
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### CreateChannel/UpdateChannel

```rust
#[derive(Debug, Deserialize)]
pub struct CreateChannel {
    pub name: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub priority: Option<i32>,
    pub rpm_limit: Option<i64>,    // NEW
    pub tpm_limit: Option<i64>,    // NEW
    pub balance: Option<f64>,      // NEW
    pub weight: Option<i32>,       // NEW
}

#[derive(Debug, Deserialize)]
pub struct UpdateChannel {
    pub name: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<Option<String>>,
    pub priority: Option<i32>,
    pub enabled: Option<bool>,
    pub rpm_limit: Option<Option<i64>>,  // NEW: None = keep, Some(None) = clear
    pub tpm_limit: Option<Option<i64>>,  // NEW
    pub balance: Option<Option<f64>>,     // NEW
    pub weight: Option<Option<i32>>,     // NEW
}
```

## Database Schema

### Migration: add_channel_fields.sql

```sql
ALTER TABLE channels ADD COLUMN rpm_limit INTEGER;
ALTER TABLE channels ADD COLUMN tpm_limit INTEGER;
ALTER TABLE channels ADD COLUMN balance REAL;
ALTER TABLE channels ADD COLUMN weight INTEGER DEFAULT 100;
```

Note: Existing channels get weight=100, other fields=NULL (unlimited).

## API Key Encryption

### Configuration

In `config.toml`:

```toml
[server]
encryption_key = "your-32-byte-key-here"
```

### Encryption Service

New crate: `crates/encryption/src/lib.rs`

```rust
pub fn encrypt(plaintext: &str, key: &[u8; 32]) -> String {
    // AES-256-GCM
    // Returns: base64(nonce || ciphertext || tag)
}

pub fn decrypt(ciphertext: &str, key: &[u8; 32]) -> Result<String, Error> {
    // AES-256-GCM
}
```

### Integration Points

1. **create_channel**: Encrypt api_key before storing
2. **get_channel**: Decrypt api_key on read (before returning JSON)
3. **update_channel**: Re-encrypt if api_key changed
4. **openai.rs / anthropic.rs**: Decrypt when using for upstream request

## Rate Limiting

### Priority

```
channel.rpm_limit > key.rate_limit > unlimited
channel.tpm_limit > key.tpm_limit > unlimited
```

### Implementation

In routing logic (`openai.rs:95`, `anthropic.rs:95`):

```rust
async fn get_rate_limits(
    state: &AppState,
    api_key: &ApiKey,
    model_name: &str,
    channel: &Channel,
) -> (Option<i64>, Option<i64>) {
    // 1. Check channel-level limits first
    if channel.rpm_limit.is_some() || channel.tpm_limit.is_some() {
        return (channel.rpm_limit, channel.tpm_limit);
    }
    // 2. Fall back to key-level limits
    let (key_rpm, key_tpm) = get_key_rate_limits(api_key, model_name).await;
    (key_rpm, key_tpm)
}
```

## Routing with Weight

### Weighted Round-Robin

When selecting a channel from the enabled list:

```rust
fn select_channel_by_weight(channels: &[Channel], request_count: u64) -> &Channel {
    let total_weight: i32 = channels.iter()
        .map(|c| c.weight.unwrap_or(100))
        .sum();
    
    let cursor = (request_count % total_weight as u64) as i32;
    let mut running = 0;
    
    for channel in channels {
        running += channel.weight.unwrap_or(100);
        if cursor < running {
            return channel;
        }
    }
    
    // Fallback to first channel
    &channels[0]
}
```

### Balance Check

Before attempting request on a channel:

```rust
fn is_channel_available(channel: &Channel) -> bool {
    channel.enabled && 
        channel.balance.map_or(true, |b| b > 0.0)
}
```

If channel.balance exists and is <= 0, skip the channel in routing.

## Balance Tracking

### On Request Completion

1. Calculate request cost
2. If channel has balance, deduct cost
3. Update channel in storage

### API Endpoints

- `GET /channels/{id}` - returns current balance
- `PATCH /channels/{id}` - update balance via UpdateChannel
- `POST /management/channels/{id}/balance` - add/reduce balance (optional convenience endpoint)

## Frontend Changes

### Channel List Page

Add columns:
- RPM Limit (or "∞")
- TPM Limit (or "∞")
- Balance (currency format)
- Weight

### Channel Form

Add fields:
- RPM Limit (number input, empty = unlimited)
- TPM Limit (number input, empty = unlimited)
- Balance (number input, empty = no limit)
- Weight (number input, default 100)

## Testing

1. **Encryption tests**: encrypt/decrypt roundtrip
2. **Channel CRUD**: create, read, update with new fields
3. **Rate limiting**: verify channel limits override key limits
4. **Routing**: verify weighted selection and balance checks
5. **Migration**: existing channels work after migration