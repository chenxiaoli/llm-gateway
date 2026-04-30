# Monetary Value Type: Integer Subunits (8 Decimals)

## Problem

All monetary values (prices, balances, costs) use `f64` floating point across the stack. This causes:

1. **Floating point rounding errors**: `0.1 + 0.2 = 0.30000000000000004` in cost calculations
2. **PostgreSQL type mismatch**: Migration used `NUMERIC` but Rust maps `f64` → `FLOAT8`
3. **Inconsistent precision**: PostgreSQL `REAL` (4-byte) for prices vs `DOUBLE PRECISION` (8-byte) for balances
4. **Compounding errors**: Settlement worker accumulates `SUM(cost)` and subtracts from balance, compounding rounding over time

## Decision

Store all monetary values as **integer subunits** (1 USD = 100,000,000 units, 8 decimal places).

- Rust type: `i64`
- PostgreSQL type: `BIGINT`
- SQLite type: `INTEGER`
- TypeScript type: `number` (JS can exactly represent integers up to 2^53)
- JSON: plain integer (e.g., `300000` means `$0.003`)
- Max representable value: ~$92 billion

## Affected Fields

### Rust structs (`crates/storage/src/types.rs`)

| Struct | Field | Current (`f64`) | New (`i64`) |
|--------|-------|-----------------|-------------|
| ApiKey | budget_monthly | `Option<f64>` | `Option<i64>` |
| Channel | markup_ratio | `f64` | `i64` (basis points: 1.0 = 10000) |
| Channel | balance | `Option<f64>` | `Option<i64>` |
| ChannelModel | markup_ratio | `f64` | `i64` (basis points) |
| Account | balance | `f64` | `i64` |
| Account | threshold | `f64` | `i64` |
| Transaction | amount | `f64` | `i64` |
| Transaction | balance_after | `f64` | `i64` |
| UserWithBalance | balance | `f64` | `i64` |
| UserWithBalance | threshold | `f64` | `i64` |
| UsageRecord | cost | `f64` | `i64` |
| UsageSummaryRecord | total_cost | `f64` | `i64` |
| DeductBalance | amount | `f64` | `i64` |
| AddBalance | amount | `f64` | `i64` |
| CostCalculation | cost | `f64` | `i64` |

### Pricing config structs (`crates/storage/src/types.rs`)

All `Option<f64>` price fields become `Option<i64>`:

- `PerTokenConfig`: input_price_1m, output_price_1m, cache_read_price_1m, cache_creation_price_1m
- `PerRequestConfig`: request_price
- `PerCharacterConfig`: input_price_1m, output_price_1m
- `TierConfig`: input_price_1m, output_price_1m
- `HybridConfig`: base_per_call, input_price_1m, output_price_1m, cache_read_price_1m, cache_creation_price_1m

Prices are per-1M-tokens in subunits. Example: GPT-4o input at $3/1M tokens → `300000000`.

### Markup ratio: basis points

Markup ratio is a multiplier (default 1.0). Store as **basis points**: `1.0 → 10000`, `1.5 → 15000`, `0.8 → 8000`. This gives 4 decimal places of precision (0.01% increments), sufficient for markup.

## API Contract

The API continues to accept and return **decimal USD values** as floats/strings. Conversion happens at the API boundary:

- **Request parsing**: `input_price_1m: 3.0` → `300_000_000` subunits
- **Response serialization**: `300_000_000` subunits → `3.0` USD
- **markup_ratio**: `1.5` → `15000` basis points → `1.5` in response

This means the frontend does NOT need to change — it continues sending/receiving decimal values.

## Conversion Helpers

A small module (`crates/storage/src/money.rs` or similar):

```rust
const UNITS_PER_USD: i64 = 100_000_000; // 8 decimal places
const BPS_FACTOR: i64 = 10_000;

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
```

## Billing Calculation Change

Current: `cost = tokens / 1_000_000 * price_per_m * markup_ratio` (f64)

New (integer arithmetic, rounding at end):
```rust
// price_per_m is in subunits per 1M tokens
// tokens is count
// cost is in subunits
let cost_units = (tokens * price_per_m) / 1_000_000;
// apply markup in basis points
let final_cost = cost_units * markup_bps / 10_000;
```

Rounding is integer division (truncation toward zero). This is deterministic and consistent across all environments.

## Database Migrations

### PostgreSQL

New migration to alter columns:
```sql
ALTER TABLE accounts ALTER COLUMN balance TYPE BIGINT USING (ROUND(balance * 100000000));
ALTER TABLE accounts ALTER COLUMN threshold TYPE BIGINT USING (ROUND(threshold * 100000000));
ALTER TABLE transactions ALTER COLUMN amount TYPE BIGINT USING (ROUND(amount * 100000000));
ALTER TABLE transactions ALTER COLUMN balance_after TYPE BIGINT USING (ROUND(balance_after * 100000000));
ALTER TABLE usage_records ALTER COLUMN cost TYPE BIGINT USING (ROUND(cost * 100000000));
ALTER TABLE channels ALTER COLUMN balance TYPE BIGINT USING (ROUND(COALESCE(balance, 0) * 100000000));
ALTER TABLE channels ALTER COLUMN markup_ratio TYPE BIGINT USING (ROUND(markup_ratio * 10000));
ALTER TABLE api_keys ALTER COLUMN budget_monthly TYPE BIGINT USING (ROUND(COALESCE(budget_monthly, 0) * 100000000));
ALTER TABLE channel_models ALTER COLUMN markup_ratio TYPE BIGINT USING (ROUND(markup_ratio * 10000));
```

### SQLite

SQLite doesn't support `ALTER COLUMN`. Use the standard recreate-table approach:
1. Create new table with INTEGER columns
2. `INSERT INTO new SELECT ... ROUND(old_col * 100000000) ... FROM old`
3. `DROP TABLE old`
4. `ALTER TABLE new RENAME TO old`

## What Does NOT Change

- **Token counts** (`input_tokens`, `output_tokens`, `cache_read_tokens`, `cache_creation_tokens`): remain `i64`
- **Pricing policy `config` column**: remains `TEXT` storing JSON. The JSON values change from floats to integers.
- **Frontend display**: continues to show decimal USD values (conversion at API boundary)
- **Rate limits** (`rpm_limit`, `tpm_limit`): remain `i64`
- **Priority, weight**: remain integer types
