# Channel Test Button — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a "Test" button to each channel row on the Channels page that sends a minimal chat completion request through the channel's upstream provider.

**Architecture:** New `POST /api/v1/admin/channels/{id}/test` endpoint resolves the channel, picks the first enabled model, sends a minimal completion request to the upstream provider, and returns success/failure with latency. Frontend adds a button to each channel row with inline status feedback and toast notifications.

**Tech Stack:** Rust (Axum, reqwest, serde), React 18, TypeScript, TanStack Query, Tailwind CSS + DaisyUI, lucide-react

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `crates/storage/src/types.rs` | Modify | Add `ChannelTestResult` struct |
| `crates/api/src/management/channels.rs` | Modify | Add `test_channel` handler |
| `crates/api/src/management/mod.rs` | Modify | Register test route |
| `web/src/types/index.ts` | Modify | Add `ChannelTestResult` interface |
| `web/src/api/providers.ts` | Modify | Add `testChannel()` function |
| `web/src/hooks/useChannels.ts` | Modify | Add `useTestChannel()` hook |
| `web/src/pages/Channels.tsx` | Modify | Add Test button to `ChannelRow` |

---

### Task 1: Add backend `ChannelTestResult` type

**Files:**
- Modify: `crates/storage/src/types.rs` (add after `ChannelModel` struct at line ~473)

- [ ] **Step 1: Add `ChannelTestResult` struct**

Add after the `ChannelModel` struct (after line 473) in `crates/storage/src/types.rs`:

```rust
#[derive(Debug, serde::Serialize)]
pub struct ChannelTestResult {
    pub success: bool,
    pub latency_ms: u64,
    pub model: String,
    pub error: Option<String>,
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check --workspace`
Expected: compiles without errors

- [ ] **Step 3: Commit**

```bash
git add crates/storage/src/types.rs
git commit -m "feat: add ChannelTestResult type for channel testing"
```

---

### Task 2: Create `test_channel` backend handler

**Files:**
- Modify: `crates/api/src/management/channels.rs` (add handler)
- Modify: `crates/api/src/management/mod.rs` (register route)

- [ ] **Step 1: Add the `test_channel` handler**

Add at the end of `crates/api/src/management/channels.rs`, before the closing (there is no closing — just append after `delete_channel` at line 449):

```rust
pub async fn test_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<llm_gateway_storage::ChannelTestResult>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    // 1. Resolve channel
    let channel = state
        .storage
        .get_channel(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Channel '{}' not found", id)))?;

    // 2. Resolve provider (for endpoint)
    let provider = state
        .storage
        .get_provider(&channel.provider_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Provider '{}' not found", channel.provider_id)))?;

    // 3. Get channel models, pick first enabled
    let channel_models = state
        .storage
        .list_channel_models_by_channel(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let cm = channel_models
        .iter()
        .find(|cm| cm.enabled)
        .ok_or(ApiError::BadRequest("No enabled models on this channel".to_string()))?;

    // 4. Resolve model name (upstream override or actual name)
    let model_name = match &cm.upstream_model_name {
        Some(name) => name.clone(),
        None => state
            .storage
            .get_model_by_id(&cm.model_id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .map(|m| m.name)
            .unwrap_or_else(|| cm.model_id.clone()),
    };

    // 5. Build upstream URL from provider endpoint
    let endpoints: serde_json::Value = provider
        .endpoints
        .and_then(|e| serde_json::from_str(&e).ok())
        .unwrap_or(serde_json::Value::Null);

    let base_url = endpoints
        .get("openai")
        .and_then(|v| v.as_str())
        .or_else(|| endpoints.get("default").and_then(|v| v.as_str()))
        .unwrap_or("")
        .trim_end_matches('/');

    let upstream_url = format!("{}/chat/completions", base_url);

    // 6. Decrypt API key
    let api_key = decrypt(&channel.api_key, &state.encryption_key)
        .unwrap_or_else(|_| channel.api_key.clone());

    // 7. Build minimal request body
    let body = serde_json::json!({
        "model": model_name,
        "messages": [{"role": "user", "content": "Hi"}],
        "max_tokens": 5
    });

    // 8. Send request with timeout
    let start = std::time::Instant::now();
    let client = reqwest::Client::new();
    let result = client
        .post(&upstream_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(30))
        .json(&body)
        .send()
        .await;

    let latency_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                Ok(Json(llm_gateway_storage::ChannelTestResult {
                    success: true,
                    latency_ms,
                    model: model_name,
                    error: None,
                }))
            } else {
                let status_code = status.as_u16();
                let error_body = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Ok(Json(llm_gateway_storage::ChannelTestResult {
                    success: false,
                    latency_ms,
                    model: model_name,
                    error: Some(format!("{} {}", status_code, error_body)),
                }))
            }
        }
        Err(e) => Ok(Json(llm_gateway_storage::ChannelTestResult {
            success: false,
            latency_ms,
            model: model_name,
            error: Some(e.to_string()),
        })),
    }
}
```

- [ ] **Step 2: Register the route in `crates/api/src/management/mod.rs`**

Add a new route after the existing `/api/v1/admin/channels/{id}/api-key` route (after line 75):

```rust
.route(
    "/api/v1/admin/channels/{id}/test",
    post(channels::test_channel),
)
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check --workspace`
Expected: compiles without errors

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/management/channels.rs crates/api/src/management/mod.rs
git commit -m "feat: add POST /api/v1/admin/channels/{id}/test endpoint"
```

---

### Task 3: Add frontend TypeScript type

**Files:**
- Modify: `web/src/types/index.ts`

- [ ] **Step 1: Add `ChannelTestResult` interface**

Add after the `UpdateChannelApiKeyRequest` interface (around line 328) in `web/src/types/index.ts`:

```typescript
export interface ChannelTestResult {
  success: boolean;
  latency_ms: number;
  model: string;
  error: string | null;
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd web && npx tsc --noEmit`
Expected: no type errors

- [ ] **Step 3: Commit**

```bash
git add web/src/types/index.ts
git commit -m "feat: add ChannelTestResult TypeScript type"
```

---

### Task 4: Add frontend API function and hook

**Files:**
- Modify: `web/src/api/providers.ts`
- Modify: `web/src/hooks/useChannels.ts`

- [ ] **Step 1: Add `testChannel()` API function**

In `web/src/api/providers.ts`, add the `ChannelTestResult` type to the import on line 2 (add `ChannelTestResult` to the import list), then add this function after `deleteChannel` (after line 65):

```typescript
export async function testChannel(id: string): Promise<ChannelTestResult> {
  const { data } = await adminApiClient.post<ChannelTestResult>(`/channels/${id}/test`);
  return data;
}
```

- [ ] **Step 2: Add `useTestChannel()` hook**

In `web/src/hooks/useChannels.ts`, add `testChannel` to the import from `../api/providers` on line 2 (add `testChannel`), and add `ChannelTestResult` to the types import on line 3. Then add this hook at the end of the file (after the `useUpdateChannelApiKey` function at line 124):

```typescript
export function useTestChannel() {
  return useMutation({
    mutationFn: (id: string) => testChannel(id),
  });
}
```

Note: No query invalidation needed — this is a one-shot action that doesn't change any data. No toast either — the caller handles feedback directly based on the result.

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd web && npx tsc --noEmit`
Expected: no type errors

- [ ] **Step 4: Commit**

```bash
git add web/src/api/providers.ts web/src/hooks/useChannels.ts
git commit -m "feat: add testChannel API function and useTestChannel hook"
```

---

### Task 5: Add Test button to ChannelRow

**Files:**
- Modify: `web/src/pages/Channels.tsx`

- [ ] **Step 1: Add imports**

At the top of `web/src/pages/Channels.tsx`, make these changes to the imports:

Add `useEffect` to the React import on line 1:
```typescript
import { useState, useEffect } from 'react';
```

Add `Zap`, `Check`, `X`, `Loader2` to the lucide-react imports (search for the existing `import { ... } from 'lucide-react'` block and add these four icons).

Add `useTestChannel` to the `useChannels` import on line 3:
```typescript
import { useAllChannels, useTestChannel } from '../hooks/useChannels';
```

Add `toast` import (if not already present):
```typescript
import { toast } from 'sonner';
```

Add `getErrorMessage` import:
```typescript
import { getErrorMessage } from '../api/client';
```

- [ ] **Step 2: Update `ChannelRowProps` and add state**

Modify the `ChannelRowProps` interface and component signature. Replace lines 346-352:

```typescript
interface ChannelRowProps {
  channel: Channel;
  providerName: string;
  index: number;
}

function ChannelRow({ channel, providerName, index }: ChannelRowProps) {
  const testMutation = useTestChannel();
  const [testStatus, setTestStatus] = useState<'idle' | 'loading' | 'success' | 'error'>('idle');

  useEffect(() => {
    if (testStatus === 'success' || testStatus === 'error') {
      const timer = setTimeout(() => setTestStatus('idle'), 3000);
      return () => clearTimeout(timer);
    }
  }, [testStatus]);

  const handleTest = () => {
    setTestStatus('loading');
    testMutation.mutate(channel.id, {
      onSuccess: (result) => {
        if (result.success) {
          setTestStatus('success');
          toast.success(`Channel OK — ${result.latency_ms}ms (model: ${result.model})`);
        } else {
          setTestStatus('error');
          toast.error(`Channel test failed: ${result.error ?? 'Unknown error'}`);
        }
      },
      onError: (err) => {
        setTestStatus('error');
        toast.error(getErrorMessage(err, 'Channel test failed'));
      },
    });
  };

  const channelModels = channel.models ?? [];
```

- [ ] **Step 3: Add Test button to the quick actions area**

Replace the quick actions div (lines 474-482) with:

```tsx
        {/* Quick actions */}
        <div className="flex items-center gap-1 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity duration-150">
          <button
            onClick={handleTest}
            disabled={testStatus === 'loading'}
            className={`inline-flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-md font-medium transition-all duration-100 border border-transparent ${
              testStatus === 'success'
                ? 'text-success hover:text-success'
                : testStatus === 'error'
                ? 'text-error hover:text-error'
                : 'text-base-content/50 hover:text-base-content/80 hover:bg-base-200/70 hover:border-base-300/40'
            }`}
          >
            {testStatus === 'loading' && <Loader2 className="h-3 w-3 animate-spin" />}
            {testStatus === 'success' && <Check className="h-3 w-3" />}
            {testStatus === 'error' && <X className="h-3 w-3" />}
            {testStatus === 'idle' && <Zap className="h-3 w-3" />}
            {testStatus === 'loading' ? 'Testing' : testStatus === 'success' ? 'OK' : testStatus === 'error' ? 'Fail' : 'Test'}
          </button>
          <Link
            to={`/admin/channels/${channel.id}`}
            className="inline-flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-md font-medium text-base-content/50 hover:text-base-content/80 hover:bg-base-200/70 transition-all duration-100 border border-transparent hover:border-base-300/40"
          >
            <Wifi className="h-3 w-3" />
            Configure
          </Link>
        </div>
```

- [ ] **Step 4: Verify TypeScript compiles**

Run: `cd web && npx tsc --noEmit`
Expected: no type errors

- [ ] **Step 5: Commit**

```bash
git add web/src/pages/Channels.tsx
git commit -m "feat: add Test button to channel rows with inline status feedback"
```

---

### Task 6: Build verification

**Files:** None (verification only)

- [ ] **Step 1: Run full backend build**

Run: `cargo build --workspace`
Expected: compiles without errors

- [ ] **Step 2: Run frontend build**

Run: `cd web && npm run build`
Expected: builds without errors

- [ ] **Step 3: Run existing tests**

Run: `cargo test --workspace`
Expected: all existing tests pass (no regressions)

Run: `cd web && npm test`
Expected: all existing tests pass

- [ ] **Step 4: Manual test**

1. Start backend: `cargo run`
2. Start frontend: `cd web && npm run dev`
3. Login as admin
4. Navigate to Channels page (`/admin/channels`)
5. Hover over a channel row — verify "Test" button appears
6. Click "Test" on a channel with a valid API key
7. Verify spinner shows during test, then green "OK" appears
8. Verify toast shows "Channel OK — Xms (model: Y)"
9. Verify button resets to "Test" after 3 seconds
10. Click "Test" on a channel with an invalid API key
11. Verify red "Fail" appears with error toast
