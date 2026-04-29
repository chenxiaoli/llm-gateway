# Monetary Integer Subunits Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert all monetary values from `f64` floating point to `i64` integer subunits (1 USD = 100,000,000 units, 8 decimal places) for exact arithmetic, with API boundary conversion so the frontend is unchanged.

**Architecture:** A new `money` module provides conversion helpers. All storage types change from `f64` to `i64`. Database row structs in both SQLite and PostgreSQL change to `i64`. Billing calculations switch to integer arithmetic. API handlers convert between decimal USD (JSON) and subunits (internal) at the request/response boundary using serde custom serialization or manual conversion. New migrations convert existing data.

**Tech Stack:** Rust, sqlx, PostgreSQL, SQLite, Axum, serde

**Spec:** `docs/superpowers/specs/2026-04-29-monetary-integer-microdollars-design.md`

---

## File Structure

| Action | File | Responsibility |
|--------|------|---------------|
| Create | `crates/storage/src/money.rs` | Conversion helpers: `usd_to_units`, `units_to_usd`, `ratio_to_bps`, `bps_to_ratio` + serde helpers |
| Modify | `crates/storage/src/types.rs` | All monetary fields `f64` → `i64`, pricing config fields `Option<f64>` → `Option<i64>` |
| Modify | `crates/storage/src/lib.rs` | `query_usage_cost_by_user` return type `Vec<(String, f64)>` → `Vec<(String, i64)>` |
| Modify | `crates/storage/src/postgres.rs` | All `PgXxxRow` structs `f64` → `i64`, SQL queries unchanged (columns change via migration) |
| Modify | `crates/storage/src/sqlite.rs` | All `SqliteXxxRow` structs `f64` → `i64`, SQL queries unchanged |
| Modify | `crates/billing/src/lib.rs` | `CostCalculation.cost` → `i64`, `calculate_cost` / `calculate_cost_with_cache` signatures |
| Modify | `crates/billing/src/pricing.rs` | `calculate_cost` → `i64`, all per-billing-type methods → integer arithmetic |
| Modify | `crates/api/src/workers.rs` | Cost computation uses `i64`, markup multiplication uses basis points |
| Modify | `crates/api/src/settlement.rs` | `HashMap<String, f64>` → `HashMap<String, i64>`, balance deduction logic |
| Modify | `crates/api/src/proxy.rs` | `ChannelModelEnriched.markup_ratio` → `i64`, balance check, `AuditTask.markup_ratio` → `i64` |
| Modify | `crates/api/src/lib.rs` | `AuditTask.markup_ratio` → `i64` |
| Modify | `crates/api/src/management/keys.rs` | Response struct `budget_monthly` conversion |
| Modify | `crates/api/src/management/channels.rs` | `ChannelWithModels.markup_ratio` / `.balance` conversion |
| Modify | `crates/api/src/management/users.rs` | `UserResponse.balance` / `.threshold` conversion |
| Modify | `crates/api/src/management/accounts.rs` | `AccountResponse`, amount validation, threshold update |
| Modify | `crates/api/src/management/channel_models.rs` | `markup_ratio` default and conversion |
| Create | `crates/storage/migrations/postgres/20260502000000_monetary_integer_subunits.sql` | ALTER all monetary columns to BIGINT |
| Create | `crates/storage/migrations/sqlite/20260502000000_monetary_integer_subunits.sql` | Recreate tables with INTEGER columns |

**No frontend changes required** — API continues to send/receive decimal floats.

---

## Key Constants

```rust
// In crates/storage/src/money.rs
pub const UNITS_PER_USD: i64 = 100_000_000; // 8 decimal places
pub const BPS_FACTOR: i64 = 10_000;          // basis points for markup_ratio
```

---

### Task 1: Money conversion module

**Files:**
- Create: `crates/storage/src/money.rs`
- Modify: `crates/storage/src/lib.rs` (add `mod money; pub use money::*;`)

- [ ] **Step 1: Create `crates/storage/src/money.rs`**

```rust
pub const UNITS_PER_USD: i64 = 100_000_000; // 8 decimal places
pub const BPS_FACTOR: i64 = 10_000;          // markup_ratio as basis points

pub fn usd_to_units(usd: f64) -> i64 {
    (usd * UNITS_PER_USD as f64).round() as i64
}

pub fn units_to_usd(units: i64) -> f64 {
    units as f64 / UNITS_PER_USD as f64
}

pub fn ratio_to_bps(ratio: f64) -> i64 {
    (ratio * BPS_FACTOR as f64).round() as i64
}

pub fn bps_to_ratio(bps: i64) -> f64 {
    bps as f64 / BPS_FACTOR as f64
}

pub fn opt_usd_to_units(usd: Option<f64>) -> Option<i64> {
    usd.map(usd_to_units)
}

pub fn opt_units_to_usd(units: Option<i64>) -> Option<f64> {
    units.map(units_to_usd)
}
```

- [ ] **Step 2: Register module in `crates/storage/src/lib.rs`**

Add after existing `mod` declarations:
```rust
pub mod money;
pub use money::*;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build -p llm-gateway-storage`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/storage/src/money.rs crates/storage/src/lib.rs
git commit -m "feat: add money conversion module for integer subunits"
```

---

### Task 2: Storage types — core monetary fields

**Files:**
- Modify: `crates/storage/src/types.rs`

- [ ] **Step 1: Change all core monetary fields from `f64` to `i64` in `types.rs`**

The following fields change type:

```
ApiKey.budget_monthly:               Option<f64> → Option<i64>
CreateApiKey.budget_monthly:         Option<f64> → Option<i64>
UpdateApiKey.budget_monthly:         Option<Option<f64>> → Option<Option<i64>>
Channel.markup_ratio:                f64 → i64
Channel.balance:                     Option<f64> → Option<i64>
CreateChannel.markup_ratio:          Option<f64> → Option<i64>
CreateChannel.balance:               Option<f64> → Option<i64>
UpdateChannel.markup_ratio:          Option<f64> → Option<i64>
UpdateChannel.balance:               Option<Option<f64>> → Option<Option<i64>>
ChannelModel.markup_ratio:           f64 → i64
CreateChannelModel.markup_ratio:     Option<f64> → Option<i64>
UpdateChannelModel.markup_ratio:     Option<f64> → Option<i64>
UsageRecord.cost:                    f64 → i64
UsageSummaryRecord.total_cost:       f64 → i64
UserWithBalance.balance:             f64 → i64
UserWithBalance.threshold:           f64 → i64
Account.balance:                     f64 → i64
Account.threshold:                   f64 → i64
Transaction.amount:                  f64 → i64
Transaction.balance_after:           f64 → i64
CreateTransaction.amount:            f64 → i64
UpdateAccountThreshold.threshold:    f64 → i64
DeductBalance.amount:                f64 → i64
DeductBalanceResult::InsufficientBalance.current_balance: f64 → i64
DeductBalanceResult::InsufficientBalance.requested:       f64 → i64
AddBalance.amount:                   f64 → i64
AccountResponse.balance:             f64 → i64
AccountResponse.threshold:           f64 → i64
TransactionResponse.amount:          f64 → i64
TransactionResponse.balance_after:   f64 → i64
```

- [ ] **Step 2: Change pricing config fields from `Option<f64>` to `Option<i64>`**

```
PerTokenConfig.input_price_1m, output_price_1m, cache_read_price_1m, cache_creation_price_1m: Option<f64> → Option<i64>
PerRequestConfig.request_price: Option<f64> → Option<i64>
PerCharacterConfig.input_price_1m, output_price_1m: Option<f64> → Option<i64>
TierConfig.input_price_1m, output_price_1m: Option<f64> → Option<i64>
HybridConfig.base_per_call, input_price_1m, output_price_1m, cache_read_price_1m, cache_creation_price_1m: Option<f64> → Option<i64>
```

Also update the accessor methods (e.g., `input_price()`, `output_price()`, `cache_read_price()`, `cache_creation_price()`, `price_per_call()`) to return `i64` with default `0` instead of `0.0`.

Update `divisor()` methods: `PerTokenConfig::divisor()` → `1_000_000i64`, `PerCharacterConfig::divisor()` → `1_000_000i64`, `TierConfig::divisor()` → `1_000_000i64` or `1_000i64`, `HybridConfig::divisor()` → `1_000_000i64`, `TieredTokenConfig::tier_divisor()` → `1_000_000i64`.

- [ ] **Step 3: Verify compilation**

Run: `cargo build -p llm-gateway-storage`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/storage/src/types.rs
git commit -m "feat: change storage types monetary fields from f64 to i64"
```

---

### Task 3: Storage trait — return types

**Files:**
- Modify: `crates/storage/src/lib.rs`

- [ ] **Step 1: Update `query_usage_cost_by_user` return type**

Change `Result<Vec<(String, f64)>, ...>` to `Result<Vec<(String, i64)>, ...>`.

- [ ] **Step 2: Verify compilation**

Run: `cargo build -p llm-gateway-storage`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add crates/storage/src/lib.rs
git commit -m "feat: update storage trait return types for i64 monetary values"
```

---

### Task 4: PostgreSQL row structs and queries

**Files:**
- Modify: `crates/storage/src/postgres.rs`

- [ ] **Step 1: Change all PgXxxRow struct monetary fields from `f64` to `i64`**

```
PgKeyRow.budget_monthly:                 Option<f64> → Option<i64>
PgUsageRow.cost:                         f64 → i64
PgUsageSummaryRow.total_cost:            f64 → i64
PgChannelRow.markup_ratio:               f64 → i64
PgChannelRow.balance:                    Option<f64> → Option<i64>
PgUserWithBalanceRow.balance:            Option<f64> → Option<i64>
PgUserWithBalanceRow.threshold:          Option<f64> → Option<i64>
PgChannelModelRow.markup_ratio:          f64 → i64
PgAccountRow.balance:                    f64 → i64
PgAccountRow.threshold:                  f64 → i64
PgTransactionRow.amount:                 f64 → i64
PgTransactionRow.balance_after:          f64 → i64
```

- [ ] **Step 2: Update `From<PgXxxRow>` implementations**

All `From` impls now pass `i64` directly — no conversion needed since both sides are `i64`. Remove any `.unwrap_or(0.0)` → `.unwrap_or(0)`. Update `UserWithBalance { balance: r.balance.unwrap_or(0.0), threshold: r.threshold.unwrap_or(1.0) }` → `{ balance: r.balance.unwrap_or(0), threshold: r.threshold.unwrap_or(100_000_000) }` (1.0 USD = 100,000,000 subunits).

- [ ] **Step 3: Update SQL queries that reference monetary columns**

SQL queries do NOT change — the column names remain the same. The column types will change via migration. The `.bind()` calls now bind `i64` instead of `f64`, which sqlx handles natively for both `BIGINT` (PostgreSQL) and `INTEGER` (SQLite).

For queries that use `COALESCE(SUM(cost), 0.0)`, change to `COALESCE(SUM(cost), 0)`.

For `deduct_balance` and `add_balance`: change `Option<(f64,)>` → `Option<(i64,)>`.

For `query_usage_cost_by_user`: change return type `Vec<(String, f64)>` → `Vec<(String, i64)>`.

- [ ] **Step 4: Verify compilation**

Run: `cargo build -p llm-gateway-storage`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/storage/src/postgres.rs
git commit -m "feat: update postgres row structs to i64 for monetary values"
```

---

### Task 5: SQLite row structs and queries

**Files:**
- Modify: `crates/storage/src/sqlite.rs`

- [ ] **Step 1: Change all SqliteXxxRow struct monetary fields from `f64` to `i64`**

```
SqliteKeyRow.budget_monthly:             Option<f64> → Option<i64>
SqliteUsageRow.cost:                     f64 → i64
SqliteUsageSummaryRow.total_cost:        f64 → i64
SqliteChannelRow.markup_ratio:           f64 → i64
SqliteChannelRow.balance:                Option<f64> → Option<i64>
SqliteUserWithBalanceRow.balance:        Option<f64> → Option<i64>
SqliteUserWithBalanceRow.threshold:      Option<f64> → Option<i64>
SqliteChannelModelRow.markup_ratio:      f64 → i64
SqliteAccountRow.balance:                f64 → i64
SqliteAccountRow.threshold:              f64 → i64
SqliteTransactionRow.amount:             f64 → i64
SqliteTransactionRow.balance_after:      f64 → i64
```

- [ ] **Step 2: Update `From<SqliteXxxRow>` implementations**

Same pattern as PostgreSQL. Update `unwrap_or` defaults: `0.0` → `0`, `1.0` → `100_000_000`.

- [ ] **Step 3: Update SQL queries**

Same changes as PostgreSQL task: `COALESCE(SUM(cost), 0.0)` → `COALESCE(SUM(cost), 0)`, `Option<(f64,)>` → `Option<(i64,)>`, `Vec<(String, f64)>` → `Vec<(String, i64)>`.

- [ ] **Step 4: Verify compilation**

Run: `cargo build -p llm-gateway-storage`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/storage/src/sqlite.rs
git commit -m "feat: update sqlite row structs to i64 for monetary values"
```

---

### Task 6: Billing calculations — integer arithmetic

**Files:**
- Modify: `crates/billing/src/lib.rs`
- Modify: `crates/billing/src/pricing.rs`

- [ ] **Step 1: Update `CostCalculation` in `lib.rs`**

```rust
pub struct CostCalculation {
    pub cost: i64,  // in subunits
}
```

- [ ] **Step 2: Update `calculate_cost` and `calculate_cost_with_cache` signatures**

These legacy functions take `f64` prices and return `CostCalculation`. Since the pricing configs are now `i64`, update signatures:

```rust
pub fn calculate_cost(input_tokens: i64, output_tokens: i64, input_price: i64, output_price: i64, request_price: i64) -> CostCalculation {
    let input_cost = if input_tokens > 0 && input_price > 0 {
        (input_tokens * input_price) / 1_000_000
    } else { 0 };
    let output_cost = if output_tokens > 0 && output_price > 0 {
        (output_tokens * output_price) / 1_000_000
    } else { 0 };
    let request_cost = request_price; // already per-request in subunits
    CostCalculation { cost: input_cost + output_cost + request_cost }
}
```

Update `calculate_cost_with_cache` similarly, adding `cache_read_price: i64` and `cache_creation_price: i64` parameters.

- [ ] **Step 3: Update `PricingCalculator` in `pricing.rs`**

Change return type from `f64` to `i64`. Each calculation method uses integer arithmetic:

```rust
fn calculate_per_token(&self, cfg: &PerTokenConfig, input_tokens: i64, output_tokens: i64, cache_read_tokens: Option<i64>, cache_creation_tokens: Option<i64>, request_count: i64) -> i64 {
    let div: i64 = 1_000_000;
    let mut cost: i64 = 0;
    if input_tokens > 0 {
        cost += (input_tokens * cfg.input_price()) / div;
    }
    if output_tokens > 0 {
        cost += (output_tokens * cfg.output_price()) / div;
    }
    if let Some(t) = cache_read_tokens {
        if t > 0 {
            cost += (t * cfg.cache_read_price()) / div;
        }
    }
    if let Some(t) = cache_creation_tokens {
        if t > 0 {
            cost += (t * cfg.cache_creation_price()) / div;
        }
    }
    cost
}
```

Apply same pattern to `calculate_per_request`, `calculate_per_character`, `calculate_tiered_token`, `calculate_hybrid`.

- [ ] **Step 4: Verify compilation**

Run: `cargo build -p llm-gateway-billing`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/billing/src/lib.rs crates/billing/src/pricing.rs
git commit -m "feat: billing calculations use integer arithmetic"
```

---

### Task 7: Workers and settlement — i64 cost propagation

**Files:**
- Modify: `crates/api/src/workers.rs`
- Modify: `crates/api/src/settlement.rs`
- Modify: `crates/api/src/proxy.rs`
- Modify: `crates/api/src/lib.rs`

- [ ] **Step 1: Update `AuditTask` in `lib.rs`**

Change `markup_ratio: f64` → `markup_ratio: i64` (stored as basis points).

- [ ] **Step 2: Update `workers.rs`**

At the cost calculation site (~line 119):
```rust
let raw_cost = PricingCalculator.calculate_cost(&policy, &usage);
// Apply markup: raw_cost * markup_bps / BPS_FACTOR
let final_cost = raw_cost * task.markup_ratio / 10_000;
```

Default fallback cost: `0` (was `0.0`).

`UsageRecord` construction: `cost: final_cost` (now `i64`).

- [ ] **Step 3: Update `settlement.rs`**

```rust
let mut account_charges: HashMap<String, i64> = HashMap::new();
// ...
*entry.or_insert(0) += cost;
// ...
if *total_cost <= 0 {
// ...
DeductBalance { user_id, amount: *total_cost }
```

Log formatting: `{:.6}` → just `{}` since it's now an integer.

- [ ] **Step 4: Update `proxy.rs`**

- `ChannelModelEnriched.markup_ratio`: `f64` → `i64`
- `SseAuditParams.markup_ratio`: `f64` → `i64`
- Construction of `ChannelModelEnriched`: `markup_ratio: cm.markup_ratio` (direct, already `i64`)
- Balance check: `account.threshold > 0 && account.balance < account.threshold` (both `i64`)
- All `AuditTask` construction: `markup_ratio: ...` passes `i64` directly

- [ ] **Step 5: Verify compilation**

Run: `cargo build -p llm-gateway-api`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/workers.rs crates/api/src/settlement.rs crates/api/src/proxy.rs crates/api/src/lib.rs
git commit -m "feat: workers, settlement, proxy use i64 monetary values"
```

---

### Task 8: Management handlers — API boundary conversion

**Files:**
- Modify: `crates/api/src/management/keys.rs`
- Modify: `crates/api/src/management/channels.rs`
- Modify: `crates/api/src/management/users.rs`
- Modify: `crates/api/src/management/accounts.rs`
- Modify: `crates/api/src/management/channel_models.rs`

The API continues to accept/return decimal USD values. Handlers convert at the boundary.

- [ ] **Step 1: Update `keys.rs`**

`CreateKeyResponse.budget_monthly` → stays `Option<f64>` for JSON output. Convert: `budget_monthly: created.budget_monthly.map(|u| units_to_usd(u))`.

Input `CreateApiKey` comes from JSON with `budget_monthly: Option<f64>`. In the handler, convert: `budget_monthly: input.budget_monthly.map(usd_to_units)`.

Similarly for `UpdateApiKey.budget_monthly`.

- [ ] **Step 2: Update `channels.rs`**

Response structs keep `f64` for JSON:
- `ChannelModelInfo.markup_ratio: f64` → convert: `bps_to_ratio(cm.markup_ratio)`
- `ChannelWithModels.markup_ratio: f64` → convert: `bps_to_ratio(c.markup_ratio)`
- `ChannelWithModels.balance: Option<f64>` → convert: `c.balance.map(units_to_usd)`

Input conversion:
- `CreateChannel`: `markup_ratio: input.markup_ratio.map(ratio_to_bps).unwrap_or(10000)`, `balance: input.balance.map(usd_to_units)`
- `UpdateChannel`: same pattern

- [ ] **Step 3: Update `users.rs`**

`UserResponse.balance: f64`, `UserResponse.threshold: f64` stay as `f64` for JSON.

Convert in `From<UserWithBalance>`: `balance: units_to_usd(u.balance)`, `threshold: units_to_usd(u.threshold)`.
Convert in `From<&User>` (no account): `balance: 0.0`, `threshold: units_to_usd(100_000_000)` (= 1.0).

- [ ] **Step 4: Update `accounts.rs`**

- `AccountResponse.balance` / `.threshold`: convert `units_to_usd()`
- `TransactionResponse.amount` / `.balance_after`: convert `units_to_usd()`
- Input `amount` from JSON: `usd_to_units(input.amount)`
- Input `threshold` from JSON: `usd_to_units(input.threshold)`
- Validation: `input.amount <= 0` (after conversion, `<= 0`), `input.threshold < 0`
- `DeductBalanceResult::InsufficientBalance` display: convert back for error message

- [ ] **Step 5: Update `channel_models.rs`**

- `CreateChannelModelRequest.markup_ratio`: `input.markup_ratio.map(ratio_to_bps).unwrap_or(10000)`
- `UpdateChannelModelRequest.markup_ratio`: same pattern

- [ ] **Step 6: Verify compilation**

Run: `cargo build -p llm-gateway-api`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add crates/api/src/management/
git commit -m "feat: management handlers convert USD↔subunits at API boundary"
```

---

### Task 9: Database migrations — PostgreSQL

**Files:**
- Create: `crates/storage/migrations/postgres/20260502000000_monetary_integer_subunits.sql`

- [ ] **Step 1: Write migration**

```sql
-- Convert monetary columns from REAL/DOUBLE PRECISION to BIGINT (integer subunits)
-- 1 USD = 100,000,000 subunits (8 decimal places)
-- markup_ratio: 1.0 = 10,000 basis points

ALTER TABLE api_keys ALTER COLUMN budget_monthly TYPE BIGINT USING ROUND(COALESCE(budget_monthly, 0) * 100000000);
ALTER TABLE usage_records ALTER COLUMN cost TYPE BIGINT USING ROUND(cost * 100000000);
ALTER TABLE channels ALTER COLUMN markup_ratio TYPE BIGINT USING ROUND(markup_ratio * 10000);
ALTER TABLE channels ALTER COLUMN balance TYPE BIGINT USING ROUND(COALESCE(balance, 0) * 100000000);
ALTER TABLE channel_models ALTER COLUMN markup_ratio TYPE BIGINT USING ROUND(markup_ratio * 10000);
ALTER TABLE accounts ALTER COLUMN balance TYPE BIGINT USING ROUND(balance * 100000000);
ALTER TABLE accounts ALTER COLUMN threshold TYPE BIGINT USING ROUND(threshold * 100000000);
ALTER TABLE transactions ALTER COLUMN amount TYPE BIGINT USING ROUND(amount * 100000000);
ALTER TABLE transactions ALTER COLUMN balance_after TYPE BIGINT USING ROUND(balance_after * 100000000);
```

- [ ] **Step 2: Commit**

```bash
git add crates/storage/migrations/postgres/20260502000000_monetary_integer_subunits.sql
git commit -m "feat: postgres migration to convert monetary columns to BIGINT"
```

---

### Task 10: Database migrations — SQLite

**Files:**
- Create: `crates/storage/migrations/sqlite/20260502000000_monetary_integer_subunits.sql`

SQLite doesn't support `ALTER COLUMN`. Must recreate affected tables.

- [ ] **Step 1: Write migration**

```sql
-- SQLite: recreate tables with INTEGER monetary columns

-- 1. api_keys: budget_monthly REAL → INTEGER
CREATE TABLE api_keys_new (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    key_hash TEXT NOT NULL UNIQUE,
    rate_limit INTEGER,
    budget_monthly INTEGER,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_by TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    user_id TEXT
);
INSERT INTO api_keys_new SELECT id, name, key_hash, rate_limit, ROUND(COALESCE(budget_monthly, 0) * 100000000), enabled, created_by, created_at, updated_at, user_id FROM api_keys;
DROP TABLE api_keys;
ALTER TABLE api_keys_new RENAME TO api_keys;

-- 2. usage_records: cost REAL → INTEGER
CREATE TABLE usage_records_new (
    id TEXT PRIMARY KEY,
    key_id TEXT NOT NULL,
    model_name TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    channel_id TEXT,
    protocol TEXT NOT NULL,
    input_tokens INTEGER,
    output_tokens INTEGER,
    cost INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    cache_read_tokens INTEGER,
    cache_creation_tokens INTEGER,
    user_id TEXT
);
INSERT INTO usage_records_new SELECT id, key_id, model_name, provider_id, channel_id, protocol, input_tokens, output_tokens, ROUND(cost * 100000000), created_at, cache_read_tokens, cache_creation_tokens, user_id FROM usage_records;
DROP TABLE usage_records;
ALTER TABLE usage_records_new RENAME TO usage_records;
CREATE INDEX idx_usage_key_date ON usage_records(key_id, created_at);
CREATE INDEX idx_usage_model_date ON usage_records(model_name, created_at);

-- 3. channels: markup_ratio and balance REAL → INTEGER
CREATE TABLE channels_new (
    id TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL,
    name TEXT NOT NULL,
    api_key TEXT NOT NULL,
    base_url TEXT,
    priority INTEGER NOT NULL DEFAULT 0,
    markup_ratio INTEGER NOT NULL DEFAULT 10000,
    balance INTEGER,
    enabled INTEGER NOT NULL DEFAULT 1,
    rpm_limit INTEGER,
    tpm_limit INTEGER,
    weight INTEGER DEFAULT 100,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    pricing_policy_id TEXT
);
INSERT INTO channels_new SELECT id, provider_id, name, api_key, base_url, priority, ROUND(markup_ratio * 10000), ROUND(COALESCE(balance, 0) * 100000000), enabled, rpm_limit, tpm_limit, weight, created_at, updated_at, pricing_policy_id FROM channels;
DROP TABLE channels;
ALTER TABLE channels_new RENAME TO channels;

-- 4. channel_models: markup_ratio REAL → INTEGER
CREATE TABLE channel_models_new (
    id TEXT PRIMARY KEY,
    channel_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    upstream_model_name TEXT NOT NULL,
    priority_override INTEGER,
    pricing_policy_id TEXT,
    markup_ratio INTEGER NOT NULL DEFAULT 10000,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(channel_id, model_id)
);
INSERT INTO channel_models_new SELECT id, channel_id, model_id, upstream_model_name, priority_override, pricing_policy_id, ROUND(markup_ratio * 10000), enabled, created_at, updated_at FROM channel_models;
DROP TABLE channel_models;
ALTER TABLE channel_models_new RENAME TO channel_models;

-- 5. accounts: balance and threshold REAL → INTEGER
CREATE TABLE accounts_new (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL UNIQUE,
    balance INTEGER NOT NULL DEFAULT 0,
    threshold INTEGER NOT NULL DEFAULT 100000000,
    currency TEXT NOT NULL DEFAULT 'USD',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
INSERT INTO accounts_new SELECT id, user_id, ROUND(balance * 100000000), ROUND(threshold * 100000000), currency, created_at, updated_at FROM accounts;
DROP TABLE accounts;
ALTER TABLE accounts_new RENAME TO accounts;
CREATE INDEX idx_accounts_user_id ON accounts(user_id);

-- 6. transactions: amount and balance_after REAL → INTEGER
CREATE TABLE transactions_new (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    type TEXT NOT NULL CHECK(type IN ('credit','debit','credit_adjustment','debit_refund')),
    amount INTEGER NOT NULL CHECK(amount > 0),
    balance_after INTEGER NOT NULL,
    description TEXT,
    reference_id TEXT,
    created_at TEXT NOT NULL
);
INSERT INTO transactions_new SELECT id, account_id, type, ROUND(amount * 100000000), ROUND(balance_after * 100000000), description, reference_id, created_at FROM transactions;
DROP TABLE transactions;
ALTER TABLE transactions_new RENAME TO transactions;
CREATE INDEX idx_transactions_account_id ON transactions(account_id);
CREATE INDEX idx_transactions_type ON transactions(type);
CREATE INDEX idx_transactions_reference_id ON transactions(reference_id);
CREATE INDEX idx_transactions_created_at ON transactions(created_at);
```

Note: The exact column list for each table must be verified against the current SQLite schema. The agent should run `PRAGMA table_info(<table>)` before writing this migration to ensure all columns are included.

- [ ] **Step 2: Commit**

```bash
git add crates/storage/migrations/sqlite/20260502000000_monetary_integer_subunits.sql
git commit -m "feat: sqlite migration to convert monetary columns to INTEGER"
```

---

### Task 11: Update existing init migrations for new databases

**Files:**
- Modify: `crates/storage/migrations/postgres/20260415000000_initial.sql`
- Modify: `crates/storage/migrations/postgres/20260426000000_accounts_and_transactions.sql`
- Modify: `crates/storage/migrations/sqlite/20260401000000_init.sql`
- Modify: `crates/storage/migrations/sqlite/20260426000000_accounts_and_transactions.sql`

So that fresh databases (no existing data) use the new types from the start.

- [ ] **Step 1: Update PostgreSQL init migration**

In `20260415000000_initial.sql`:
- `api_keys.budget_monthly REAL` → `INTEGER`
- `usage_records.cost REAL NOT NULL` → `BIGINT NOT NULL`
- `channels.markup_ratio REAL NOT NULL DEFAULT 1.0` → `BIGINT NOT NULL DEFAULT 10000`
- `channels.balance REAL` → `BIGINT`
- `channel_models.markup_ratio REAL NOT NULL DEFAULT 1.0` → `BIGINT NOT NULL DEFAULT 10000`

In `20260426000000_accounts_and_transactions.sql`:
- `accounts.balance DOUBLE PRECISION NOT NULL DEFAULT 0` → `BIGINT NOT NULL DEFAULT 0`
- `accounts.threshold DOUBLE PRECISION NOT NULL DEFAULT 1.0` → `BIGINT NOT NULL DEFAULT 100000000`
- `transactions.amount DOUBLE PRECISION NOT NULL CHECK(amount > 0)` → `BIGINT NOT NULL CHECK(amount > 0)`
- `transactions.balance_after DOUBLE PRECISION NOT NULL` → `BIGINT NOT NULL`

- [ ] **Step 2: Update SQLite init migration**

In `20260401000000_init.sql`:
- `api_keys.budget_monthly REAL` → `INTEGER`
- `usage_records.cost REAL NOT NULL` → `INTEGER NOT NULL`
- `channels.markup_ratio REAL NOT NULL DEFAULT 1.0` → `INTEGER NOT NULL DEFAULT 10000`
- `channels.balance REAL` → `INTEGER`
- `channel_models.markup_ratio REAL NOT NULL DEFAULT 1.0` → `INTEGER NOT NULL DEFAULT 10000`

In `20260426000000_accounts_and_transactions.sql` (sqlite):
- `accounts.balance REAL NOT NULL DEFAULT 0` → `INTEGER NOT NULL DEFAULT 0`
- `accounts.threshold REAL NOT NULL DEFAULT 1.0` → `INTEGER NOT NULL DEFAULT 100000000`
- `transactions.amount REAL NOT NULL CHECK(amount > 0)` → `INTEGER NOT NULL CHECK(amount > 0)`
- `transactions.balance_after REAL NOT NULL` → `INTEGER NOT NULL`

- [ ] **Step 3: Commit**

```bash
git add crates/storage/migrations/
git commit -m "feat: update init migrations to use integer types for monetary columns"
```

---

### Task 12: Build and test

**Files:** None new

- [ ] **Step 1: Full workspace build**

Run: `cargo build --workspace`
Expected: PASS

- [ ] **Step 2: Run existing tests**

Run: `cargo test --workspace`
Expected: Tests that use floating point comparisons need updating to use integer comparisons. Fix any failures.

- [ ] **Step 3: Run backend with SQLite**

```bash
# Switch config to SQLite
cargo run
```
Verify: server starts, no migration errors, `GET /api/v1/admin/channels` returns 401 (not 500).

- [ ] **Step 4: Run backend with PostgreSQL**

```bash
# Switch config to PostgreSQL at 10.0.17.11
cargo run
```
Verify: server starts, migration applies, no errors.

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "feat: fix tests and verify integer monetary values"
```

---

## Execution Order

Tasks 1–5 are the foundation (money module + types + row structs). Task 6 depends on 2 (billing uses types). Task 7 depends on 6 (workers use billing). Task 8 depends on 7 (handlers use internal types). Tasks 9–11 (migrations) can run in parallel with 6–8. Task 12 is final verification.

```
Task 1 (money module)
  → Task 2 (storage types)
    → Task 3 (storage trait)
      → Task 4 (postgres rows)  ─┐
      → Task 5 (sqlite rows)  ──┤
                                 ├→ Task 6 (billing)
Task 9 (pg migration) ──────────┤   → Task 7 (workers/settlement)
Task 10 (sqlite migration) ─────┤     → Task 8 (handlers)
Task 11 (init migrations) ──────┘       → Task 12 (build & test)
```
