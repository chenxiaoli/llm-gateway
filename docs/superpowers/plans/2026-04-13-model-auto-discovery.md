# Model Auto-Discovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add on-demand model discovery feature - admin clicks "Sync Models" button to fetch available models from upstream providers and save to database.

**Architecture:** Add `model_type` field to Model, create sync endpoint that calls upstream `/models` for each protocol, upserts to database with `enabled: false` for new models.

**Tech Stack:** Rust (axum), SQLite, React

---

## File Structure

### Backend Files
- `crates/storage/src/types.rs` - Model struct
- `crates/storage/src/sqlite.rs` - SQLite implementation
- `crates/storage/src/lib.rs` - Storage trait
- `crates/api/src/management/models.rs` - Sync endpoint
- `crates/api/src/management/mod.rs` - Route registration
- `data/gateway.db` - SQLite database file

### Frontend Files
- `web/src/pages/ProviderDetail.tsx` - Add sync button
- `web/src/hooks/useProviders.ts` - Add sync mutation
- `web/src/api/providers.ts` - Add sync API call
- `web/src/types/index.ts` - Add SyncModelsResponse type

---

## Task 1: Database Migration

**Files:**
- Modify: `data/gateway.db` (SQLite)

- [ ] **Step 1: Add model_type column to models table**

Run: `sqlite3 data/gateway.db "ALTER TABLE models ADD COLUMN model_type VARCHAR(100);"`

Verify: `sqlite3 data/gateway.db ".schema models" | grep model_type`

Expected: Column appears in schema

- [ ] **Step 2: Commit**

```bash
git add data/gateway.db
git commit -m "db: add model_type column to models table"
```

---

## Task 2: Update Storage Types

**Files:**
- Modify: `crates/storage/src/types.rs:131-147`

- [ ] **Step 1: Find Model struct and add model_type field**

Read: `crates/storage/src/types.rs` lines 131-147

Current:
```rust
pub struct Model {
    pub name: String,
    pub provider_id: String,
    pub billing_type: BillingType,
    pub input_price: f64,
    pub output_price: f64,
    pub request_price: f64,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}
```

Add `model_type` after `provider_id`:
```rust
pub struct Model {
    pub name: String,
    pub provider_id: String,
    pub model_type: Option<String>,
    pub billing_type: BillingType,
    pub input_price: f64,
    pub output_price: f64,
    pub request_price: f64,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}
```

- [ ] **Step 2: Update SqliteChannelRow and From impl**

Read: `crates/storage/src/sqlite.rs` lines 276-290 (SqliteChannelRow)

Add `model_type` field to struct, update From impl the same way.

- [ ] **Step 3: Ensure existing code builds**

Run: `cargo check --package llm-gateway-storage`

Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add crates/storage/src/types.rs crates/storage/src/sqlite.rs
git commit -m "feat(storage): add model_type to Model struct"
```

---

## Task 3: Update Storage Trait

**Files:**
- Modify: `crates/storage/src/lib.rs:150-170` (around Model references)

- [ ] **Step 1: Check create_model signature**

Read: `crates/storage/src/lib.rs` - find `create_model` function signature

- [ ] **Step 2: Check if create_model needs update**

The Model struct now has `model_type: Option<String>`. Ensure `CreateModel` accepts it. Read `CreateModel` struct in types.rs.

Read: `crates/storage/src/types.rs` lines ~150-165

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(storage): ensure CreateModel accepts model_type"
```

---

## Task 4: Add Sync Endpoint

**Files:**
- Modify: `crates/api/src/management/models.rs`

- [ ] **Step 1: Read existing models.rs**

Read: `crates/api/src/management/models.rs`

- [ ] **Step 2: Add model discovery function**

Add after existing functions (around line 160):

```rust
pub async fn sync_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
) -> Result<Json<SyncModelsResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let provider = state
        .storage
        .get_provider(&provider_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Provider '{}' not found", provider_id)))?;

    let mut new_count = 0;
    let mut updated_count = 0;
    let mut synced_models: Vec<SyncedModel> = Vec::new();

    let client = reqwest::Client::new();

    // Sync OpenAI models if base_url configured
    if let Some(openai_url) = &provider.openai_base_url {
        let url = format!("{}/models", openai_url);
        match client.get(&url)
            .header("Authorization", format!("Bearer {}", state.storage.get_provider_api_key(&provider_id).await.unwrap_or_default()))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let body = resp.text().await.unwrap_or_default();
                if let Ok(json) = serde_json::from_str::<Value>(&body) {
                    if let Some(models) = json.get("data").and_then(|m| m.as_array()) {
                        for m in models {
                            if let Some(name) = m.get("id").and_then(|n| n.as_str()) {
                                let model_type = m.get("type").and_then(|t| t.as_str()).map(String::from);
                                
                                // Check if exists
                                let existing = state.storage.get_model(&provider_id, name).await.ok().flatten();
                                
                                let model = Model {
                                    name: name.to_string(),
                                    provider_id: provider_id.clone(),
                                    model_type: model_type.clone(),
                                    billing_type: BillingType::Token,
                                    input_price: 0.0,
                                    output_price: 0.0,
                                    request_price: 0.0,
                                    enabled: false,
                                    created_at: chrono::Utc::now(),
                                };
                                
                                if existing.is_some() {
                                    updated_count += 1;
                                    synced_models.push(SyncedModel {
                                        name: name.to_string(),
                                        model_type: model_type.clone(),
                                        created: false,
                                    });
                                } else {
                                    state.storage.create_model(&provider_id, &model).await?;
                                    new_count += 1;
                                    synced_models.push(SyncedModel {
                                        name: name.to_string(),
                                        model_type: model_type.clone(),
                                        created: true,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Sync Anthropic models if base_url configured (similar pattern)
    // ... repeat for anthropic_base_url

    Ok(Json(SyncModelsResponse {
        new: new_count,
        updated: updated_count,
        models: synced_models,
    }))
}
```

- [ ] **Step 3: Add response types at top of file**

Add after imports:
```rust
#[derive(Serialize)]
pub struct SyncModelsResponse {
    pub new: i32,
    pub updated: i32,
    pub models: Vec<SyncedModel>,
}

#[derive(Serialize)]
pub struct SyncedModel {
    pub name: String,
    pub model_type: Option<String>,
    pub created: bool,
}
```

- [ ] **Step 4: Register route**

Modify: `crates/api/src/management/mod.rs`

Add route:
```rust
post(providers::sync_models).post(channels::create_channel).get(channels::list_channels),
```

As:
```rust
post(providers::sync_models).post(channels::create_channel).get(channels::list_channels),
```

Actually, models sync is under providers - add to existing providers route group.

- [ ] **Step 5: Test build**

Run: `cargo check --package llm-gateway-api`

Expected: No errors

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/management/models.rs crates/api/src/management/mod.rs
git commit -m "feat(api): add sync_models endpoint for model discovery"
```

---

## Task 5: Frontend - API Client

**Files:**
- Modify: `web/src/api/providers.ts`

- [ ] **Step 1: Add syncModels function**

Read: `web/src/api/providers.ts`

Add after existing functions:
```typescript
export async function syncModels(providerId: string): Promise<SyncModelsResponse> {
  const { data } = await apiClient.post<SyncModelsResponse>(`/providers/${providerId}/sync-models`, {});
  return data;
}
```

- [ ] **Step 2: Add type**

Read: `web/src/types/index.ts`

Add:
```typescript
export interface SyncedModel {
  name: string;
  model_type: string | null;
  created: boolean;
}

export interface SyncModelsResponse {
  new: number;
  updated: number;
  models: SyncedModel[];
}
```

- [ ] **Step 3: Commit**

```bash
git add web/src/api/providers.ts web/src/types/index.ts
git commit -m "feat(web): add syncModels API function"
```

---

## Task 6: Frontend - Hook

**Files:**
- Modify: `web/src/hooks/useProviders.ts`

- [ ] **Step 1: Add useSyncModels mutation**

Read: `web/src/hooks/useProviders.ts` - find where other mutations are defined

Add:
```typescript
export function useSyncModels(providerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => syncModels(providerId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['providers', providerId, 'models'] });
      toast.success('Models synced');
    },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to sync models')); },
  });
}
```

- [ ] **Step 2: Commit**

```bash
git add web/src/hooks/useProviders.ts
git commit -m "feat(web): add useSyncModels hook"
```

---

## Task 7: Frontend - Provider Detail Page

**Files:**
- Modify: `web/src/pages/ProviderDetail.tsx`

- [ ] **Step 1: Import hook and add button**

Read: `web/src/pages/ProviderDetail.tsx` - find where buttons are defined (around models section header)

Add button next to "Add Model" button:
```typescript
const syncModelsMutation = useSyncModels(id!);

// In the models section header:
<Button icon={<ArrowRepeat className="h-4 w-4" />} onClick={() => syncModelsMutation.mutate()} loading={syncModelsMutation.isPending}>
  Sync Models
</Button>
```

Need to import `ArrowRepeat` from lucide-react.

- [ ] **Step 2: Test build**

Run: `cd web && npm run build`

Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add web/src/pages/ProviderDetail.tsx
git commit -m "feat(web): add Sync Models button to provider detail"
```

---

## Task 8: Integration Test

**Files:**
- Test: manually verify or existing tests

- [ ] **Step 1: Test the flow**

1. Start backend: `cargo run --package llm-gateway-api`
2. Start frontend: `cd web && npm run dev`
3. Navigate to provider detail
4. Click "Sync Models"
5. Verify toast shows count
6. Verify new models appear in table with enabled: false

- [ ] **Step 2: Commit**

```bash
git commit -m "test: verify model sync feature works"
```

---

## Execution

**Plan complete and saved to `docs/superpowers/plans/2026-04-13-model-auto-discovery.md`. Two execution options:**

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**