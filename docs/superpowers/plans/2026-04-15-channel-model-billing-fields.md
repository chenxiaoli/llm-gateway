# ChannelModel Billing Fields Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add billing_type, input_price, output_price, request_price fields to ChannelModel entity to allow per-channel-model pricing override.

**Architecture:** Add columns to channel_models table via migration, update Rust types, API handlers, and frontend form. Existing markup_ratio field preserved. Pricing resolution follows: channel_model → model → pricing_policy priority.

**Tech Stack:** Rust (Axum, SQLite), React (TypeScript)

---

### Task 1: Database Migration

**Files:**
- Create: `crates/storage/migrations/20260415000003_add_channel_model_billing_fields.sql`

- [ ] **Step 1: Create migration file**

```sql
-- Migration: Add billing and pricing fields to channel_models table
-- Date: 2026-04-15

ALTER TABLE channel_models ADD COLUMN billing_type TEXT;
ALTER TABLE channel_models ADD COLUMN input_price REAL;
ALTER TABLE channel_models ADD COLUMN output_price REAL;
ALTER TABLE channel_models ADD COLUMN request_price REAL;
```

- [ ] **Step 2: Commit**

```bash
git add crates/storage/migrations/20260415000003_add_channel_model_billing_fields.sql
git commit -m "db: add billing and pricing fields to channel_models table"
```

---

### Task 2: Update Rust Types

**Files:**
- Modify: `crates/storage/src/types.rs:250-290` (ChannelModel, CreateChannelModel, UpdateChannelModel)
- Modify: `crates/storage/src/sqlite.rs` (channel_model CRUD operations)

- [ ] **Step 1: Update ChannelModel struct in types.rs**

```rust
// Original (lines 250-262):
pub struct ChannelModel {
    pub id: String,
    pub channel_id: String,
    pub model_id: String,
    pub upstream_model_name: String,
    pub priority_override: Option<i32>,
    pub cost_policy_id: Option<String>,
    pub markup_ratio: f64,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// New: Add billing and pricing fields
pub struct ChannelModel {
    pub id: String,
    pub channel_id: String,
    pub model_id: String,
    pub upstream_model_name: String,
    pub priority_override: Option<i32>,
    pub cost_policy_id: Option<String>,
    pub markup_ratio: f64,
    pub billing_type: Option<String>,   // NEW
    pub input_price: Option<f64>,       // NEW
    pub output_price: Option<f64>,      // NEW
    pub request_price: Option<f64>,      // NEW
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

- [ ] **Step 2: Update CreateChannelModel in types.rs**

```rust
// Original (lines 264-272):
#[derive(Debug, Deserialize)]
pub struct CreateChannelModel {
    pub channel_id: String,
    pub model_id: String,
    pub upstream_model_name: String,
    pub priority_override: Option<i32>,
    pub cost_policy_id: Option<String>,
    pub markup_ratio: Option<f64>,
}

// New:
#[derive(Debug, Deserialize)]
pub struct CreateChannelModel {
    pub channel_id: String,
    pub model_id: String,
    pub upstream_model_name: String,
    pub priority_override: Option<i32>,
    pub cost_policy_id: Option<String>,
    pub markup_ratio: Option<f64>,
    pub billing_type: Option<String>,   // NEW
    pub input_price: Option<f64>,        // NEW
    pub output_price: Option<f64>,       // NEW
    pub request_price: Option<f64>,       // NEW
}
```

- [ ] **Step 3: Update UpdateChannelModel in types.rs**

```rust
// Original (lines 274-285):
#[derive(Debug, Deserialize)]
pub struct UpdateChannelModel {
    pub upstream_model_name: Option<String>,
    pub priority_override: Option<Option<i32>>,
    pub cost_policy_id: Option<Option<String>>,
    pub markup_ratio: Option<f64>,
    pub enabled: Option<bool>,
}

// New:
#[derive(Debug, Deserialize)]
pub struct UpdateChannelModel {
    pub upstream_model_name: Option<String>,
    pub priority_override: Option<Option<i32>>,
    pub cost_policy_id: Option<Option<String>>,
    pub markup_ratio: Option<f64>,
    pub billing_type: Option<String>,      // NEW
    pub input_price: Option<f64>,           // NEW
    pub output_price: Option<f64>,          // NEW
    pub request_price: Option<f64>,          // NEW
    pub enabled: Option<bool>,
}
```

- [ ] **Step 4: Update sqlite.rs channel_model queries**

Find and update:
- `create_channel_model` function: Add new fields to INSERT
- `get_channel_model` function: Add new fields to SELECT
- `list_channel_models` function: Add new fields to SELECT
- `update_channel_model` function: Add new fields to UPDATE

Add to INSERT: `.bind(&cm.billing_type)`, `.bind(&cm.input_price)`, `.bind(&cm.output_price)`, `.bind(&cm.request_price)`

Add to SELECT: `billing_type`, `input_price`, `output_price`, `request_price`

- [ ] **Step 5: Run cargo check to verify**

```bash
cd /workspace && cargo check --package llm-gateway-storage
```

Expected: No errors

- [ ] **Step 6: Commit**

```bash
git add crates/storage/src/types.rs crates/storage/src/sqlite.rs
git commit -m "feat: add billing fields to ChannelModel struct"
```

---

### Task 3: Update API Handlers

**Files:**
- Modify: `crates/api/src/management/channel_models.rs`

- [ ] **Step 1: Update create_channel_model handler**

Accept new fields from input and pass to storage:

```rust
// In create_channel_model function, add to ChannelModel construction:
billing_type: input.billing_type,
input_price: input.input_price,
output_price: input.output_price,
request_price: input.request_price,
```

- [ ] **Step 2: Update update_channel_model handler**

Accept and apply new fields:

```rust
// In update_channel_model function, add after other field updates:
if let Some(billing_type) = input.billing_type {
    cm.billing_type = Some(billing_type);
}
if let Some(input_price) = input.input_price {
    cm.input_price = Some(input_price);
}
if let Some(output_price) = input.output_price {
    cm.output_price = Some(output_price);
}
if let Some(request_price) = input.request_price {
    cm.request_price = Some(request_price);
}
```

- [ ] **Step 3: Run cargo check**

```bash
cd /workspace && cargo check --package llm-gateway-api
```

Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/management/channel_models.rs
git commit -m "feat: support billing fields in channel_model API handlers"
```

---

### Task 4: Update Frontend

**Files:**
- Modify: `web/src/types/index.ts` (ChannelModel type)
- Modify: `web/src/pages/ProviderDetail.tsx` or relevant channel-model form

- [ ] **Step 1: Update ChannelModel type in types/index.ts**

```typescript
// Add billing fields to ChannelModel interface
export interface ChannelModel {
  id: string;
  channel_id: string;
  model_id: string;
  upstream_model_name: string;
  priority_override?: number;
  cost_policy_id?: string;
  markup_ratio: number;
  billing_type?: string;      // NEW
  input_price?: number;       // NEW
  output_price?: number;      // NEW
  request_price?: number;     // NEW
  enabled: boolean;
  created_at: string;
  updated_at: string;
}
```

- [ ] **Step 2: Update CreateChannelModelRequest type**

```typescript
export interface CreateChannelModelRequest {
  channel_id: string;
  model_id: string;
  upstream_model_name: string;
  priority_override?: number;
  cost_policy_id?: string;
  markup_ratio?: number;
  billing_type?: string;      // NEW
  input_price?: number;       // NEW
  output_price?: number;      // NEW
  request_price?: number;     // NEW
}
```

- [ ] **Step 3: Update ChannelModel form**

Add billing_type dropdown and pricing inputs to the channel-model create/edit form in ProviderDetail.tsx:

```typescript
// Add BILLING_TYPES constant
const BILLING_TYPES = [
  { value: 'per_token', label: 'Per Token' },
  { value: 'per_request', label: 'Per Request' },
  { value: 'per_character', label: 'Per Character' },
  { value: 'tiered_token', label: 'Tiered Token' },
  { value: 'hybrid', label: 'Hybrid' },
];

// Add to form:
// - billing_type select dropdown
// - input_price number input
// - output_price number input
// - request_price number input
```

- [ ] **Step 4: Commit**

```bash
git add web/src/types/index.ts web/src/pages/ProviderDetail.tsx
git commit -m "feat: add billing fields to frontend channel model form"
```

---

### Task 5: Final Verification

- [ ] **Step 1: Run full build**

```bash
cd /workspace && cargo build --release 2>&1 | tail -20
```

Expected: Build successful

- [ ] **Step 2: Commit**

```bash
git add .
git commit -m "feat: add billing fields to ChannelModel"
```