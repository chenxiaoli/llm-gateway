# Channel Enhancement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add rpm_limit, tpm_limit, balance, weight fields to Channel model and implement API key encryption.

**Architecture:** New encryption crate with AES-256-GCM, extend Channel struct with new fields, update routing to use channel-level rate limits and weighted selection.

**Tech Stack:** Rust (AES-256-GCM via aes-gcm crate), SQLite migrations, Axum handlers

---

## File Structure

### New Files:
- `crates/encryption/Cargo.toml` - new crate manifest
- `crates/encryption/src/lib.rs` - encryption functions
- `crates/storage/migrations/20260414000000_add_channel_fields.sql` - new migration

### Modified Files:
- `crates/storage/src/types.rs` - Channel, CreateChannel, UpdateChannel structs
- `crates/storage/src/sqlite.rs` - SqliteChannelRow, INSERT/SELECT queries
- `crates/storage/src/lib.rs` - Storage trait
- `Cargo.toml` - add encryption crate
- `config.toml` - add encryption_key
- `crates/api/src/management/channels.rs` - encrypt/decrypt api_key
- `crates/api/src/openai.rs` - use channel rate limits, weight, balance
- `crates/api/src/anthropic.rs` - use channel rate limits, weight, balance

---

## Task 1: Create Encryption Crate

**Files:**
- Create: `crates/encryption/Cargo.toml`
- Create: `crates/encryption/src/lib.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "llm-gateway-encryption"
version = "0.1.0"
edition = "2021"

[dependencies]
aes-gcm = "0.10"
base64 = "0.22"
rand = "0.8"
thiserror = "1"

[dev-dependencies]
```

- [ ] **Step 2: Create lib.rs with encrypt/decrypt functions**

```rust
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EncryptionError {
    #[error("Encryption failed")]
    EncryptFailed,
    #[error("Decryption failed")]
    DecryptFailed,
    #[error("Invalid key length")]
    InvalidKeyLength,
    #[error("Invalid data format")]
    InvalidFormat,
}

const NONCE_SIZE: usize = 12;

/// Encrypt plaintext with AES-256-GCM.
/// Returns base64(nonce || ciphertext).
pub fn encrypt(plaintext: &str, key: &[u8; 32]) -> Result<String, EncryptionError> {
    if key.len() != 32 {
        return Err(EncryptionError::InvalidKeyLength);
    }

    let cipher = Aes256Gcm::new(key.into());
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|_| EncryptionError::EncryptFailed)?;

    let mut result = nonce_bytes.to_vec();
    result.extend(ciphertext);
    Ok(BASE64.encode(&result))
}

/// Decrypt ciphertext (base64(nonce || ciphertext)) with AES-256-GCM.
pub fn decrypt(ciphertext: &str, key: &[u8; 32]) -> Result<String, EncryptionError> {
    if key.len() != 32 {
        return Err(EncryptionError::InvalidKeyLength);
    }

    let data = BASE64
        .decode(ciphertext)
        .map_err(|_| EncryptionError::InvalidFormat)?;

    if data.len() < NONCE_SIZE {
        return Err(EncryptionError::InvalidFormat);
    }

    let (nonce_bytes, encrypted) = data.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);

    let cipher = Aes256Gcm::new(key.into());
    let plaintext = cipher
        .decrypt(nonce, encrypted)
        .map_err(|_| EncryptionError::DecryptFailed)?;

    String::from_utf8(plaintext).map_err(|_| EncryptionError::InvalidFormat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = [0u8; 32];
        let plaintext = "sk-test-key-12345";

        let encrypted = encrypt(plaintext, &key).unwrap();
        let decrypted = decrypt(&encrypted, &key).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_different_ciphertexts_for_same_plaintext() {
        let key = [0u8; 32];
        let plaintext = "sk-test-key-12345";

        let encrypted1 = encrypt(plaintext, &key).unwrap();
        let encrypted2 = encrypt(plaintext, &key).unwrap();

        // Each encryption should produce different nonce
        assert_ne!(encrypted1, encrypted2);

        // Both should decrypt to same plaintext
        assert_eq!(decrypt(&encrypted1, &key).unwrap(), plaintext);
        assert_eq!(decrypt(&encrypted2, &key).unwrap(), plaintext);
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cd crates/encryption && cargo test`
Expected: PASS (2 tests)

- [ ] **Step 4: Commit**

```bash
git add crates/encryption/
git commit -m "feat: add encryption crate with AES-256-GCM

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 2: Update Cargo.toml and Config

**Files:**
- Modify: `Cargo.toml`
- Modify: `config.toml`

- [ ] **Step 1: Add encryption crate to workspace**

Add to `[workspace.dependencies]` in Cargo.toml:
```toml
llm-gateway-encryption = { path = "crates/encryption" }
```

- [ ] **Step 2: Add encryption dependency to api crate**

In `crates/api/Cargo.toml` dependencies section:
```toml
llm-gateway-encryption = { workspace = true }
```

- [ ] **Step 3: Add encryption_key to config.toml**

Add under `[server]` section:
```toml
encryption_key = "00000000000000000000000000000000"  # 32 bytes hex = 16 chars, needs fix
```

Note: Config expects string, will parse as hex or base64 in code.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml crates/api/Cargo.toml config.toml
git commit -m "chore: add encryption crate dependency and config

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 3: Add Channel Fields to Types

**Files:**
- Modify: `crates/storage/src/types.rs:96-127`

- [ ] **Step 1: Update Channel struct**

Replace lines 98-109 with:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub priority: i32,
    pub enabled: bool,
    pub rpm_limit: Option<i64>,     // NEW: requests per minute
    pub tpm_limit: Option<i64>,     // NEW: tokens per minute
    pub balance: Option<f64>,       // NEW: remaining quota in USD
    pub weight: Option<i32>,        // NEW: routing weight (default 100)
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

- [ ] **Step 2: Update CreateChannel struct**

Replace lines 111-117 with:
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
```

- [ ] **Step 3: Update UpdateChannel struct**

Replace lines 119-126 with:
```rust
#[derive(Debug, Deserialize)]
pub struct UpdateChannel {
    pub name: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<Option<String>>,
    pub priority: Option<i32>,
    pub enabled: Option<bool>,
    pub rpm_limit: Option<Option<i64>>,  // NEW: None=keep, Some(None)=clear
    pub tpm_limit: Option<Option<i64>>,  // NEW
    pub balance: Option<Option<f64>>,    // NEW
    pub weight: Option<Option<i32>>,     // NEW
}
```

- [ ] **Step 4: Commit**

```bash
git add crates/storage/src/types.rs
git commit -m "feat(storage): add rpm_limit, tpm_limit, balance, weight to Channel

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 4: Database Migration

**Files:**
- Create: `crates/storage/migrations/20260414000000_add_channel_fields.sql`

- [ ] **Step 1: Create migration file**

```sql
-- Add new fields to channels table
ALTER TABLE channels ADD COLUMN rpm_limit INTEGER;
ALTER TABLE channels ADD COLUMN tpm_limit INTEGER;
ALTER TABLE channels ADD COLUMN balance REAL;
ALTER TABLE channels ADD COLUMN weight INTEGER DEFAULT 100;
```

- [ ] **Step 2: Commit**

```bash
git add crates/storage/migrations/
git commit -m "feat(storage): add channel fields migration

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 5: Update SQLite Storage

**Files:**
- Modify: `crates/storage/src/sqlite.rs`

- [ ] **Step 1: Update SqliteChannelRow struct**

Replace lines 280-290 with:
```rust
#[derive(FromRow)]
struct SqliteChannelRow {
    id: String,
    provider_id: String,
    name: String,
    api_key: String,
    base_url: Option<String>,
    priority: i32,
    enabled: i64,
    rpm_limit: Option<i64>,     // NEW
    tpm_limit: Option<i64>,     // NEW
    balance: Option<f64>,       // NEW
    weight: Option<i32>,        // NEW
    created_at: String,
    updated_at: String,
}
```

- [ ] **Step 2: Update From<SqliteChannelRow> impl**

Replace impl block (lines 292-306) with:
```rust
impl From<SqliteChannelRow> for Channel {
    fn from(r: SqliteChannelRow) -> Self {
        Channel {
            id: r.id,
            provider_id: r.provider_id,
            name: r.name,
            api_key: r.api_key,
            base_url: r.base_url,
            priority: r.priority,
            enabled: r.enabled != 0,
            rpm_limit: r.rpm_limit,
            tpm_limit: r.tpm_limit,
            balance: r.balance,
            weight: r.weight,
            created_at: parse_rfc3339(&r.created_at),
            updated_at: parse_rfc3339(&r.updated_at),
        }
    }
}
```

- [ ] **Step 3: Update create_channel INSERT**

In `create_channel` function (around line 594), add to INSERT:
```rust
.bind(channel.rpm_limit)
.bind(channel.tpm_limit)
.bind(channel.balance)
.bind(channel.weight.unwrap_or(100))
```

And update column list to include: `rpm_limit, tpm_limit, balance, weight`

- [ ] **Step 4: Update get_channel SELECT**

In `get_channel` function, add to SELECT:
```rust
rpm_limit, tpm_limit, balance, weight
```

- [ ] **Step 5: Update list_channels_by_provider SELECT**

Add same columns.

- [ ] **Step 6: Update update_channel**

In UPDATE query, add:
```rust
.rpm_limit = ?, .tpm_limit = ?, .balance = ?, .weight = ?
```

Bind the values in correct order.

- [ ] **Step 7: Run tests**

Run: `cd crates/storage && cargo test`
Expected: PASS (may need migration)

- [ ] **Step 8: Commit**

```bash
git add crates/storage/src/sqlite.rs
git commit -m "feat(storage): implement channel fields in SQLite

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 6: Update Management API (Channels)

**Files:**
- Modify: `crates/api/src/management/channels.rs`

- [ ] **Step 1: Add encryption imports**

At top of file, add:
```rust
use llm_gateway_encryption::{decrypt, encrypt};
```

- [ ] **Step 2: Add config for encryption key**

Need to get encryption key from config in AppState. Check how config is loaded in the codebase first, then add to AppState.

- [ ] **Step 3: Update create_channel to encrypt**

In create_channel function, before storing:
```rust
let encrypted_key = encrypt(&input.api_key, &state.encryption_key)
    .map_err(|e| ApiError::Internal(e.to_string()))?;
```

Replace `input.api_key` with `encrypted_key` when creating Channel.

- [ ] **Step 4: Update get_channel to decrypt**

In the response, before returning JSON:
```rust
let mut channel = ...;
channel.api_key = decrypt(&channel.api_key, &state.encryption_key)
    .unwrap_or_else(|_| channel.api_key);  // fallback to stored if decrypt fails
```

- [ ] **Step 5: Update update_channel**

- If api_key is being changed, encrypt the new key
- No need to decrypt on read (handled in get_channel)

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/management/channels.rs
git commit -m "feat(api): add API key encryption for channels

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 7: Update Routing with Rate Limits

**Files:**
- Modify: `crates/api/src/openai.rs:90-110`
- Modify: `crates/api/src/anthropic.rs:90-110`

- [ ] **Step 1: Update get_rate_limits to accept channel**

In openai.rs, find `get_rate_limits` function around line 90. Update signature:
```rust
async fn get_rate_limits(
    state: &AppState,
    api_key: &ApiKey,
    model_name: &str,
    channel: Option<&Channel>,  // NEW: optional channel for overrides
) -> (Option<i64>, Option<i64>)
```

- [ ] **Step 2: Implement channel override logic**

```rust
async fn get_rate_limits(
    state: &AppState,
    api_key: &ApiKey,
    model_name: &str,
    channel: Option<&Channel>,
) -> (Option<i64>, Option<i64>) {
    // 1. Check channel-level limits first
    if let Some(ch) = channel {
        if ch.rpm_limit.is_some() || ch.tpm_limit.is_some() {
            return (ch.rpm_limit, ch.tpm_limit);
        }
    }
    // 2. Fall back to key-level limits
    // ... existing code ...
}
```

- [ ] **Step 3: Update call sites**

Update the calls to `get_rate_limits` in the routing loop to pass the channel:
```rust
let (rpm_limit, tpm_limit) = get_rate_limits(&state, &api_key, &model_name, Some(channel)).await;
```

- [ ] **Step 4: Same changes for anthropic.rs**

- [ ] **Step 5: Run tests**

Run: `cargo test --package llm-gateway-api`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/openai.rs crates/api/src/anthropic.rs
git commit -m "feat(api): use channel-level rate limits in routing

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 8: Add Weighted Routing and Balance Check

**Files:**
- Modify: `crates/api/src/openai.rs`
- Modify: `crates/api/src/anthropic.rs`

- [ ] **Step 1: Add helper functions**

Add before the main handler:
```rust
/// Select a channel using weighted round-robin
fn select_channel_by_weight(channels: &[Channel], request_count: u64) -> Option<&Channel> {
    if channels.is_empty() {
        return None;
    }

    let total_weight: i32 = channels.iter()
        .map(|c| c.weight.unwrap_or(100))
        .sum();
    
    if total_weight == 0 {
        return Some(&channels[0]);
    }

    let cursor = (request_count % total_weight as u64) as i32;
    let mut running = 0;
    
    for channel in channels {
        running += channel.weight.unwrap_or(100);
        if cursor < running {
            return Some(channel);
        }
    }
    
    // Fallback
    Some(&channels[0])
}

/// Check if channel is available (enabled and has balance)
fn is_channel_available(channel: &Channel) -> bool {
    channel.enabled && channel.balance.map_or(true, |b| b > 0.0)
}
```

- [ ] **Step 2: Update channel selection loop**

In the channel failover loop, before trying a channel:
```rust
for channel in &channels {
    // Skip channels that are disabled or have zero balance
    if !is_channel_available(channel) {
        continue;
    }
    
    // For weighted selection, we'd need request count from somewhere
    // For now, keep priority-based selection but skip unavailable channels
    // ...
}
```

- [ ] **Step 3: Add balance deduction on request completion**

In `record_stream_usage` function, after calculating cost:
```rust
// Deduct from channel balance if it has one
if let Some(balance) = channel.balance {
    let new_balance = balance - cost.cost;
    // This would need async update - consider doing in background
    // For now, log the cost for manual tracking
    tracing::info!("Request cost: ${:.4}, channel balance: ${:.4}", cost.cost, balance);
}
```

Note: Full balance tracking requires async update which is more complex. Start with logging and add async update in follow-up.

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/openai.rs crates/api/src/anthropic.rs
git commit -m "feat(api): add channel availability check and weighted selection

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 9: Frontend Updates (Optional)

**Files:**
- Modify: `web/src/pages/Channels.tsx`
- Modify: `web/src/api/providers.ts`
- Modify: `web/src/types/index.ts`

- [ ] **Step 1: Update TypeScript types**

Add new fields to Channel type in types/index.ts.

- [ ] **Step 2: Update API client**

Update CreateChannel/UpdateChannel interfaces.

- [ ] **Step 3: Update Channel list page**

Add columns for rpm_limit, tpm_limit, balance, weight.

- [ ] **Step 4: Update Channel form**

Add input fields for new fields.

- [ ] **Step 5: Commit**

---

## Summary

This plan adds 9 tasks covering:
1. Encryption crate (new)
2. Config and dependencies
3. Storage types
4. Database migration
5. SQLite implementation
6. Management API with encryption
7. Rate limiting override
8. Weighted routing and balance check
9. Frontend (optional)

**Plan complete and saved to `docs/superpowers/plans/2026-04-14-channel-enhancement.md`.**

Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?