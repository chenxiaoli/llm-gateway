# Per-Request Deduction & request_id Traceability — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace batch settlement with per-request balance deduction, linked by a shared `request_id` across usage_records, audit_logs, and transactions.

**Architecture:** Gateway generates a `request_id` (UUID) per LLM request and flows it through NATS events to both workers. Usage worker inserts the usage record, then atomically deducts balance and creates a transaction — all sharing the same `request_id`. Batch settlement worker is removed entirely.

**Tech Stack:** Rust (Axum, SQLx, NATS JetStream), PostgreSQL, React/TypeScript frontend

---

### Task 1: Database migration — add request_id columns

**Files:**
- Create: `crates/storage/migrations/postgres/20260507000000_add_request_id.sql`

- [ ] **Step 1: Create the migration file**

```sql
-- Add request_id to usage_records
ALTER TABLE usage_records ADD COLUMN request_id TEXT;
CREATE INDEX idx_usage_request_id ON usage_records(request_id);

-- Add request_id to audit_logs
ALTER TABLE audit_logs ADD COLUMN request_id TEXT;
CREATE INDEX idx_audit_request_id ON audit_logs(request_id);

-- Add request_id to transactions (nullable — manual credits don't have one)
ALTER TABLE transactions ADD COLUMN request_id TEXT;
CREATE INDEX idx_transactions_request_id ON transactions(request_id);
```

- [ ] **Step 2: Update build.rs to include the new migration**

In `crates/storage/build.rs`, add a new `rerun-if-changed` line for the migration file. Check the existing pattern and add:

```
"migrations/postgres/20260507000000_add_request_id.sql",
```

- [ ] **Step 3: Commit**

```bash
git add crates/storage/migrations/postgres/20260507000000_add_request_id.sql crates/storage/build.rs
git commit -m "feat: add request_id column migration to usage_records, audit_logs, transactions"
```

---

### Task 2: Update Rust types

**Files:**
- Modify: `crates/storage/src/types.rs`
- Modify: `crates/nats-publisher/src/lib.rs`
- Modify: `crates/api/src/lib.rs`

- [ ] **Step 1: Add `request_id` to `UsageRecord`**

In `crates/storage/src/types.rs`, add `request_id: String` to the `UsageRecord` struct (after the `id` field):

```rust
pub struct UsageRecord {
    pub id: String,
    pub request_id: String,
    pub key_id: String,
    // ... rest unchanged
}
```

- [ ] **Step 2: Add `request_id` to `AuditLog`**

Add `request_id: Option<String>` to the `AuditLog` struct (after the `id` field). It's optional because existing records won't have it:

```rust
pub struct AuditLog {
    pub id: String,
    pub request_id: Option<String>,
    pub key_id: String,
    // ... rest unchanged
}
```

- [ ] **Step 3: Add `request_id` to `Transaction`**

Add `request_id: Option<String>` to the `Transaction` struct (after the `reference_id` field):

```rust
pub struct Transaction {
    pub id: String,
    pub account_id: String,
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub amount: i64,
    pub balance_after: i64,
    pub description: Option<String>,
    pub reference_id: Option<String>,
    pub request_id: Option<String>,
    pub created_at: DateTime<Utc>,
}
```

- [ ] **Step 4: Add `request_id` to `DeductBalance`**

Add `request_id: Option<String>` to the `DeductBalance` struct (after `reference_id`):

```rust
pub struct DeductBalance {
    pub account_id: String,
    pub amount: i64,
    pub transaction_type: TransactionType,
    pub description: Option<String>,
    pub reference_id: Option<String>,
    pub request_id: Option<String>,
}
```

- [ ] **Step 5: Add `request_id` to `UsageEvent`**

In `crates/nats-publisher/src/lib.rs`, add `request_id: String` to the `UsageEvent` struct (after `id`):

```rust
pub struct UsageEvent {
    pub id: String,
    pub request_id: String,
    pub key_id: String,
    // ... rest unchanged
}
```

- [ ] **Step 6: Add `request_id` to `AuditEvent`**

Add `request_id: String` to the `AuditEvent` struct (after `id`):

```rust
pub struct AuditEvent {
    pub id: String,
    pub request_id: String,
    pub key_id: String,
    // ... rest unchanged
}
```

- [ ] **Step 7: Add `request_id` to `AuditTask`**

In `crates/api/src/lib.rs`, add `request_id: String` to the `AuditTask` struct (after the opening brace):

```rust
pub struct AuditTask {
    pub request_id: String,
    pub key_id: String,
    // ... rest unchanged
}
```

- [ ] **Step 8: Commit**

```bash
git add crates/storage/src/types.rs crates/nats-publisher/src/lib.rs crates/api/src/lib.rs
git commit -m "feat: add request_id field to UsageRecord, AuditLog, Transaction, NATS events, AuditTask"
```

---

### Task 3: Update storage layer SQL queries

**Files:**
- Modify: `crates/storage/src/postgres.rs`
- Modify: `crates/storage/src/lib.rs`

- [ ] **Step 1: Update `record_usage()` SQL**

In `crates/storage/src/postgres.rs`, find the `record_usage()` function. Update the INSERT to include `request_id`:

```sql
INSERT INTO usage_records (id, request_id, key_id, model_name, provider_id, channel_id, protocol, 
    input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens, cost, user_id, created_at)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
```

Add the `request_id` parameter binding in the correct position. The existing code binds parameters in order — insert `&usage.request_id` after `&usage.id`.

- [ ] **Step 2: Update `insert_log()` SQL**

Find the `insert_log()` function. Add `request_id` to the INSERT. It goes after `id` in the column list and a `$N` parameter. Add `&log.request_id` in the correct binding position.

- [ ] **Step 3: Update `deduct_balance()` SQL**

Find the `deduct_balance()` function. Update the transaction INSERT to include `request_id`:

```sql
INSERT INTO transactions (id, account_id, type, amount, balance_after, description, reference_id, request_id, created_at)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
```

Add `&req.request_id` in the correct binding position.

- [ ] **Step 4: Update `get_transactions()` queries**

Find all SELECT queries on the transactions table and add `request_id` to the SELECT list. There are likely queries in:
- Transaction list for an account
- Single transaction lookup
- Transaction by reference_id

The SELECT should now include `request_id` as a column, and the row-mapping closure should map it to the struct field.

- [ ] **Step 5: Update `get_usage_records()` queries**

Find SELECT queries on usage_records and add `request_id` to the column list and row mapping.

- [ ] **Step 6: Update `get_audit_logs()` queries**

Find SELECT queries on audit_logs and add `request_id` to the column list and row mapping.

- [ ] **Step 7: Remove `query_usage_cost_by_user()` from trait and implementation**

In `crates/storage/src/lib.rs`, remove the `query_usage_cost_by_user()` trait method.

In `crates/storage/src/postgres.rs`, remove the `query_usage_cost_by_user()` implementation.

- [ ] **Step 8: Add `get_transaction_by_request_id()` to trait and implementation**

In `crates/storage/src/lib.rs`, add to the Storage trait:

```rust
async fn get_transaction_by_request_id(&self, request_id: &str) -> Result<Option<Transaction>, Box<dyn std::error::Error + Send + Sync>>;
```

In `crates/storage/src/postgres.rs`, implement it:

```rust
async fn get_transaction_by_request_id(&self, request_id: &str) -> Result<Option<Transaction>, Box<dyn std::error::Error + Send + Sync>> {
    let row = sqlx::query_as(
        "SELECT id, account_id, type, amount, balance_after, description, reference_id, request_id, created_at FROM transactions WHERE request_id = $1",
    )
    .bind(request_id)
    .fetch_optional(&self.pool)
    .await
    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    Ok(row)
}
```

Note: Check whether `query_as` is used with a `FromRow` derive or manual row mapping. Follow the existing pattern in the file. If the code uses manual `query()` + `.fetch_optional()` + row.get(), follow that pattern instead.

- [ ] **Step 9: Commit**

```bash
git add crates/storage/src/postgres.rs crates/storage/src/lib.rs
git commit -m "feat: update storage SQL to include request_id, add per-request lookup, remove batch query"
```

---

### Task 4: Generate request_id in proxy and pass to events

**Files:**
- Modify: `crates/api/src/proxy.rs`

- [ ] **Step 1: Generate request_id at the top of `proxy()`**

In the `proxy()` function (line ~644, after the function signature), generate the request_id:

```rust
let request_id = uuid::Uuid::new_v4().to_string();
```

- [ ] **Step 2: Pass request_id to AuditTask at all 3 construction sites**

There are 3 places where `AuditTask` is constructed:

1. **Error response** (~line 1010): Add `request_id: request_id.clone(),`
2. **SSE streaming** — the `SseAuditParams` struct also needs `request_id`. Add `request_id: request_id.clone()` to the `SseAuditParams` construction, then add it to the `AuditTask` construction (~line 532).
3. **Non-streaming success** (~line 1180): Add `request_id: request_id.clone(),`

Also add `request_id: String` to the `SseAuditParams` struct definition.

- [ ] **Step 3: Update `publish_audit_events()` to use request_id from AuditTask**

In `publish_audit_events()`, set the `request_id` on both events from the task:

```rust
let usage_event = UsageEvent {
    id: uuid::Uuid::new_v4().to_string(),
    request_id: task.request_id.clone(),
    // ... rest unchanged
};

let audit_event = AuditEvent {
    id: uuid::Uuid::new_v4().to_string(),
    request_id: task.request_id.clone(),
    // ... rest unchanged
};
```

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/proxy.rs
git commit -m "feat: generate request_id per request, pass through AuditTask and NATS events"
```

---

### Task 5: Update usage worker for per-request deduction

**Files:**
- Modify: `crates/usage-worker/src/main.rs`
- Modify: `crates/usage-worker/Cargo.toml`

- [ ] **Step 1: Add dependencies to usage-worker Cargo.toml**

Add `llm_gateway_storage` dependency if not already present (it should be, for `record_usage`). No new crates needed.

- [ ] **Step 2: Add per-request deduction logic in the usage worker**

In `crates/usage-worker/src/main.rs`, after the existing `storage.record_usage(&record)` call (around line 86), add deduction logic:

```rust
// Record usage
storage.record_usage(&record).await.map_err(|e| {
    tracing::error!("[USAGE] Failed to record usage: {}", e);
    async_nats::AckKind::Nak(None)
})?;

// Per-request deduction
if let Some(ref user_id) = record.user_id {
    if record.cost > 0 {
        match storage.get_account_by_user_id(user_id).await {
            Ok(Some(account)) => {
                // Idempotency: check if already deducted
                match storage.get_transaction_by_request_id(&record.request_id).await {
                    Ok(None) => {
                        // No existing transaction — deduct
                        let req = DeductBalance {
                            account_id: account.id,
                            amount: record.cost,
                            transaction_type: TransactionType::Debit,
                            description: Some(format!("{} - {}", record.model_name, record.request_id)),
                            reference_id: None,
                            request_id: Some(record.request_id.clone()),
                        };
                        match storage.deduct_balance(&req).await {
                            Ok(DeductBalanceResult::Success(_)) => {}
                            Ok(DeductBalanceResult::InsufficientBalance { current_balance, requested }) => {
                                tracing::warn!(
                                    "[USAGE] Insufficient balance for user={}, balance={}, cost={}",
                                    user_id, current_balance, requested
                                );
                            }
                            Ok(DeductBalanceResult::AccountNotFound) => {
                                tracing::warn!("[USAGE] Account not found for user={}", user_id);
                            }
                            Err(e) => {
                                tracing::error!("[USAGE] Deduction failed for request_id={}: {}", record.request_id, e);
                                return Err(async_nats::AckKind::Nak(None));
                            }
                        }
                    }
                    Ok(Some(_)) => {
                        tracing::debug!("[USAGE] Already deducted for request_id={}", record.request_id);
                    }
                    Err(e) => {
                        tracing::error!("[USAGE] Idempotency check failed for request_id={}: {}", record.request_id, e);
                    }
                }
            }
            Ok(None) => {
                tracing::debug!("[USAGE] No account for user={}, skipping deduction", user_id);
            }
            Err(e) => {
                tracing::error!("[USAGE] Failed to lookup account for user={}: {}", user_id, e);
            }
        }
    }
}
```

Add the necessary imports at the top of the file:

```rust
use llm_gateway_storage::{DeductBalance, DeductBalanceResult, TransactionType};
```

- [ ] **Step 3: Add `request_id` to the UsageRecord construction**

In the event → UsageRecord mapping, add:

```rust
request_id: event.request_id,
```

- [ ] **Step 4: Commit**

```bash
git add crates/usage-worker/src/main.rs crates/usage-worker/Cargo.toml
git commit -m "feat: usage worker performs per-request balance deduction with request_id"
```

---

### Task 6: Update audit worker to pass request_id

**Files:**
- Modify: `crates/audit-worker/src/main.rs`

- [ ] **Step 1: Add request_id to the log_request() call**

In `crates/audit-worker/src/main.rs`, the `log_request()` call passes many positional arguments. Add `request_id` to the `AuditLog` construction or to the `log_request()` call.

Check whether the worker constructs an `AuditLog` struct directly or passes individual args to `log_request()`. Either way, add `request_id: Some(event.request_id)` or `request_id: event.request_id` in the appropriate position.

- [ ] **Step 2: Commit**

```bash
git add crates/audit-worker/src/main.rs
git commit -m "feat: audit worker passes request_id to audit_logs"
```

---

### Task 7: Remove batch settlement worker

**Files:**
- Delete: `crates/api/src/settlement.rs`
- Modify: `crates/api/src/lib.rs`
- Modify: `crates/api/Cargo.toml`
- Modify: `crates/gateway/src/main.rs`

- [ ] **Step 1: Delete settlement.rs**

```bash
rm crates/api/src/settlement.rs
```

- [ ] **Step 2: Remove settlement from api/src/lib.rs**

In `crates/api/src/lib.rs`:
- Remove `pub mod settlement;`
- Remove `pub use settlement::{start_settlement_worker, SettlementTrigger};`
- Remove `use tokio::sync::mpsc;`
- Remove `settlement_tx: mpsc::Sender<settlement::SettlementTrigger>,` from `AppState`

- [ ] **Step 3: Remove settlement from gateway/src/main.rs**

In `crates/gateway/src/main.rs`:
- Remove the settlement channel creation: `let (settlement_tx, settlement_rx) = tokio::sync::mpsc::channel::<llm_gateway_api::SettlementTrigger>(1);`
- Remove the settlement worker spawn: `tokio::spawn(llm_gateway_api::start_settlement_worker(...));`
- Remove `settlement_tx` from the `AppState` construction

- [ ] **Step 4: Remove settlement_tx from AppState consumers**

Check if `state.settlement_tx` is referenced anywhere else in the codebase. It should not be — the exploration confirmed it was never used. If there are references, remove them.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: remove batch settlement worker, switch to per-request deduction"
```

---

### Task 8: Add API endpoints for request_id lookup

**Files:**
- Modify: `crates/api/src/management.rs` (or wherever transaction endpoints are)
- Modify: `crates/storage/src/lib.rs`
- Modify: `crates/storage/src/postgres.rs`

- [ ] **Step 1: Add storage query method for usage by request_id**

In `crates/storage/src/lib.rs`, add to the Storage trait:

```rust
async fn get_usage_by_request_id(&self, request_id: &str) -> Result<Option<UsageRecord>, Box<dyn std::error::Error + Send + Sync>>;

async fn get_audit_by_request_id(&self, request_id: &str) -> Result<Option<AuditLog>, Box<dyn std::error::Error + Send + Sync>>;
```

In `crates/storage/src/postgres.rs`, implement both:

```rust
async fn get_usage_by_request_id(&self, request_id: &str) -> Result<Option<UsageRecord>, Box<dyn std::error::Error + Send + Sync>> {
    // SELECT * FROM usage_records WHERE request_id = $1
    // Follow existing query pattern for usage_records
}

async fn get_audit_by_request_id(&self, request_id: &str) -> Result<Option<AuditLog>, Box<dyn std::error::Error + Send + Sync>> {
    // SELECT * FROM audit_logs WHERE request_id = $1
    // Follow existing query pattern for audit_logs
}
```

Follow the existing row mapping pattern in the file (either `FromRow` derive or manual `.get()` calls).

- [ ] **Step 2: Add management API endpoint**

Find where transaction-related endpoints are defined in `crates/api/src/management.rs`. Add a new endpoint:

```rust
pub async fn get_request_details(
    State(state): State<Arc<AppState>>,
    Path(request_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let transaction = state.storage.get_transaction_by_request_id(&request_id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let usage = state.storage.get_usage_by_request_id(&request_id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let audit = state.storage.get_audit_by_request_id(&request_id).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "transaction": transaction,
        "usage": usage,
        "audit": audit,
    })))
}
```

Register the route in the router (find where other admin routes are defined):

```rust
.route("/admin/requests/:request_id", get(get_request_details))
```

- [ ] **Step 3: Commit**

```bash
git add crates/api/src/management.rs crates/storage/src/lib.rs crates/storage/src/postgres.rs
git commit -m "feat: add request_id lookup API endpoint and storage queries"
```

---

### Task 9: Update frontend for transaction drill-down

**Files:**
- Modify: `web/src/api/client.ts` (or relevant API file)
- Modify: `web/src/pages/Account.tsx` or `web/src/pages/AccountBalance.tsx`
- Modify: `web/src/hooks/useAccounts.ts`
- Modify: `web/src/i18n/en.json`
- Modify: `web/src/i18n/zh.json`

- [ ] **Step 1: Add API client function for request details**

In the API client file, add:

```typescript
export async function getRequestDetails(requestId: string) {
  const { data } = await apiClient.get(`/admin/requests/${requestId}`);
  return data;
}
```

- [ ] **Step 2: Add React Query hook**

In `web/src/hooks/useAccounts.ts`, add:

```typescript
export function useRequestDetails(requestId: string) {
  return useQuery({
    queryKey: ['request-details', requestId],
    queryFn: () => getRequestDetails(requestId),
    enabled: !!requestId,
  });
}
```

- [ ] **Step 3: Add drill-down UI on transaction page**

On the Account/AccountBalance page where transactions are listed, make debit transactions clickable. When clicked, open a drawer/modal showing:
- Transaction details (amount, balance after, time)
- Usage record (model, tokens, cost)
- Link to audit log

Add the request_id column to the transaction table. For debit rows, make the request_id a clickable link that opens the detail drawer.

- [ ] **Step 4: Add i18n keys**

Add translation keys for the new UI elements (drawer title, labels for usage details, token labels, etc.) to both en.json and zh.json.

- [ ] **Step 5: Commit**

```bash
git add web/src/
git commit -m "feat: frontend transaction drill-down with request_id"
```

---

### Task 10: Final verification and cleanup

**Files:**
- Review all modified files

- [ ] **Step 1: Verify build**

```bash
cargo build --workspace
```

- [ ] **Step 2: Verify tests**

```bash
cargo test --workspace
```

Note: Integration tests require a PostgreSQL database (set `DATABASE_URL` environment variable). Unit tests should pass without it.

- [ ] **Step 3: Verify frontend build**

```bash
cd web && npm run build
```

- [ ] **Step 4: Check for remaining references to settlement**

```bash
grep -rn "settlement\|SettlementTrigger\|query_usage_cost_by_user\|last_settlement" crates/ --include="*.rs"
```

Expected: no results (all removed).

- [ ] **Step 5: Commit any fixes**

```bash
git add -A
git commit -m "fix: final cleanup for per-request deduction"
```
