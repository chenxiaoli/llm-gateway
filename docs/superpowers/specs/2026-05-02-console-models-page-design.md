# Console Models Page — Design Spec

**Date:** 2026-05-02
**Status:** Approved

## Goal

Add a read-only Models listing page to the Console sidebar section (visible to all authenticated users) so users can browse available models, see pricing, and check availability without needing admin access.

## Background

Currently, the Models page lives under the Admin section only. Regular authenticated users have no way to see which models are available, their pricing, or whether they're live. The existing non-admin endpoint (`GET /v1/models`) uses API-key auth and returns minimal OpenAI-compatible data unsuitable for a rich UI.

## Design

### Backend: New user-facing endpoint

**Endpoint:** `GET /api/v1/user/models`
**Auth:** JWT session (same as other `/api/v1/` endpoints)

**New response type `UserModelView`:**

```rust
struct UserModelView {
    name: String,
    model_type: Option<String>,
    pricing_policy_name: Option<String>,
    pricing: Option<UserPricingInfo>,
    is_available: bool,
    created_at: String,
}
```

`UserPricingInfo` contains the display-ready pricing breakdown (input/output/cache per-token rates) extracted from the pricing policy, not the internal policy ID.

**Logic:**
1. Call `state.storage.list_models()` to get all `ModelWithProvider` entries.
2. For each model, check if it has at least one enabled `ChannelModel` via `state.storage.get_channel_models_for_model()`.
3. If a `pricing_policy_id` is set, look up the policy and extract display-ready pricing.
4. Map to `UserModelView`, stripping `channel_ids`, `channel_names`, and `pricing_policy_id`.
5. Return `Vec<UserModelView>`.

**What is NOT exposed:** channel IDs, channel names, pricing_policy_id, internal model ID — these are admin infrastructure details.

### Frontend: API layer

**New file:** `web/src/api/userModels.ts`
- `listUserModels()`: GET `/api/v1/user/models` via `apiClient` (JWT auth)

**New file:** `web/src/hooks/useUserModels.ts`
- `useUserModels()`: React Query hook wrapping `listUserModels()`, query key `['user-models']`

### Frontend: Page component

**New file:** `web/src/pages/ConsoleModels.tsx`

**Layout:**
- Search bar at top to filter models by name
- Stat pills: Total / Live / Idle counts
- Responsive card grid (1/2/3 columns)

**Each card shows:**
- Model name (prominent)
- Model type badge (if set)
- Availability status pill: green "Live" or gray "Idle" (based on `is_available`)
- Pricing: policy name + per-token breakdown (input/output/cache) if pricing data exists

**No:** edit/create/delete buttons, channel details, provider names, internal IDs.

**Pattern:** Follows the same card grid + stat pill pattern as the admin Models page, but simplified and read-only.

### Frontend: Routing & Navigation

- Add "Models" entry with `RectangleStackIcon` to the Console section in `Layout.tsx`
- Add route `/console/models` pointing to `ConsoleModels` in `App.tsx`

## Scope

- This is read-only. No model selection, testing, or modification.
- Search is text-only filter by model name.
- No pagination (model counts are typically small, <100).

## Files Changed

| File | Action |
|------|--------|
| `crates/api/src/user_models.rs` | New — endpoint handler |
| `crates/api/src/lib.rs` | Modify — register module |
| `crates/storage/src/types.rs` | Modify — add `UserModelView`, `UserPricingInfo` |
| `crates/gateway/src/main.rs` | Modify — mount route |
| `web/src/api/userModels.ts` | New — API client function |
| `web/src/hooks/useUserModels.ts` | New — React Query hook |
| `web/src/pages/ConsoleModels.tsx` | New — page component |
| `web/src/components/Layout.tsx` | Modify — add sidebar entry |
| `web/src/App.tsx` | Modify — add route |
| `web/src/types/index.ts` | Modify — add TypeScript types |
