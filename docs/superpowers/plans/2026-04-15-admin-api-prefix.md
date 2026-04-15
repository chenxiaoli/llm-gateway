# Admin API Prefix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `/api/v1/admin` prefix to all admin-only management endpoints while keeping public and user endpoints unchanged.

**Architecture:** Update router registration in `crates/api/src/management/mod.rs` to use `.route("/api/v1/admin/...")` for admin endpoints. Public endpoints (`/api/v1/auth/*`, `/api/v1/version`) and user endpoints (`/api/v1/keys`, `/api/v1/usage`) remain at original paths.

**Tech Stack:** Axum router, Rust

---

### Task 1: Update admin routes to use /api/v1/admin prefix

**Files:**
- Modify: `crates/api/src/management/mod.rs:34-100`

- [ ] **Step 1: Edit management/mod.rs - change providers routes**

```rust
// Original (lines 35-38):
.route(
    "/api/v1/providers",
    post(providers::create_provider).get(providers::list_providers),
)

// New:
.route(
    "/api/v1/admin/providers",
    post(providers::create_provider).get(providers::list_providers),
)
```

- [ ] **Step 2: Run cargo check to verify syntax**

```bash
cd /workspace && cargo check --package llm-gateway-api
```

Expected: No errors

- [ ] **Step 3: Continue updating remaining admin routes**

Update each admin route path:
- `/api/v1/providers/{id}` → `/api/v1/admin/providers/{id}`
- `/api/v1/providers/{id}/channels` → `/api/v1/admin/providers/{id}/channels`
- `/api/v1/channels` → `/api/v1/admin/channels`
- `/api/v1/channels/{id}` → `/api/v1/admin/channels/{id}`
- `/api/v1/models` → `/api/v1/admin/models`
- `/api/v1/providers/{id}/models` → `/api/v1/admin/providers/{id}/models`
- `/api/v1/providers/{id}/models/{model_name}` → `/api/v1/admin/providers/{id}/models/{model_name}`
- `/api/v1/providers/{id}/sync-models` → `/api/v1/admin/providers/{id}/sync-models`
- `/api/v1/providers/{provider_id}/channel-models` → `/api/v1/admin/providers/{provider_id}/channel-models`
- `/api/v1/channel-models/{id}` → `/api/v1/admin/channel-models/{id}`
- `/api/v1/users` → `/api/v1/admin/users`
- `/api/v1/users/{id}` → `/api/v1/admin/users/{id}`
- `/api/v1/settings` → `/api/v1/admin/settings`
- `/api/v1/logs` → `/api/v1/admin/logs`
- `/api/v1/pricing-policies` → `/api/v1/admin/pricing-policies`
- `/api/v1/pricing-policies/{id}` → `/api/v1/admin/pricing-policies/{id}`

- [ ] **Step 4: Run cargo check to verify all routes**

```bash
cd /workspace && cargo check --package llm-gateway-api
```

Expected: No errors

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/management/mod.rs
git commit -m "feat: add /api/v1/admin prefix to admin routes"
```