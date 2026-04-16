# System Settings & Audit Log Config Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add server_host and audit logging config (request/response body toggle) to system settings

**Architecture:** Settings stored in key-value table. AuditLogger reads settings per-request to decide what to log. Frontend fetches settings and displays server_host in Quick Start.

**Tech Stack:** Rust (Axum), React (TanStack Query), SQLite

---

## File Structure

### Modified Files:
- `crates/api/src/management/settings.rs` - Add server_host, audit_log_request, audit_log_response
- `crates/audit/src/lib.rs` - Read settings to determine what to log
- `crates/api/src/openai.rs` - Pass settings to audit logger
- `crates/api/src/anthropic.rs` - Pass settings to audit logger
- `web/src/api/settings.ts` - Add new settings types
- `web/src/hooks/useSettings.ts` - Add new settings to hook
- `web/src/pages/Settings.tsx` - Add audit log toggles
- `web/src/pages/Home.tsx` - Fetch and display server_host in Quick Start
- `web/src/types/index.ts` - Add SettingsResponse type

---

## Task 1: Update Backend Settings API

**Files:**
- Modify: `crates/api/src/management/settings.rs:1-41`

- [ ] **Step 1: Update SettingsResponse and UpdateSettingsRequest**

Replace `crates/api/src/management/settings.rs:11-19`:

```rust
#[derive(Deserialize)]
pub struct UpdateSettingsRequest {
    pub allow_registration: bool,
    pub server_host: Option<String>,
    pub audit_log_request: Option<bool>,
    pub audit_log_response: Option<bool>,
}

#[derive(serde::Serialize)]
pub struct SettingsResponse {
    pub allow_registration: bool,
    pub server_host: String,
    pub audit_log_request: bool,
    pub audit_log_response: bool,
}
```

- [ ] **Step 2: Update get_settings handler**

Replace `crates/api/src/management/settings.rs:21-29`:

```rust
pub async fn get_settings(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<SettingsResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;
    
    let allow_reg = state.storage.get_setting("allow_registration").await.map_err(|e| ApiError::Internal(e.to_string()))?;
    let server_host = state.storage.get_setting("server_host").await.map_err(|e| ApiError::Internal(e.to_string()))?;
    let audit_req = state.storage.get_setting("audit_log_request").await.map_err(|e| ApiError::Internal(e.to_string()))?;
    let audit_res = state.storage.get_setting("audit_log_response").await.map_err(|e| ApiError::Internal(e.to_string()))?;
    
    Ok(Json(SettingsResponse {
        allow_registration: allow_reg.map(|v| v == "true").unwrap_or(true),
        server_host: server_host.unwrap_or_default(),
        audit_log_request: audit_req.map(|v| v == "true").unwrap_or(true),
        audit_log_response: audit_res.map(|v| v == "true").unwrap_or(true),
    }))
}
```

- [ ] **Step 3: Update update_settings handler**

Replace `crates/api/src/management/settings.rs:31-40`:

```rust
pub async fn update_settings(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<UpdateSettingsRequest>,
) -> Result<Json<SettingsResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;
    
    if let Some(ar) = Some(input.allow_registration) {
        state.storage.set_setting("allow_registration", if ar { "true" } else { "false" })
            .await.map_err(|e| ApiError::Internal(e.to_string()))?;
    }
    if let Some(sh) = input.server_host {
        state.storage.set_setting("server_host", &sh)
            .await.map_err(|e| ApiError::Internal(e.to_string()))?;
    }
    if let Some(alr) = input.audit_log_request {
        state.storage.set_setting("audit_log_request", if alr { "true" } else { "false" })
            .await.map_err(|e| ApiError::Internal(e.to_string()))?;
    }
    if let Some(alp) = input.audit_log_response {
        state.storage.set_setting("audit_log_response", if alp { "true" } else { "false" })
            .await.map_err(|e| ApiError::Internal(e.to_string()))?;
    }
    
    // Return updated settings
    let allow_reg = state.storage.get_setting("allow_registration").await.map_err(|e| ApiError::Internal(e.to_string()))?;
    let server_host = state.storage.get_setting("server_host").await.map_err(|e| ApiError::Internal(e.to_string()))?;
    let audit_req = state.storage.get_setting("audit_log_request").await.map_err(|e| ApiError::Internal(e.to_string()))?;
    let audit_res = state.storage.get_setting("audit_log_response").await.map_err(|e| ApiError::Internal(e.to_string()))?;
    
    Ok(Json(SettingsResponse {
        allow_registration: allow_reg.map(|v| v == "true").unwrap_or(true),
        server_host: server_host.unwrap_or_default(),
        audit_log_request: audit_req.map(|v| v == "true").unwrap_or(true),
        audit_log_response: audit_res.map(|v| v == "true").unwrap_or(true),
    }))
}
```

- [ ] **Step 4: Run tests**

Run: `cargo check --package llm-gateway-api`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/management/settings.rs
git commit -m 'feat(api): add server_host and audit log config to settings

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>'
```

---

## Task 2: Update AuditLogger to Read Settings

**Files:**
- Modify: `crates/audit/src/lib.rs`

- [ ] **Step 1: Update AuditLogger struct**

Replace `crates/audit/src/lib.rs:1-11`:

```rust
use llm_gateway_storage::{AuditLog, Protocol, Storage};
use std::sync::Arc;

pub struct AuditLogger {
    storage: Arc<dyn Storage>,
}

impl AuditLogger {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
    }

    fn should_log_request(&self, settings: &SettingSnapshot) -> bool {
        settings.audit_log_request
    }

    fn should_log_response(&self, settings: &SettingSnapshot) -> bool {
        settings.audit_log_response
    }
}

pub struct SettingSnapshot {
    pub audit_log_request: bool,
    pub audit_log_response: bool,
}

impl AuditLogger {
    pub async fn get_settings(&self) -> SettingSnapshot {
        let audit_req = self.storage.get_setting("audit_log_request").await.ok()
            .flatten()
            .map(|v| v == "true")
            .unwrap_or(true);
        let audit_res = self.storage.get_setting("audit_log_response").await.ok()
            .flatten()
            .map(|v| v == "true")
            .unwrap_or(true);
        
        SettingSnapshot {
            audit_log_request: audit_req,
            audit_log_response: audit_res,
        }
    }
}
```

- [ ] **Step 2: Add settings parameter to log_request**

Replace `crates/api/src/management/settings.rs:31-43`:

```rust
pub async fn log_request(
    &self,
    key_id: &str,
    model_name: &str,
    provider_id: &str,
    protocol: Protocol,
    request_body: &str,
    response_body: &str,
    status_code: i32,
    latency_ms: i64,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Get settings to determine what to log
    let settings = self.get_settings().await;
    
    let final_request_body = if settings.audit_log_request {
        request_body.to_string()
    } else {
        "{}".to_string()
    };
    
    let final_response_body = if settings.audit_log_response {
        response_body.to_string()
    } else {
        "{}".to_string()
    };
    
    let log = AuditLog {
        id: uuid::Uuid::new_v4().to_string(),
        key_id: key_id.to_string(),
        model_name: model_name.to_string(),
        provider_id: provider_id.to_string(),
        channel_id: None,
        protocol,
        request_body: final_request_body,
        response_body: final_response_body,
        status_code,
        latency_ms,
        input_tokens,
        output_tokens,
        created_at: chrono::Utc::now(),
    };
    self.storage.insert_log(&log).await
}
```

- [ ] **Step 3: Run tests**

Run: `cargo check --package llm-gateway-audit`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/audit/src/lib.rs
git commit -m 'feat(audit): add settings-based request/response logging

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>'
```

---

## Task 3: Update Frontend Settings Types & Hook

**Files:**
- Modify: `web/src/types/index.ts:185-195`
- Modify: `web/src/hooks/useSettings.ts`

- [ ] **Step 1: Add SettingsResponse type**

Add to `web/src/types/index.ts`:

```typescript
export interface SettingsResponse {
  allow_registration: boolean;
  server_host: string;
  audit_log_request: boolean;
  audit_log_response: boolean;
}

export interface UpdateSettingsRequest {
  allow_registration?: boolean;
  server_host?: string;
  audit_log_request?: boolean;
  audit_log_response?: boolean;
}
```

- [ ] **Step 2: Update useSettings hook**

Read `web/src/hooks/useSettings.ts` and add the new fields to return type.

- [ ] **Step 3: Run tests**

Run: `npm run build` in web/
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add web/src/types/index.ts web/src/hooks/useSettings.ts
git commit -m 'feat(web): add server_host and audit log settings types

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>'
```

---

## Task 4: Update Settings Page UI

**Files:**
- Modify: `web/src/pages/Settings.tsx:35-48`

- [ ] **Step 1: Add audit log toggles**

In the settings card, add after the allow_registration toggle:

```tsx
<div className="flex items-center justify-between py-3 border-b border-base-200">
  <div>
    <span className="text-sm text-base-content/70">Log Request Body</span>
    <p className="text-xs text-base-content/40">Store request body in audit logs</p>
  </div>
  <Toggle checked={settings?.audit_log_request ?? true} onChange={(checked) => updateMutation.mutate({ audit_log_request: checked })} />
</div>

<div className="flex items-center justify-between py-3">
  <div>
    <span className="text-sm text-base-content/70">Log Response Body</span>
    <p className="text-xs text-base-content/40">Store response body in audit logs</p>
  </div>
  <Toggle checked={settings?.audit_log_response ?? true} onChange={(checked) => updateMutation.mutate({ audit_log_response: checked })} />
</div>
```

- [ ] **Step 2: Run tests**

Run: `npm run build` in web/
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add web/src/pages/Settings.tsx
git commit -m 'feat(web): add audit log toggles to settings page

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>'
```

---

## Task 5: Update Home Page Quick Start

**Files:**
- Modify: `web/src/pages/Home.tsx`

- [ ] **Step 1: Fetch server_host from settings**

Add to Home component:
```tsx
const { data: settings } = useSettings();
```

- [ ] **Step 2: Use server_host in curl examples**

Replace localhost with settings.server_host || 'localhost':
```tsx
// In quickStartExamples.openai.curl and quickStartExamples.anthropic.curl
// Change: http://localhost:8080
// To: ${settings?.server_host || 'http://localhost:8080'}
```

- [ ] **Step 3: Run tests**

Run: `npm run build` in web/
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add web/src/pages/Home.tsx
git commit -m 'feat(web): show server_host in Quick Start

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>'
```

---

## Summary

This plan adds 5 tasks covering:
1. Backend Settings API (server_host, audit_log_request, audit_log_response)
2. AuditLogger with settings-based logging
3. Frontend types and hooks
4. Settings page UI with toggles
5. Home page Quick Start with server_host

**Plan complete and saved to `docs/superpowers/plans/2026-04-14-system-settings-audit-config.md`.**

Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?