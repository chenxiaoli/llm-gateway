# user_id Denormalization Design

## Goal

Add `user_id` column to `usage_records` and `audit_logs` tables so queries are filtered directly by user instead of joining through `api_keys.created_by`. Eliminates the key-lookup workaround and settlement N+1.

## Background

Currently, neither `usage_records` nor `audit_logs` stores the requesting user's ID. The only link is `key_id -> api_keys.created_by`. This forces every user-scoped query to:
1. Fetch all keys for the user (`list_keys`)
2. Pass key IDs as `IN (...)` to the usage query
3. The settlement worker does an N+1 reverse lookup per record

The `user_id` is available at insert time — the proxy handler already loads the full `ApiKey` (including `created_by`) during authentication.

## Schema Changes

Add nullable `user_id TEXT` column to both tables. Nullable because keys created without a user (seed data, admin tools) have no `created_by`.

### Migration (SQLite + PostgreSQL)

```sql
ALTER TABLE usage_records ADD COLUMN user_id TEXT;
ALTER TABLE audit_logs ADD COLUMN user_id TEXT;

UPDATE usage_records SET user_id = (
  SELECT created_by FROM api_keys WHERE api_keys.id = usage_records.key_id
);
UPDATE audit_logs SET user_id = (
  SELECT created_by FROM api_keys WHERE api_keys.id = audit_logs.key_id
);
```

No foreign key constraint — consistent with the existing `created_by` soft reference pattern.

## Write Path

### AuditTask struct (`crates/api/src/lib.rs`)

Add field: `user_id: Option<String>`

### Proxy handler (`crates/api/src/proxy.rs`)

Where `AuditTask` is constructed, set `user_id: api_key.created_by.clone()`. The `api_key` variable already holds the full `ApiKey` with `created_by` populated.

### Audit worker (`crates/api/src/workers.rs`)

Pass `task.user_id` into `UsageRecord` and `AuditLog` when constructing them.

### Rust structs (`crates/storage/src/types.rs`)

Add `user_id: Option<String>` to `UsageRecord` and `AuditLog`.

### Storage INSERT statements (`crates/storage/src/sqlite.rs`, `postgres.rs`)

Include `user_id` in all INSERT statements for `usage_records` and `audit_logs`.

## Query Path Simplification

### UsageFilter (`crates/storage/src/types.rs`)

- Add `user_id: Option<String>` field (server-side only, `#[serde(skip)]`)
- Remove `key_ids: Option<Vec<String>>` field (the workaround we just added)
- Keep `key_id: Option<String>` for single-key filtering from the frontend dropdown

### Storage WHERE clauses (SQLite + PostgreSQL)

Add `AND user_id = ?` / `$N` support to `query_usage`, `query_usage_paginated`, `query_usage_summary`. Remove `key_ids IN (...)` logic.

### Usage API handlers (`crates/api/src/management/usage.rs`)

For non-admin users:
```rust
if claims.role != "admin" {
    filter.user_id = Some(claims.sub);
}
```
Remove `get_user_key_ids()` helper and `apply_user_scope()`. No key lookup needed.

### Audit log handlers (`crates/api/src/management/audit.rs`)

For non-admin users, add `WHERE user_id = ?` filter. Same pattern as usage.

### Settlement worker (`crates/api/src/settlement.rs`)

Replace the N+1 loop with a grouped query:
```sql
SELECT user_id, SUM(cost) as total_cost
FROM usage_records
WHERE user_id IS NOT NULL
  AND created_at >= ? AND created_at < ?
GROUP BY user_id
```
Then one `get_account_by_user_id` per group instead of per record.

### Frontend

No changes. The API contract is the same — just properly scoped.

## Files Modified

| File | Change |
|------|--------|
| `crates/storage/migrations/sqlite/YYYYMMDD_add_user_id.sql` | New migration |
| `crates/storage/migrations/postgres/YYYYMMDD_add_user_id.sql` | New migration |
| `crates/storage/src/types.rs` | Add `user_id` to structs and filter, remove `key_ids` |
| `crates/storage/src/sqlite.rs` | Update INSERT/WHERE for `user_id`, remove `key_ids` IN |
| `crates/storage/src/postgres.rs` | Same |
| `crates/api/src/lib.rs` | Add `user_id` to `AuditTask` |
| `crates/api/src/proxy.rs` | Set `user_id` from `api_key.created_by` |
| `crates/api/src/workers.rs` | Pass `user_id` into record construction |
| `crates/api/src/management/usage.rs` | Use `filter.user_id` instead of key lookup |
| `crates/api/src/management/audit.rs` | Add `user_id` filter for non-admin |
| `crates/api/src/settlement.rs` | Replace N+1 with grouped query |
