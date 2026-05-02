# Console Models Page — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only Models page to the Console sidebar so all authenticated users can browse available models with pricing and availability info.

**Architecture:** New backend endpoint `GET /api/v1/user/models` (JWT auth) returns a `UserModelView` type that hides channel/provider internals. New React page reuses the admin Models page card-grid pattern but is read-only and omits admin details.

**Tech Stack:** Rust (Axum, SQLx), React 18, TypeScript, TanStack Query, Tailwind CSS + DaisyUI, framer-motion, lucide-react

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `crates/storage/src/types.rs` | Modify | Add `UserModelView`, `UserPricingInfo` structs |
| `crates/api/src/user_models.rs` | Create | `GET /api/v1/user/models` handler |
| `crates/api/src/lib.rs` | Modify | Register `user_models` module |
| `crates/gateway/src/main.rs` | Modify | Mount route on main router |
| `web/src/types/index.ts` | Modify | Add `UserModelView`, `UserPricingInfo` TypeScript interfaces |
| `web/src/api/userModels.ts` | Create | API client function `listUserModels()` |
| `web/src/hooks/useUserModels.ts` | Create | React Query hook `useUserModels()` |
| `web/src/pages/ConsoleModels.tsx` | Create | Read-only models page with search + cards |
| `web/src/components/Layout.tsx` | Modify | Add "Models" to `consoleItems` array |
| `web/src/App.tsx` | Modify | Add `/console/models` route |

---

### Task 1: Add backend types for user-facing model view

**Files:**
- Modify: `crates/storage/src/types.rs` (add after `ModelWithProvider` at line ~441)

- [ ] **Step 1: Add `UserPricingInfo` and `UserModelView` structs**

Add these structs after the `ModelWithProvider` definition (after line 441):

```rust
#[derive(Debug, serde::Serialize)]
pub struct UserPricingInfo {
    pub billing_type: String,
    pub config: serde_json::Value,
}

#[derive(Debug, serde::Serialize)]
pub struct UserModelView {
    pub name: String,
    pub model_type: Option<String>,
    pub pricing_policy_name: Option<String>,
    pub pricing: Option<UserPricingInfo>,
    pub is_available: bool,
    pub created_at: String,
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check --workspace`
Expected: compiles without errors

- [ ] **Step 3: Commit**

```bash
git add crates/storage/src/types.rs
git commit -m "feat: add UserModelView and UserPricingInfo types for console models"
```

---

### Task 2: Create user-facing models endpoint

**Files:**
- Create: `crates/api/src/user_models.rs`
- Modify: `crates/api/src/lib.rs` (add module declaration)
- Modify: `crates/gateway/src/main.rs` (mount route)

- [ ] **Step 1: Create the handler file**

Create `crates/api/src/user_models.rs`:

```rust
use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use std::sync::Arc;

use llm_gateway_storage::{UserModelView, UserPricingInfo};

use crate::error::ApiError;
use crate::extractors::require_auth;
use crate::AppState;

/// GET /api/v1/user/models — list models for console users (JWT auth)
pub async fn list_user_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<UserModelView>>, ApiError> {
    let _claims = require_auth(&headers, &state.jwt_secret)?;

    let models = state
        .storage
        .list_models()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let mut views: Vec<UserModelView> = Vec::new();
    for m in models {
        let channel_models = state
            .storage
            .get_channel_models_for_model(&m.model.id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        let is_available = channel_models.iter().any(|cm| cm.enabled);

        let (pricing_policy_name, pricing) = match &m.model.pricing_policy_id {
            Some(policy_id) => {
                let policy = state
                    .storage
                    .get_pricing_policy(policy_id)
                    .await
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
                match policy {
                    Some(p) => (
                        Some(p.name),
                        Some(UserPricingInfo {
                            billing_type: p.billing_type,
                            config: p.config,
                        }),
                    ),
                    None => (m.pricing_policy_name.clone(), None),
                }
            }
            None => (None, None),
        };

        views.push(UserModelView {
            name: m.model.name,
            model_type: m.model.model_type,
            pricing_policy_name,
            pricing,
            is_available,
            created_at: m.model.created_at.to_rfc3339(),
        });
    }

    Ok(Json(views))
}
```

- [ ] **Step 2: Register the module in `crates/api/src/lib.rs`**

Add after the existing `pub mod models;` line (line 4):

```rust
pub mod user_models;
```

- [ ] **Step 3: Mount the route in `crates/gateway/src/main.rs`**

Add after the existing `.route("/v1/models", get(api::models::list_models))` line (line 122):

```rust
.route("/api/v1/user/models", get(api::user_models::list_user_models))
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check --workspace`
Expected: compiles without errors

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/user_models.rs crates/api/src/lib.rs crates/gateway/src/main.rs
git commit -m "feat: add GET /api/v1/user/models endpoint for console models"
```

---

### Task 3: Add frontend TypeScript types

**Files:**
- Modify: `web/src/types/index.ts`

- [ ] **Step 1: Add `UserPricingInfo` and `UserModelView` interfaces**

Add after the existing `UpdateModelRequest` interface (after line 89) in `web/src/types/index.ts`:

```typescript
export interface UserPricingInfo {
  billing_type: string;
  config: PricingConfig;
}

export interface UserModelView {
  name: string;
  model_type: string | null;
  pricing_policy_name: string | null;
  pricing: UserPricingInfo | null;
  is_available: boolean;
  created_at: string;
}
```

Note: These reference the existing `PricingConfig` type (line 394).

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd web && npx tsc --noEmit`
Expected: no type errors

- [ ] **Step 3: Commit**

```bash
git add web/src/types/index.ts
git commit -m "feat: add UserModelView and UserPricingInfo TypeScript types"
```

---

### Task 4: Add frontend API function and hook

**Files:**
- Create: `web/src/api/userModels.ts`
- Create: `web/src/hooks/useUserModels.ts`

- [ ] **Step 1: Create API client function**

Create `web/src/api/userModels.ts`:

```typescript
import { apiClient } from './client';
import type { UserModelView } from '../types';

export async function listUserModels(): Promise<UserModelView[]> {
  const { data } = await apiClient.get<UserModelView[]>('/user/models');
  return data;
}
```

- [ ] **Step 2: Create React Query hook**

Create `web/src/hooks/useUserModels.ts`:

```typescript
import { useQuery } from '@tanstack/react-query';
import { listUserModels } from '../api/userModels';

export function useUserModels() {
  return useQuery({ queryKey: ['user-models'], queryFn: listUserModels });
}
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd web && npx tsc --noEmit`
Expected: no type errors

- [ ] **Step 4: Commit**

```bash
git add web/src/api/userModels.ts web/src/hooks/useUserModels.ts
git commit -m "feat: add listUserModels API function and useUserModels hook"
```

---

### Task 5: Create Console Models page component

**Files:**
- Create: `web/src/pages/ConsoleModels.tsx`

This is the largest task. The page follows the same card-grid + stat pill pattern as the admin `Models.tsx` but is read-only and shows user-facing data only.

- [ ] **Step 1: Create the page**

Create `web/src/pages/ConsoleModels.tsx`:

```tsx
import { useState } from 'react';
import { motion } from 'framer-motion';
import { Cpu, Search } from 'lucide-react';
import { useUserModels } from '../hooks/useUserModels';
import { useReducedMotion } from '../hooks/useReducedMotion';
import type { UserModelView } from '../types';

function formatPrice(val: number | undefined): string {
  if (val === undefined || val === null) return '—';
  return `$${val.toFixed(2)}`;
}

function StatPill({ label, value, accent }: { label: string; value: number; accent?: boolean }) {
  return (
    <div className={`inline-flex items-center gap-2 px-3 py-1.5 rounded-lg border text-sm font-mono ${
      accent
        ? 'bg-accent/5 border-accent/20 text-accent'
        : 'bg-base-200/40 border-base-300/40 text-base-content/60'
    }`}>
      <span className="text-xs font-bold uppercase tracking-wider opacity-60">{label}</span>
      <span className="font-bold">{value}</span>
    </div>
  );
}

function ConsoleModelCard({ model, index, reducedMotion }: { model: UserModelView; index: number; reducedMotion: boolean }) {
  const policy = model.pricing;
  const billingType = policy?.billing_type ?? '';
  const config = (policy?.config ?? {}) as Record<string, unknown>;
  const isPerToken = billingType === 'per_token';

  return (
    <motion.div
      initial={reducedMotion ? false : { opacity: 0, y: 16 }}
      animate={{ opacity: 1, y: 0 }}
      transition={reducedMotion ? { duration: 0 } : { duration: 0.4, delay: 0.05 + Math.min(index, 12) * 0.04, ease: [0.16, 1, 0.3, 1] }}
    >
      <div className={`
        relative rounded-2xl overflow-hidden transition-all duration-300
        ${model.is_available
          ? 'bg-base-100 border border-base-300/50 hover:border-accent/30 hover:shadow-[0_0_24px_-4px_rgba(var(--accent),0.08)]'
          : 'bg-base-100/40 border border-base-300/30 hover:border-base-300/60 hover:bg-base-100/70'
        }
        hover:-translate-y-0.5
      `}>
        {model.is_available && (
          <div className="absolute left-0 top-0 bottom-0 w-[3px] bg-accent/60 rounded-l-2xl" />
        )}

        <div className="relative p-5">
          {/* Header */}
          <div className="flex items-start justify-between mb-4">
            <div className="flex items-center gap-3">
              <div className={`
                w-10 h-10 rounded-xl flex items-center justify-center shrink-0
                ${model.is_available ? 'bg-accent/10' : 'bg-base-200/60'}
              `}>
                <Cpu className={`h-5 w-5 ${model.is_available ? 'text-accent' : 'text-base-content/40'}`} />
              </div>
              <div className="min-w-0">
                <div className="font-mono text-lg font-bold text-base-content leading-tight truncate max-w-[200px]" title={model.name}>
                  {model.name}
                </div>
                {model.model_type && (
                  <div className="text-xs mt-0.5 text-base-content/50">{model.model_type}</div>
                )}
              </div>
            </div>

            {/* Status badge */}
            <div className={`
              shrink-0 flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-bold uppercase tracking-wider border
              ${model.is_available
                ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'
                : 'bg-base-200/40 text-base-content/40 border-base-300/40'
              }
            `}>
              <span className={`w-1.5 h-1.5 rounded-full ${model.is_available ? 'bg-emerald-400' : 'bg-base-content/20'}`} />
              {model.is_available ? 'Live' : 'Idle'}
            </div>
          </div>

          {/* Divider */}
          <div className="h-px bg-base-300/20 mb-4" />

          {/* Pricing section */}
          <div>
            <div className="flex items-center gap-2 mb-3">
              <div className="text-xs font-bold uppercase tracking-widest text-base-content/50">Pricing</div>
              <div className="flex-1 h-px bg-base-300/20" />
            </div>

            {policy ? (
              <div className="space-y-2">
                <div className="flex items-center gap-2">
                  <span className={`
                    inline-flex items-center px-2 py-0.5 rounded-md text-sm font-semibold border
                    ${model.is_available
                      ? 'bg-base-200/50 text-base-content/70 border-base-300/40'
                      : 'bg-base-200/50 text-base-content/60 border-base-300/40'
                    }
                  `}>
                    {model.pricing_policy_name}
                  </span>
                  {isPerToken && (
                    <span className="text-xs text-base-content/40">per 1M tokens</span>
                  )}
                </div>

                {isPerToken ? (
                  <div className="grid grid-cols-3 gap-1.5 p-2.5 rounded-xl border bg-base-200/20 border-base-300/20">
                    {[
                      { label: 'Input', key: 'input_price_1m' },
                      { label: 'Output', key: 'output_price_1m' },
                      { label: 'Cache', key: 'cache_read_price_1m' },
                    ].map(({ label, key }) => {
                      const val = config[key] as number | undefined;
                      return (
                        <div key={label} className="flex flex-col items-center text-center py-1">
                          <span className="text-xs font-semibold text-base-content/40 mb-1">{label}</span>
                          <span className={`font-mono text-lg font-bold ${model.is_available ? 'text-base-content' : 'text-base-content/60'}`}>
                            {formatPrice(val)}
                          </span>
                        </div>
                      );
                    })}
                  </div>
                ) : (
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-mono text-base-content/60">{billingType}</span>
                  </div>
                )}
              </div>
            ) : (
              <div className="flex items-center gap-1.5">
                <div className="w-1 h-1 rounded-full bg-base-content/25" />
                <span className="text-sm italic text-base-content/40">No pricing policy</span>
              </div>
            )}
          </div>
        </div>
      </div>
    </motion.div>
  );
}

export default function ConsoleModels() {
  const { data: models, isLoading } = useUserModels();
  const [search, setSearch] = useState('');
  const reducedMotion = useReducedMotion();

  const filtered = models?.filter(m =>
    m.name.toLowerCase().includes(search.toLowerCase())
  ) ?? [];

  const totalModels = models?.length ?? 0;
  const liveModels = models?.filter(m => m.is_available).length ?? 0;
  const idleModels = totalModels - liveModels;

  if (isLoading) {
    return (
      <div className="px-6 pb-8">
        <div className="mb-8 pt-8">
          <div className="space-y-2">
            <div className="h-7 w-24 bg-base-200/60 rounded-lg animate-pulse" />
            <div className="h-4 w-48 bg-base-200/40 rounded animate-pulse" />
          </div>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {[...Array(6)].map((_, i) => (
            <div key={i} className="h-56 bg-base-100/30 rounded-2xl border border-base-300/20 animate-pulse" />
          ))}
        </div>
      </div>
    );
  }

  return (
    <div className="px-6 pb-8">
      {/* Header */}
      <motion.div
        initial={reducedMotion ? false : { opacity: 0, y: -12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={reducedMotion ? { duration: 0 } : { duration: 0.5, ease: [0.16, 1, 0.3, 1] }}
        className="mb-8 pt-8"
      >
        <div className="flex items-start justify-between">
          <div>
            <h1 className="text-3xl font-black tracking-tight text-base-content leading-none mb-1">
              Models
            </h1>
            <p className="text-base text-base-content/50">
              {totalModels === 0
                ? 'No models available yet'
                : `${liveModels} live · ${idleModels} idle`
              }
            </p>
          </div>

          {/* Search */}
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-base-content/40" />
            <input
              type="text"
              placeholder="Search models..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="input input-sm input-bordered pl-9 w-56 bg-base-200/40 border-base-300/40 focus:border-accent/40 focus:outline-none"
            />
          </div>
        </div>

        {/* Stats row */}
        {totalModels > 0 && (
          <motion.div
            initial={reducedMotion ? false : { opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            transition={reducedMotion ? { duration: 0 } : { duration: 0.4, delay: 0.1, ease: [0.16, 1, 0.3, 1] }}
            className="flex flex-wrap gap-2.5 mt-6"
          >
            <StatPill label="Total" value={totalModels} />
            <StatPill label="Live" value={liveModels} accent />
            <StatPill label="Idle" value={idleModels} />
          </motion.div>
        )}
      </motion.div>

      {/* Empty state */}
      {filtered.length === 0 && totalModels > 0 && (
        <div className="flex flex-col items-center justify-center py-20 text-center">
          <Search className="h-10 w-10 text-base-content/20 mb-4" />
          <p className="text-base-content/40">No models match "{search}"</p>
        </div>
      )}

      {/* Grid */}
      {filtered.length > 0 && (
        <motion.div
          initial="hidden"
          animate="visible"
          variants={{ hidden: {}, visible: { transition: { staggerChildren: 0.05 } } }}
          className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4"
        >
          {filtered.map((model, i) => (
            <ConsoleModelCard
              key={model.name}
              model={model}
              index={i}
              reducedMotion={reducedMotion}
            />
          ))}
        </motion.div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd web && npx tsc --noEmit`
Expected: no type errors

- [ ] **Step 3: Commit**

```bash
git add web/src/pages/ConsoleModels.tsx
git commit -m "feat: add ConsoleModels page with read-only model cards"
```

---

### Task 6: Wire up routing and sidebar navigation

**Files:**
- Modify: `web/src/components/Layout.tsx`
- Modify: `web/src/App.tsx`

- [ ] **Step 1: Add "Models" to Console sidebar in Layout.tsx**

In `web/src/components/Layout.tsx`, add the `RectangleStack` import to the lucide-react import block (line 3-26). Add `RectangleStack` to the import list.

Then add a new entry to the `consoleItems` array (lines 31-36). The array should become:

```typescript
const consoleItems = [
  { key: '/console/dashboard', icon: LayoutDashboard, label: 'Dashboard' },
  { key: '/console/keys', icon: KeyRound, label: 'API Keys' },
  { key: '/console/model-fallbacks', icon: ArrowRightLeft, label: 'Model Fallbacks' },
  { key: '/console/models', icon: RectangleStack, label: 'Models' },
  { key: '/console/usage', icon: BarChart3, label: 'Usage' },
];
```

The `routeLabels` map already has `models: 'Models'` (line 57), so no change needed there.

- [ ] **Step 2: Add route in App.tsx**

In `web/src/App.tsx`:

First, add the import after the existing `import ModelFallbacks` line (line 14):

```typescript
import ConsoleModels from './pages/ConsoleModels';
```

Then add the route inside the console routes block, after the `model-fallbacks` route (line 60). The routes should become:

```tsx
<Route path="model-fallbacks" element={<ModelFallbacks />} />
<Route path="models" element={<ConsoleModels />} />
<Route path="usage" element={<Usage />} />
```

- [ ] **Step 3: Verify TypeScript compiles and dev server starts**

Run: `cd web && npx tsc --noEmit`
Expected: no type errors

- [ ] **Step 4: Commit**

```bash
git add web/src/components/Layout.tsx web/src/App.tsx
git commit -m "feat: add Models to console sidebar and routing"
```

---

### Task 7: Build verification and manual testing

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
3. Login as a regular user (not admin)
4. Verify "Models" appears in the Console sidebar
5. Navigate to Models page
6. Verify cards show model name, type, pricing, and live/idle status
7. Verify no channel names or internal IDs are visible
8. Test search filtering
9. Verify admin Models page still works at `/admin/models`
