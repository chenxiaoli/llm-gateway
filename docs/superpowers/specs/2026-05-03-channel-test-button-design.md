# Channel Test Button — Design Spec

**Date:** 2026-05-03
**Status:** Approved

## Goal

Add a "Test" button to each channel row on the Channels list page that sends a minimal chat completion request through the specific channel's upstream provider to verify connectivity, API key validity, and model availability.

## Background

Channel configuration involves an API key, endpoint URL, and model assignments. Currently there is no way to verify a channel works without making a manual API call. Admins need a quick way to validate that a channel's upstream connection is healthy.

## Design

### Backend: Channel test endpoint

**Endpoint:** `POST /api/v1/admin/channels/{id}/test`
**Auth:** JWT session, admin-only

**Response type `ChannelTestResult`:**

```json
{
  "success": true,
  "latency_ms": 123,
  "model": "gpt-4o",
  "error": null
}
```

On failure:

```json
{
  "success": false,
  "latency_ms": 1503,
  "model": "gpt-4o",
  "error": "401 Unauthorized: Invalid API key"
}
```

**Logic:**
1. Resolve channel by ID — load channel with its provider (endpoint, API key).
2. Load channel models for this channel. Pick the first enabled `ChannelModel`. If none, return `{ success: false, error: "No enabled models on this channel" }`.
3. If the model has an `upstream_model_name` override, use that as the model name; otherwise use the model's name.
4. Build a minimal chat completion request body: `{ "model": "<name>", "messages": [{"role": "user", "content": "Hi"}], "max_tokens": 5 }`.
5. Determine the upstream URL using the channel's provider endpoint (OpenAI format) + `/chat/completions`.
6. Send the request with a timeout (e.g., 30s). Measure wall-clock latency.
7. If upstream returns 2xx: `{ success: true, latency_ms, model, error: null }`.
8. If upstream returns error or times out: `{ success: false, latency_ms, model, error: "<status> <message>" }`.

### Frontend: API layer

**New file:** `web/src/api/channels.ts` (or add to existing `providers.ts`)
- `testChannel(id: string)`: POST `/admin/channels/{id}/test` via `adminApiClient`

**New file:** `web/src/hooks/useChannels.ts` (add mutation)
- `useTestChannel()`: React Query mutation, query key invalidation not needed (one-shot action)

### Frontend: Channel row button

In `web/src/pages/Channels.tsx`, modify the `ChannelRow` component:

- Add a "Test" button next to the existing "Configure" button in the row actions area.
- Button states:
  - **Default:** "Test" label with a `Zap` icon
  - **Loading:** Spinner animation, button disabled
  - **Success:** Green checkmark icon + "OK" text, auto-resets after 3 seconds
  - **Failure:** Red X icon + toast with error message, auto-resets after 3 seconds

- Success toast: `"Channel OK — {latency}ms (model: {name})"`
- Failure toast: `"Channel test failed: {error}"`

## Scope

- Admin-only feature (not exposed to console users)
- Tests only the OpenAI chat completions path (most common protocol)
- One model tested per channel (first enabled)
- No streaming — buffered response only
- Does not modify any channel state

## Files Changed

| File | Action |
|------|--------|
| `crates/storage/src/types.rs` | Modify — add `ChannelTestResult` |
| `crates/api/src/management/channels.rs` | Modify — add `test_channel` handler |
| `crates/api/src/management/mod.rs` | Modify — register route |
| `web/src/api/providers.ts` | Modify — add `testChannel()` function |
| `web/src/hooks/useChannels.ts` | Modify — add `useTestChannel()` hook |
| `web/src/pages/Channels.tsx` | Modify — add Test button to `ChannelRow` |
