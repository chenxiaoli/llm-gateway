# Model billing_type Field Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `billing_type` field to Model entity with support for 5 billing types: per_token, per_request, per_character, tiered_token, hybrid.

**Architecture:** Add column to models table via migration, update Rust types, API handlers, and frontend form. Keep backward compatibility with existing "token" and "request" values by mapping to new "per_token" and "per_request".

**Tech Stack:** Rust (Axum, SQLite), React (TypeScript)

---

### Task 1: Database Migration

**Files:**
- Create: `crates/storage/migrations/20260415000002_add_model_billing_type.sql`

- [ ] **Step 1: Create migration file**

```sql
-- Migration: Add billing_type column to models table
-- Date: 2026-04-15

ALTER TABLE models ADD COLUMN billing_type TEXT NOT NULL DEFAULT 'per_token' 
    CHECK(billing_type IN ('per_token', 'per_request', 'per_character', 'tiered_token', 'hybrid'));
```

- [ ] **Step 2: Commit**

```bash
git add crates/storage/migrations/20260415000002_add_model_billing_type.sql
git commit -m "db: add billing_type column to models table"
```

---

### Task 2: Update Rust Types

**Files:**
- Modify: `crates/storage/src/types.rs:149-160` (Model struct)
- Modify: `crates/storage/src/types.rs:219-235` (CreateModel, UpdateModel)
- Modify: `crates/storage/src/sqlite.rs:781-870` (model CRUD operations)

- [ ] **Step 1: Update Model struct in types.rs**

```rust
// Original (lines 149-160):
pub struct Model {
    pub id: String,
    pub name: String,
    pub provider_id: String,
    pub model_type: Option<String>,
    pub pricing_policy_id: Option<String>,
    pub input_price: f64,
    pub output_price: f64,
    pub request_price: f64,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

// New: Add billing_type field
pub struct Model {
    pub id: String,
    pub name: String,
    pub provider_id: String,
    pub model_type: Option<String>,
    pub pricing_policy_id: Option<String>,
    pub billing_type: String,  // NEW: "per_token" | "per_request" | "per_character" | "tiered_token" | "hybrid"
    pub input_price: f64,
    pub output_price: f64,
    pub request_price: f64,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}
```

- [ ] **Step 2: Update CreateModel in types.rs**

```rust
// Original (lines 219-226):
#[derive(Debug, Deserialize)]
pub struct CreateModel {
    pub name: String,
    pub pricing_policy_id: Option<String>,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub request_price: Option<f64>,
}

// New:
#[derive(Debug, Deserialize)]
pub struct CreateModel {
    pub name: String,
    pub pricing_policy_id: Option<String>,
    pub billing_type: Option<String>,  // NEW
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub request_price: Option<f64>,
}
```

- [ ] **Step 3: Update UpdateModel in types.rs**

```rust
// Original (lines 228-235):
#[derive(Debug, Deserialize)]
pub struct UpdateModel {
    pub pricing_policy_id: Option<Option<String>>,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub request_price: Option<f64>,
    pub enabled: Option<bool>,
}

// New:
#[derive(Debug, Deserialize)]
pub struct UpdateModel {
    pub pricing_policy_id: Option<Option<String>>,
    pub billing_type: Option<String>,  // NEW
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub request_price: Option<f64>,
    pub enabled: Option<bool>,
}
```

- [ ] **Step 4: Update sqlite.rs model queries**

Find and update:
- `create_model` function (line ~781): Add `billing_type` to INSERT
- `get_model` function (line ~802): Add `billing_type` to SELECT
- `update_model` function (line ~853): Add `billing_type` to UPDATE

Example for create_model:
```rust
// In INSERT statement, add:
.bind(&model.billing_type)
```

- [ ] **Step 5: Run cargo check to verify**

```bash
cd /workspace && cargo check --package llm-gateway-storage
```

Expected: No errors

- [ ] **Step 6: Commit**

```bash
git add crates/storage/src/types.rs crates/storage/src/sqlite.rs
git commit -m "feat: add billing_type field to Model struct"
```

---

### Task 3: Update API Handlers

**Files:**
- Modify: `crates/api/src/management/models.rs`

- [ ] **Step 1: Update create_model handler**

Find `create_model` function and:
1. Accept `billing_type` from input
2. Default to "per_token" if not provided
3. Pass to storage `create_model` call

```rust
// In create_model function, add:
let billing_type = input.billing_type.unwrap_or_else(|| "per_token".to_string());
let model = Model {
    id: uuid::Uuid::new_v4().to_string(),
    name: input.name,
    provider_id,
    model_type: input.model_type,
    pricing_policy_id: input.pricing_policy_id,
    billing_type,  // NEW
    input_price: input.input_price.unwrap_or(0.0),
    output_price: input.output_price.unwrap_or(0.0),
    request_price: input.request_price.unwrap_or(0.0),
    enabled: true,
    created_at: chrono::Utc::now(),
};
```

- [ ] **Step 2: Update update_model handler**

Find `update_model` function and:
1. Accept `billing_type` from input
2. Update model.billing_type if provided

```rust
// In update_model function, add after other field updates:
if let Some(billing_type) = input.billing_type {
    model.billing_type = billing_type;
}
```

- [ ] **Step 3: Run cargo check**

```bash
cd /workspace && cargo check --package llm-gateway-api
```

Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/management/models.rs
git commit -m "feat: support billing_type in model API handlers"
```

---

### Task 4: Update Frontend

**Files:**
- Modify: `web/src/types/index.ts` (Model type)
- Modify: `web/src/pages/Models.tsx` (model form and list)

- [ ] **Step 1: Update Model type in types/index.ts**

```typescript
// Add billing_type to Model interface
export interface Model {
  id: string;
  name: string;
  provider_id: string;
  model_type?: string;
  pricing_policy_id?: string;
  billing_type: string;  // NEW
  input_price: number;
  output_price: number;
  request_price: number;
  enabled: boolean;
  created_at: string;
}
```

- [ ] **Step 2: Update Models.tsx form**

Add billing_type dropdown to the model create/edit form:

```typescript
// Options for billing_type
const BILLING_TYPES = [
  { value: 'per_token', label: 'Per Token' },
  { value: 'per_request', label: 'Per Request' },
  { value: 'per_character', label: 'Per Character' },
  { value: 'tiered_token', label: 'Tiered Token' },
  { value: 'hybrid', label: 'Hybrid' },
];

// Add to form:
<select
  value={formData.billing_type || 'per_token'}
  onChange={e => setFormData({ ...formData, billing_type: e.target.value })}
>
  {BILLING_TYPES.map(t => (
    <option key={t.value} value={t.value}>{t.label}</option>
  ))}
</select>
```

- [ ] **Step 3: Update Models.tsx list**

Add billing_type column to the models table:

```typescript
// In table columns, add:
{
  title: 'Billing Type',
  dataIndex: 'billing_type',
  key: 'billing_type',
  render: (text: string) => (
    <Tag>{text}</Tag>
  ),
}
```

- [ ] **Step 4: Commit**

```bash
git add web/src/types/index.ts web/src/pages/Models.tsx
git commit -m "feat: add billing_type to frontend model form and list"
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
git commit -m "feat: add billing_type field to Model"
```