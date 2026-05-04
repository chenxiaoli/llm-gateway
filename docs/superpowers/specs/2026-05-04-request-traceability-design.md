# Per-Request Deduction & request_id Traceability — Design Spec

**Date:** 2026-05-04
**Status:** Approved

## Goal

Replace batch settlement with per-request balance deduction. Add a shared `request_id` across `usage_records`, `audit_logs`, and `transactions` for 1:1:1 traceability. Users can drill from any transaction to its usage record and audit log.

## Current State

- Gateway publishes `UsageEvent` and `AuditEvent` to NATS per request
- Usage worker inserts `usage_records` (no balance deduction)
- Audit worker inserts `audit_logs`
- Settlement worker (60s interval) aggregates `SUM(cost) GROUP BY user_id` and creates batch debit transactions
- No link between usage_records, audit_logs, or transactions — only time-window correlation via `user_id` and `created_at`

## New Architecture

### request_id

Gateway generates a UUID `request_id` when each LLM request arrives. This ID flows through:

1. `UsageEvent` → `usage_records.request_id`
2. `AuditEvent` → `audit_logs.request_id`
3. Usage worker creates transaction → `transactions.request_id`

All three tables share the same `request_id`, creating a 1:1:1 chain.

### Per-Request Deduction

The usage worker now deducts balance for each usage event:

```
For each UsageEvent:
  1. INSERT INTO usage_records (with request_id)
  2. Look up account by user_id
  3. If account exists:
     a. BEGIN transaction
     b. SELECT accounts WHERE id = $1 FOR UPDATE
     c. If balance >= cost: UPDATE accounts SET balance = balance - cost
     d. INSERT INTO transactions (request_id, type='debit', amount=cost)
     e. COMMIT
  4. If account not found or balance insufficient: skip deduction (usage still recorded)
```

Deduction is async (in the usage worker, not the API request path) so request latency is unaffected.

### What's Removed

- Batch settlement worker (`crates/api/src/settlement.rs`) — deleted
- `last_settlement_time` setting — no longer needed
- `reference_id` on transactions — replaced by `request_id` for idempotency
- Settlement spawning in gateway main — removed

## Database Changes

### Migration

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

### Rust Type Changes

- `UsageRecord`: add `request_id: String`
- `UsageEvent` (NATS): add `request_id: String`
- `AuditEvent` (NATS): add `request_id: String`
- `Transaction`: add `request_id: Option<String>`
- `DeductBalance`: replace `reference_id` with `request_id`

## Backend Changes

### crates/api/src/proxy.rs

- Generate `request_id` (UUID) at request entry
- Pass to `publish_audit_events()` → included in both UsageEvent and AuditEvent

### crates/nats-publisher/src/lib.rs

- Add `request_id` to `UsageEvent` and `AuditEvent` structs

### crates/usage-worker/src/main.rs

- After inserting usage_record, call `storage.deduct_balance()` with `request_id`
- Dedup: check if transaction with this `request_id` already exists before deducting

### crates/api/src/settlement.rs

- Delete this file entirely

### crates/gateway/src/main.rs

- Remove settlement worker spawning and interval

### crates/storage/

- `record_usage()`: include `request_id` in INSERT
- `create_audit_log()`: include `request_id` in INSERT
- `deduct_balance()`: use `request_id` instead of `reference_id` for idempotency
- New method: `get_transaction_by_request_id(request_id) -> Option<Transaction>`
- New method: `get_audit_by_request_id(request_id) -> Option<AuditLog>`
- Remove: `query_usage_cost_by_user()` (batch aggregation, no longer needed)

### crates/api/src/workers.rs

- Remove `parse_usage()` and `calculate_cost()` if they were only used by settlement. (Check — they may also be used by the proxy for constructing UsageEvent. If so, keep them.)

## API Changes

### New endpoint

`GET /api/v1/admin/transactions/:request_id` — returns the transaction, usage record, and audit log for a given request_id. Or alternatively, existing endpoints gain `?request_id=` query parameter.

### Removed

- Settlement-related endpoint or settings (if any)

## Frontend Changes

### Account / Transaction page

- Debit transaction rows show `request_id`
- Click a debit transaction → drawer/modal shows:
  - Usage record details: model, tokens (input/output/cache), cost, latency
  - Link to audit log entry (full request/response details)

### Audit Logs page

- Each log entry shows `request_id`
- Click to see the associated transaction (charge amount, balance after)

### No new pages

This is a drill-down enhancement to existing pages.

## Not Changed

- NATS JetStream infrastructure (same streams, same consumers)
- Audit worker (just adds request_id to INSERT)
- Account creation / manual credit/debit flows (transactions without request_id)
- Pricing calculation logic
- API request/response format for LLM proxy endpoints

## Idempotency

Per-request deduction uses `request_id` as the natural idempotency key. If the usage worker processes the same event twice (NATS redelivery):

1. Check if `transactions.request_id` already exists for this account
2. If yes, skip deduction (already processed)
3. Usage record INSERT uses `ON CONFLICT (id) DO NOTHING` (existing behavior)

## Error Handling

- **Account not found**: Usage is still recorded, no deduction. Log a warning.
- **Insufficient balance**: Usage is still recorded, no deduction. This allows post-pay or unbounded accounts. Optionally, could emit a low-balance alert.
- **Deduction DB error**: Usage is recorded. NATS nack causes redelivery. On retry, idempotency check prevents double-deduction.
