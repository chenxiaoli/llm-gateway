# Billing & Account System Design

## 1. Overview

Add user-level balance management and async billing to LLM Gateway. The system records every financial transaction (recharge, charge, refund), supports admin manual top-ups, and can be extended to integrate real payment systems (Stripe, etc.) in the future.

## 2. Data Model

### 2.1 accounts table (new)

```sql
CREATE TABLE accounts (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL UNIQUE REFERENCES users(id),
  balance NUMERIC NOT NULL DEFAULT 0,
  threshold NUMERIC NOT NULL DEFAULT 1.0,
  currency TEXT NOT NULL DEFAULT 'USD',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_accounts_user_id ON accounts(user_id);
```

- `balance`: current balance in USD
- `threshold`: requests are rejected when balance falls below this value (default $1.0)
- `currency`: reserved for future multi-currency support
- An account is auto-created when a user registers

### 2.2 transactions table (new)

```sql
CREATE TABLE transactions (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL REFERENCES accounts(id),
  type TEXT NOT NULL,        -- credit | debit | credit_adjustment | debit_refund
  amount NUMERIC NOT NULL,   -- positive value in USD
  balance_after NUMERIC NOT NULL,
  description TEXT,
  reference_id TEXT,          -- UsageRecord.id or payment order id
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_transactions_account_id ON transactions(account_id);
CREATE INDEX idx_transactions_type ON transactions(type);
CREATE INDEX idx_transactions_created_at ON transactions(created_at);
```

- `balance_after`: snapshot of balance after this transaction (for fast historical queries)
- `reference_id`: idempotency key, prevents duplicate charges for the same reference
- `type` values:
  - `credit`: top-up / recharge
  - `debit`: usage charge
  - `credit_adjustment`: manual balance adjustment (compensation)
  - `debit_refund`: refund

## 3. Settlement Flow (Async Batch)

### 3.1 Background Task

Runs every N minutes (configurable, default 1 minute):

```
1. Get last settlement checkpoint T
2. Query all UsageRecords in (T, now]
3. Group and sum usage cost by user_id
4. For each user:
   a. Get associated account
   b. Check if a debit transaction with the same reference_id already exists (idempotency)
   c. Balance sufficient → create debit transaction, deduct balance
   d. Balance insufficient → log overdraw warning, skip (retry next run)
5. Log settlement task execution
```

### 3.2 Concurrency Safety

- Use distributed lock (PostgreSQL advisory lock or SQLite file lock)
- Only one settlement task runs at a time
- Transaction insert uses optimistic lock or ON CONFLICT for idempotency

### 3.3 Overdraw Handling

- When balance is insufficient, requests are NOT rejected (they already completed)
- Overdraw state is logged; admin can view a list of overdrawn users
- Next settlement run retries the deduction

## 4. Request Blocking (Pre-check)

Added after authentication and before proxying to upstream:

```
Request arrives
  → Auth passed (API Key → User → Account)
  → Balance check
       ├─ balance ≥ threshold → allow (continue normal flow)
       └─ balance < threshold → return 402 Payment Required
```

- Threshold = 0 means no limit
- Top-up automatically unblocks the user

## 5. Transaction Guarantees

### 5.1 Idempotency

- `reference_id` maps to UsageRecord.id
- Settlement checks if a transaction with the same reference_id exists; skips if so

### 5.2 Balance Consistency

- Each debit operation is atomic: query balance → insert transaction → update account.balance
- Database row locks ensure concurrent safety

### 5.3 Transaction Boundary

- Transaction record and account.balance update happen in the same database transaction
- On failure, rollback — no inconsistent state

## 6. API Endpoints

### 6.1 Admin API

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/admin/users/{id}/recharge` | Top up balance |
| `POST` | `/admin/users/{id}/adjust` | Manual balance adjustment |
| `POST` | `/admin/users/{id}/refund` | Issue refund |
| `GET` | `/admin/users/{id}/balance` | Get balance |
| `GET` | `/admin/users/{id}/transactions` | Paginated transaction history |
| `PATCH` | `/admin/users/{id}/balance-threshold` | Update threshold |

### 6.2 User API

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/me/balance` | Get own balance |
| `GET` | `/me/transactions` | Own transaction history |

### 6.3 Error Codes

- `402 Payment Required`: balance below threshold, request rejected

## 7. Frontend Pages

- User detail page: new "Balance" tab showing:
  - Current balance
  - Paginated transaction history
  - Admin actions: recharge, adjust buttons
- User dashboard: balance overview widget

## 8. Database Migration

SQL migrations create:
1. `accounts` table
2. `transactions` table
3. `users` table unchanged (user_id linked via accounts)

No breaking changes, backward compatible with existing data.

## 9. Extension Points

- **Payment Integration**: transaction.reference_id links to payment order ID; webhook confirms payment success
- **Multi-currency**: accounts.currency field reserved; UI and settlement logic need corresponding updates
- **Overdraw Alerts**: extend to send email/notification when balance runs low

## 10. Testing Strategy

- Unit tests: settlement calculation, idempotency, threshold logic
- Integration tests: full settlement flow (requires real database)
- Concurrency tests: multiple settlement tasks running simultaneously, verify no duplicate charges
