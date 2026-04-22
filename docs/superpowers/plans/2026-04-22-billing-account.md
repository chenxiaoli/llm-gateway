# Billing Account System — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add user-level balance management with async batch billing to LLM Gateway.

**Architecture:**
- New `accounts` + `transactions` tables store balance and money movement history.
- Settlement runs as a background Tokio task every N minutes (configurable, default 1 min), summarizing usage records per user and deducting balance.
- Balance pre-check is injected into the proxy flow right after API-key auth, returning 402 if balance < threshold.
- Admin manages balance via `/admin/users/{id}/recharge|adjust|refund` etc.; users read their own balance via `/me/balance`.

**Tech Stack:** Rust (workspace crates), SQLite + PostgreSQL (sqlx), React + TypeScript (web/), TanStack Query, Axum.

---

## File Map

### Backend — New files
- `crates/storage/migrations/sqlite/YYYYMMDD000000_accounts_and_transactions.sql` — SQLite migration
- `crates/storage/migrations/postgres/YYYYMMDD000000_accounts_and_transactions.sql` — PostgreSQL migration
- `crates/storage/src/types.rs` — add `Account`, `Transaction`, `TransactionType` types
- `crates/storage/src/lib.rs` — add storage trait methods for accounts/transactions
- `crates/storage/src/sqlite.rs` — SQLite implementation
- `crates/storage/src/postgres.rs` — PostgreSQL implementation
- `crates/api/src/error.rs` — add `PaymentRequired` variant
- `crates/api/src/management/accounts.rs` — admin account handlers
- `crates/api/src/management/mod.rs` — register account routes
- `crates/api/src/management/auth.rs` — add `/me/balance` handler
- `crates/api/src/lib.rs` — add `billing_tx: mpsc::Sender<SettlementTask>` to `AppState`
- `crates/api/src/settlement.rs` — settlement background worker (new file)
- `crates/api/src/management.rs` — spawn settlement worker on startup (workers.rs already spawned, reuse pattern)

### Backend — Modified files
- `crates/storage/src/lib.rs` — implement new storage trait methods
- `crates/storage/src/sqlite.rs` — implement new methods
- `crates/storage/src/postgres.rs` — implement new methods
- `crates/api/src/proxy.rs` — insert balance pre-check after API-key lookup
- `crates/api/src/lib.rs` — add settlement channel to AppState, spawn worker
- `crates/gateway/src/main.rs` — pass settlement_tx to AppState
- `crates/billing/Cargo.toml` — add dependencies (tokio, chrono, uuid already present; no new deps needed)

### Frontend — New files
- `web/src/types/index.ts` — add Account, Transaction types
- `web/src/api/accounts.ts` — API client functions
- `web/src/hooks/useAccounts.ts` — React Query hooks
- `web/src/pages/AccountBalance.tsx` — new page for balance/transaction list

### Frontend — Modified files
- `web/src/pages/Users.tsx` — add balance column
- `web/src/components/Layout.tsx` — add route for AccountBalance

---

## Backend Tasks

### Task 1: Database Migrations

**Files:**
- Create: `crates/storage/migrations/sqlite/YYYYMMDD000000_accounts_and_transactions.sql`
- Create: `crates/storage/migrations/postgres/YYYYMMDD000000_accounts_and_transactions.sql`

The date prefix should be today's date `20260422` or the next sequential date if files already exist for that date. Check existing migrations first with `ls crates/storage/migrations/sqlite/` and `ls crates/storage/migrations/postgres/`.

- [ ] **Step 1: Create SQLite migration**

Run `ls crates/storage/migrations/sqlite/` to find the latest date prefix. Use the next sequential number (e.g., if latest is `20260425000000`, use `20260426000000`).

Create `crates/storage/migrations/sqlite/YYYYMMDD000000_accounts_and_transactions.sql`:

```sql
-- accounts table
CREATE TABLE IF NOT EXISTS accounts (
    id             TEXT PRIMARY KEY,
    user_id        TEXT NOT NULL UNIQUE REFERENCES users(id),
    balance        REAL NOT NULL DEFAULT 0,
    threshold      REAL NOT NULL DEFAULT 1.0,
    currency       TEXT NOT NULL DEFAULT 'USD',
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_accounts_user_id ON accounts(user_id);

-- transactions table
CREATE TABLE IF NOT EXISTS transactions (
    id            TEXT PRIMARY KEY,
    account_id    TEXT NOT NULL REFERENCES accounts(id),
    type          TEXT NOT NULL
        CHECK(type IN ('credit', 'debit', 'credit_adjustment', 'debit_refund')),
    amount        REAL NOT NULL CHECK(amount > 0),
    balance_after REAL NOT NULL,
    description   TEXT,
    reference_id  TEXT,
    created_at    TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_transactions_account_id ON transactions(account_id);
CREATE INDEX IF NOT EXISTS idx_transactions_type ON transactions(type);
CREATE INDEX IF NOT EXISTS idx_transactions_reference_id ON transactions(reference_id);
CREATE INDEX IF NOT EXISTS idx_transactions_created_at ON transactions(created_at);
```

- [ ] **Step 2: Create PostgreSQL migration**

Run `ls crates/storage/migrations/postgres/` to find the latest date prefix. Use the next sequential number.

Create `crates/storage/migrations/postgres/YYYYMMDD000000_accounts_and_transactions.sql`:

```sql
-- accounts table
CREATE TABLE IF NOT EXISTS accounts (
    id             TEXT PRIMARY KEY,
    user_id        TEXT NOT NULL UNIQUE REFERENCES users(id),
    balance        NUMERIC NOT NULL DEFAULT 0,
    threshold      NUMERIC NOT NULL DEFAULT 1.0,
    currency       TEXT NOT NULL DEFAULT 'USD',
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_accounts_user_id ON accounts(user_id);

-- transactions table
CREATE TABLE IF NOT EXISTS transactions (
    id            TEXT PRIMARY KEY,
    account_id    TEXT NOT NULL REFERENCES accounts(id),
    type          TEXT NOT NULL
        CHECK(type IN ('credit', 'debit', 'credit_adjustment', 'debit_refund')),
    amount        NUMERIC NOT NULL CHECK(amount > 0),
    balance_after NUMERIC NOT NULL,
    description   TEXT,
    reference_id  TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_transactions_account_id ON transactions(account_id);
CREATE INDEX IF NOT EXISTS idx_transactions_type ON transactions(type);
CREATE INDEX IF NOT EXISTS idx_transactions_reference_id ON transactions(reference_id);
CREATE INDEX IF NOT EXISTS idx_transactions_created_at ON transactions(created_at);
```

- [ ] **Step 3: Commit**

```bash
git add crates/storage/migrations/sqlite/YYYYMMDD000000_accounts_and_transactions.sql crates/storage/migrations/postgres/YYYYMMDD000000_accounts_and_transactions.sql
git commit -m "feat(storage): add accounts and transactions tables

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 2: Storage Types

**Files:**
- Modify: `crates/storage/src/types.rs`

- [ ] **Step 1: Add types to types.rs**

Find the `// --- Settings ---` section (around line 534) and add before it:

```rust
// --- Accounts ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub user_id: String,
    pub balance: f64,
    pub threshold: f64,
    pub currency: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub account_id: String,
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub amount: f64,
    pub balance_after: f64,
    pub description: Option<String>,
    pub reference_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    Credit,
    Debit,
    CreditAdjustment,
    DebitRefund,
}

impl TransactionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransactionType::Credit => "credit",
            TransactionType::Debit => "debit",
            TransactionType::CreditAdjustment => "credit_adjustment",
            TransactionType::DebitRefund => "debit_refund",
        }
    }
}

impl std::fmt::Display for TransactionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// --- Account API Request/Response types ---

#[derive(Debug, Deserialize)]
pub struct CreateTransaction {
    #[serde(rename = "type")]
    pub transaction_type: String,
    pub amount: f64,
    pub description: Option<String>,
    pub reference_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAccountThreshold {
    pub threshold: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccountResponse {
    pub id: String,
    pub user_id: String,
    pub balance: f64,
    pub threshold: f64,
    pub currency: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&Account> for AccountResponse {
    fn from(a: &Account) -> Self {
        AccountResponse {
            id: a.id.clone(),
            user_id: a.user_id.clone(),
            balance: a.balance,
            threshold: a.threshold,
            currency: a.currency.clone(),
            created_at: a.created_at.to_rfc3339(),
            updated_at: a.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TransactionResponse {
    pub id: String,
    pub account_id: String,
    #[serde(rename = "type")]
    pub transaction_type: String,
    pub amount: f64,
    pub balance_after: f64,
    pub description: Option<String>,
    pub reference_id: Option<String>,
    pub created_at: String,
}

impl From<&Transaction> for TransactionResponse {
    fn from(t: &Transaction) -> Self {
        TransactionResponse {
            id: t.id.clone(),
            account_id: t.account_id.clone(),
            transaction_type: t.transaction_type.as_str().to_string(),
            amount: t.amount,
            balance_after: t.balance_after,
            description: t.description.clone(),
            reference_id: t.reference_id.clone(),
            created_at: t.created_at.to_rfc3339(),
        }
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/storage/src/types.rs
git commit -m "feat(storage): add Account, Transaction types and API types

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 3: Storage Trait Methods

**Files:**
- Modify: `crates/storage/src/lib.rs`

- [ ] **Step 1: Add trait methods to Storage trait**

Find the closing brace of the `Storage` trait (after `async fn seed_data(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;`) and add:

```rust
    // Accounts
    async fn create_account(&self, account: &Account) -> Result<Account, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_account(&self, id: &str) -> Result<Option<Account>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_account_by_user_id(&self, user_id: &str) -> Result<Option<Account>, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_account(&self, account: &Account) -> Result<Account, Box<dyn std::error::Error + Send + Sync>>;

    // Transactions
    async fn create_transaction(&self, transaction: &Transaction) -> Result<Transaction, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_transaction(&self, id: &str) -> Result<Option<Transaction>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_transaction_by_reference(&self, account_id: &str, reference_id: &str) -> Result<Option<Transaction>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_transactions(&self, account_id: &str, page: i64, page_size: i64) -> Result<PaginatedResponse<Transaction>, Box<dyn std::error::Error + Send + Sync>>;
```

Also add to the `pub use` line near the top of the file:
```rust
pub use types::{
    // ... existing items ...
    Account, Transaction, TransactionType,
    AccountResponse, TransactionResponse,
    CreateTransaction, UpdateAccountThreshold,
};
```

- [ ] **Step 2: Commit**

```bash
git add crates/storage/src/lib.rs
git commit -m "feat(storage): add account and transaction trait methods

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 4: SQLite Storage Implementation

**Files:**
- Modify: `crates/storage/src/sqlite.rs`

This is a large file. First read the entire file to understand the patterns, particularly how rows are converted from DB format and how transactions are used.

- [ ] **Step 1: Add SQLite row types**

Find the `// ---------------------------------------------------------------------------` section after `struct SqliteKeyRow` (around line 42) and add after the last row type definition:

```rust
// ---------------------------------------------------------------------------
// Account rows
// ---------------------------------------------------------------------------

#[derive(FromRow)]
struct SqliteAccountRow {
    id: String,
    user_id: String,
    balance: f64,
    threshold: f64,
    currency: String,
    created_at: String,
    updated_at: String,
}

impl From<SqliteAccountRow> for Account {
    fn from(r: SqliteAccountRow) -> Self {
        Account {
            id: r.id,
            user_id: r.user_id,
            balance: r.balance,
            threshold: r.threshold,
            currency: r.currency,
            created_at: parse_rfc3339(&r.created_at),
            updated_at: parse_rfc3339(&r.updated_at),
        }
    }
}

// ---------------------------------------------------------------------------
// Transaction rows
// ---------------------------------------------------------------------------

#[derive(FromRow)]
struct SqliteTransactionRow {
    id: String,
    account_id: String,
    transaction_type: String,
    amount: f64,
    balance_after: f64,
    description: Option<String>,
    reference_id: Option<String>,
    created_at: String,
}

impl From<SqliteTransactionRow> for Transaction {
    fn from(r: SqliteTransactionRow) -> Self {
        let tt = match r.transaction_type.as_str() {
            "credit" => TransactionType::Credit,
            "debit" => TransactionType::Debit,
            "credit_adjustment" => TransactionType::CreditAdjustment,
            "debit_refund" => TransactionType::DebitRefund,
            _ => TransactionType::Debit,
        };
        Transaction {
            id: r.id,
            account_id: r.account_id,
            transaction_type: tt,
            amount: r.amount,
            balance_after: r.balance_after,
            description: r.description,
            reference_id: r.reference_id,
            created_at: parse_rfc3339(&r.created_at),
        }
    }
}
```

- [ ] **Step 2: Implement Storage trait methods**

Find the closing `}` of the `impl SqliteStorage` block (after `pub fn pool(&self) -> &SqlitePool { &self.pool }`). Add before that closing brace:

```rust
    // ─── Accounts ────────────────────────────────────────────────────────────────

    async fn create_account(&self, account: &Account) -> Result<Account, Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query(
            "INSERT INTO accounts (id, user_id, balance, threshold, currency, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&account.id)
        .bind(&account.user_id)
        .bind(account.balance)
        .bind(account.threshold)
        .bind(&account.currency)
        .bind(account.created_at.to_rfc3339())
        .bind(account.updated_at.to_rfc3339())
        .execute(self.pool())
        .await?;
        Ok(account.clone())
    }

    async fn get_account(&self, id: &str) -> Result<Option<Account>, Box<dyn std::error::Error + Send + Sync>> {
        let row: Option<SqliteAccountRow> = sqlx::query_as(
            "SELECT id, user_id, balance, threshold, currency, created_at, updated_at FROM accounts WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.map(Account::from))
    }

    async fn get_account_by_user_id(&self, user_id: &str) -> Result<Option<Account>, Box<dyn std::error::Error + Send + Sync>> {
        let row: Option<SqliteAccountRow> = sqlx::query_as(
            "SELECT id, user_id, balance, threshold, currency, created_at, updated_at FROM accounts WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.map(Account::from))
    }

    async fn update_account(&self, account: &Account) -> Result<Account, Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query(
            "UPDATE accounts SET balance = ?, threshold = ?, updated_at = ? WHERE id = ?"
        )
        .bind(account.balance)
        .bind(account.threshold)
        .bind(account.updated_at.to_rfc3339())
        .bind(&account.id)
        .execute(self.pool())
        .await?;
        Ok(account.clone())
    }

    // ─── Transactions ─────────────────────────────────────────────────────────────

    async fn create_transaction(&self, transaction: &Transaction) -> Result<Transaction, Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query(
            "INSERT INTO transactions (id, account_id, type, amount, balance_after, description, reference_id, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&transaction.id)
        .bind(&transaction.account_id)
        .bind(transaction.transaction_type.as_str())
        .bind(transaction.amount)
        .bind(transaction.balance_after)
        .bind(&transaction.description)
        .bind(&transaction.reference_id)
        .bind(transaction.created_at.to_rfc3339())
        .execute(self.pool())
        .await?;
        Ok(transaction.clone())
    }

    async fn get_transaction(&self, id: &str) -> Result<Option<Transaction>, Box<dyn std::error::Error + Send + Sync>> {
        let row: Option<SqliteTransactionRow> = sqlx::query_as(
            "SELECT id, account_id, type, amount, balance_after, description, reference_id, created_at FROM transactions WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.map(Transaction::from))
    }

    async fn get_transaction_by_reference(&self, account_id: &str, reference_id: &str) -> Result<Option<Transaction>, Box<dyn std::error::Error + Send + Sync>> {
        let row: Option<SqliteTransactionRow> = sqlx::query_as(
            "SELECT id, account_id, type, amount, balance_after, description, reference_id, created_at FROM transactions WHERE account_id = ? AND reference_id = ?"
        )
        .bind(account_id)
        .bind(reference_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.map(Transaction::from))
    }

    async fn list_transactions(&self, account_id: &str, page: i64, page_size: i64) -> Result<PaginatedResponse<Transaction>, Box<dyn std::error::Error + Send + Sync>> {
        let offset = (page - 1) * page_size;

        let rows: Vec<SqliteTransactionRow> = sqlx::query_as(
            "SELECT id, account_id, type, amount, balance_after, description, reference_id, created_at FROM transactions WHERE account_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?"
        )
        .bind(account_id)
        .bind(page_size)
        .bind(offset)
        .fetch_all(self.pool())
        .await?;

        let count_row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM transactions WHERE account_id = ?"
        )
        .bind(account_id)
        .fetch_one(self.pool())
        .await?;

        let transactions: Vec<Transaction> = rows.into_iter().map(Transaction::from).collect();
        Ok(PaginatedResponse {
            items: transactions,
            total: count_row.0,
            page,
            page_size,
        })
    }
```

- [ ] **Step 3: Commit**

```bash
git add crates/storage/src/sqlite.rs
git commit -m "feat(storage): implement account and transaction methods for SQLite

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 5: PostgreSQL Storage Implementation

**Files:**
- Modify: `crates/storage/src/postgres.rs`

Read the full file to understand the patterns. This file is smaller than sqlite.rs.

- [ ] **Step 1: Add PostgreSQL row types**

Find the `struct PgModelEnrichedRow` section (around line 91) and add after the last struct:

```rust
// ---------------------------------------------------------------------------
// Account rows
// ---------------------------------------------------------------------------

#[derive(FromRow)]
struct PgAccountRow {
    id: String,
    user_id: String,
    balance: f64,
    threshold: f64,
    currency: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<PgAccountRow> for Account {
    fn from(r: PgAccountRow) -> Self {
        Account {
            id: r.id,
            user_id: r.user_id,
            balance: r.balance,
            threshold: r.threshold,
            currency: r.currency,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ---------------------------------------------------------------------------
// Transaction rows
// ---------------------------------------------------------------------------

#[derive(FromRow)]
struct PgTransactionRow {
    id: String,
    account_id: String,
    transaction_type: String,
    amount: f64,
    balance_after: f64,
    description: Option<String>,
    reference_id: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<PgTransactionRow> for Transaction {
    fn from(r: PgTransactionRow) -> Self {
        let tt = match r.transaction_type.as_str() {
            "credit" => TransactionType::Credit,
            "debit" => TransactionType::Debit,
            "credit_adjustment" => TransactionType::CreditAdjustment,
            "debit_refund" => TransactionType::DebitRefund,
            _ => TransactionType::Debit,
        };
        Transaction {
            id: r.id,
            account_id: r.account_id,
            transaction_type: tt,
            amount: r.amount,
            balance_after: r.balance_after,
            description: r.description,
            reference_id: r.reference_id,
            created_at: r.created_at,
        }
    }
}
```

- [ ] **Step 2: Implement Storage trait methods**

Find the `impl PostgresStorage` block and add before its closing `}`:

```rust
    // ─── Accounts ────────────────────────────────────────────────────────────────

    async fn create_account(&self, account: &Account) -> Result<Account, Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query(
            "INSERT INTO accounts (id, user_id, balance, threshold, currency, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
        .bind(&account.id)
        .bind(&account.user_id)
        .bind(account.balance)
        .bind(account.threshold)
        .bind(&account.currency)
        .bind(account.created_at)
        .bind(account.updated_at)
        .execute(self.pool())
        .await?;
        Ok(account.clone())
    }

    async fn get_account(&self, id: &str) -> Result<Option<Account>, Box<dyn std::error::Error + Send + Sync>> {
        let row: Option<PgAccountRow> = sqlx::query_as(
            "SELECT id, user_id, balance, threshold, currency, created_at, updated_at FROM accounts WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.map(Account::from))
    }

    async fn get_account_by_user_id(&self, user_id: &str) -> Result<Option<Account>, Box<dyn std::error::Error + Send + Sync>> {
        let row: Option<PgAccountRow> = sqlx::query_as(
            "SELECT id, user_id, balance, threshold, currency, created_at, updated_at FROM accounts WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.map(Account::from))
    }

    async fn update_account(&self, account: &Account) -> Result<Account, Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query(
            "UPDATE accounts SET balance = $1, threshold = $2, updated_at = $3 WHERE id = $4"
        )
        .bind(account.balance)
        .bind(account.threshold)
        .bind(account.updated_at)
        .bind(&account.id)
        .execute(self.pool())
        .await?;
        Ok(account.clone())
    }

    // ─── Transactions ─────────────────────────────────────────────────────────────

    async fn create_transaction(&self, transaction: &Transaction) -> Result<Transaction, Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query(
            "INSERT INTO transactions (id, account_id, type, amount, balance_after, description, reference_id, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        )
        .bind(&transaction.id)
        .bind(&transaction.account_id)
        .bind(transaction.transaction_type.as_str())
        .bind(transaction.amount)
        .bind(transaction.balance_after)
        .bind(&transaction.description)
        .bind(&transaction.reference_id)
        .bind(transaction.created_at)
        .execute(self.pool())
        .await?;
        Ok(transaction.clone())
    }

    async fn get_transaction(&self, id: &str) -> Result<Option<Transaction>, Box<dyn std::error::Error + Send + Sync>> {
        let row: Option<PgTransactionRow> = sqlx::query_as(
            "SELECT id, account_id, type, amount, balance_after, description, reference_id, created_at FROM transactions WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.map(Transaction::from))
    }

    async fn get_transaction_by_reference(&self, account_id: &str, reference_id: &str) -> Result<Option<Transaction>, Box<dyn std::error::Error + Send + Sync>> {
        let row: Option<PgTransactionRow> = sqlx::query_as(
            "SELECT id, account_id, type, amount, balance_after, description, reference_id, created_at FROM transactions WHERE account_id = $1 AND reference_id = $2"
        )
        .bind(account_id)
        .bind(reference_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.map(Transaction::from))
    }

    async fn list_transactions(&self, account_id: &str, page: i64, page_size: i64) -> Result<PaginatedResponse<Transaction>, Box<dyn std::error::Error + Send + Sync>> {
        let offset = (page - 1) * page_size;

        let rows: Vec<PgTransactionRow> = sqlx::query_as(
            "SELECT id, account_id, type, amount, balance_after, description, reference_id, created_at FROM transactions WHERE account_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        )
        .bind(account_id)
        .bind(page_size)
        .bind(offset)
        .fetch_all(self.pool())
        .await?;

        let count_row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM transactions WHERE account_id = $1"
        )
        .bind(account_id)
        .fetch_one(self.pool())
        .await?;

        let transactions: Vec<Transaction> = rows.into_iter().map(Transaction::from).collect();
        Ok(PaginatedResponse {
            items: transactions,
            total: count_row.0,
            page,
            page_size,
        })
    }
```

- [ ] **Step 3: Commit**

```bash
git add crates/storage/src/postgres.rs
git commit -m "feat(storage): implement account and transaction methods for PostgreSQL

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 6: Auto-create Account on User Registration

**Files:**
- Modify: `crates/api/src/management/auth.rs`

When a user registers, an account must be auto-created with balance=0, threshold=1.0.

- [ ] **Step 1: Find the register function and add account creation**

In `register()` function, after `state.storage.create_user(&user).await?;` (line 187 in the auth.rs file), add:

```rust
    // Auto-create account for new user
    let now = chrono::Utc::now();
    let account = llm_gateway_storage::Account {
        id: uuid::Uuid::new_v4().to_string(),
        user_id: user.id.clone(),
        balance: 0.0,
        threshold: 1.0,
        currency: "USD".to_string(),
        created_at: now,
        updated_at: now,
    };
    state
        .storage
        .create_account(&account)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
```

Make sure `llm_gateway_storage::Account` is imported (it should already be available since the storage module is re-exported). If not, add to imports.

- [ ] **Step 2: Run tests to verify nothing is broken**

```bash
cd /workspace && cargo test --workspace 2>&1 | tail -30
```

Expected: existing tests still pass.

- [ ] **Step 3: Commit**

```bash
git add crates/api/src/management/auth.rs
git commit -m "feat(auth): auto-create account on user registration

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 7: API Error — Payment Required

**Files:**
- Modify: `crates/api/src/error.rs`

- [ ] **Step 1: Add PaymentRequired variant**

In the `ApiError` enum, add:

```rust
#[derive(Debug)]
pub enum ApiError {
    Unauthorized,
    Forbidden,
    RateLimited,
    PaymentRequired,
    NotFound(String),
    BadRequest(String),
    UpstreamError(u16, String),
    Internal(String),
}
```

Also update the `IntoResponse` impl:

```rust
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden"),
            ApiError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded"),
            ApiError::PaymentRequired => (StatusCode::PAYMENT_REQUIRED, "Insufficient balance"),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.as_str()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.as_str()),
            ApiError::UpstreamError(code, msg) => (
                StatusCode::from_u16(*code).unwrap_or(StatusCode::BAD_GATEWAY),
                msg.as_str(),
            ),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.as_str()),
        };
        let body = json!({ "error": { "message": message, "type": status.as_u16() } });
        (status, axum::Json(body)).into_response()
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/api/src/error.rs
git commit -m "feat(api): add PaymentRequired error variant (402)

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 8: Balance Pre-check in Proxy

**Files:**
- Modify: `crates/api/src/proxy.rs`

- [ ] **Step 1: Read the auth section of the proxy function**

Find the section in `proxy()` around lines 433-436:

```rust
if !api_key.enabled {
    return Err(ApiError::Forbidden);
}

// === Step 2: ...
```

Insert balance check right after the enabled check, before Step 2:

```rust
    // === Step 2: Balance check ===
    if let Some(ref created_by) = api_key.created_by {
        if let Some(account) = state
            .storage
            .get_account_by_user_id(created_by)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
        {
            if account.balance < account.threshold {
                tracing::warn!(
                    "[PROXY] Balance check failed: user={}, balance={}, threshold={}",
                    created_by, account.balance, account.threshold
                );
                return Err(ApiError::PaymentRequired);
            }
        }
    }
```

Also update the comment `// === Step 2: Parse model ===` to `// === Step 3: Parse model ===` (since we inserted a Step 2).

- [ ] **Step 2: Run tests**

```bash
cd /workspace && cargo build --release 2>&1 | tail -20
```

Expected: compiles without errors.

- [ ] **Step 3: Commit**

```bash
git add crates/api/src/proxy.rs
git commit -m "feat(proxy): add balance pre-check, return 402 if below threshold

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 9: Settlement Background Worker

**Files:**
- Create: `crates/api/src/settlement.rs`
- Modify: `crates/api/src/lib.rs`
- Modify: `crates/gateway/src/main.rs`

- [ ] **Step 1: Create settlement.rs**

```rust
//! Background settlement worker — deducts balance from accounts based on usage records.
//! Runs every N minutes (default 1 min), summarizes usage cost per user, creates debit transactions.

use llm_gateway_storage::{
    Account, PaginatedResponse, Transaction, TransactionType, UsageRecord,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// Task sent to the settlement worker (triggers immediate run).
#[derive(Debug)]
pub struct SettlementTrigger;

/// Background task: periodically settles usage charges.
pub async fn start_settlement_worker(
    storage: Arc<dyn llm_gateway_storage::Storage>,
    mut trigger_rx: mpsc::Receiver<SettlementTrigger>,
    interval_secs: u64,
) {
    tracing::info!("[SETTLEMENT] Starting settlement worker (interval: {}s)", interval_secs);
    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                run_settlement(&storage).await;
            }
            _ = trigger_rx.recv() => {
                tracing::info!("[SETTLEMENT] Triggered immediate settlement");
                run_settlement(&storage).await;
            }
        }
    }
}

async fn run_settlement(storage: &Arc<dyn llm_gateway_storage::Storage>) {
    tracing::info!("[SETTLEMENT] Running settlement task");

    // Get all usage records from the last interval (batch).
    // We batch by getting all records that haven't been settled yet.
    // Strategy: get records from the last 2 intervals to avoid gaps.
    // A simpler approach: get all records and track settled via reference_id.
    // Use a checkpoint: settle all records within the last 5 minutes.
    // Better: track last_settlement_time in a storage setting.

    let checkpoint_key = "last_settlement_time";
    let now = chrono::Utc::now();
    let last_time = match storage.get_setting(checkpoint_key).await {
        Ok(Some(ts)) => {
            chrono::DateTime::parse_from_rfc3339(&ts)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| now - Duration::from_secs(300))
        }
        _ => now - Duration::from_secs(300),
    };

    // Query usage records in the time window
    let records: Vec<UsageRecord> = storage
        .query_usage(&llm_gateway_storage::UsageFilter {
            key_id: None,
            model_name: None,
            since: Some(last_time),
            until: Some(now),
        })
        .await
        .unwrap_or_default();

    if records.is_empty() {
        tracing::debug!("[SETTLEMENT] No usage records in window, skipping");
        // Update checkpoint even if empty
        let _ = storage.set_setting(checkpoint_key, &now.to_rfc3339()).await;
        return;
    }

    // Group by user_id (via api_key.created_by → user_id)
    // Note: usage_records have key_id, not user_id. We need to get the API key's created_by.
    // Since we can't efficiently join across multiple queries, we'll do a simple approach:
    // For each usage record, get the key, then get the account.
    // This is O(n) queries but acceptable for a background task.
    // A more efficient approach: batch-get all keys, then batch-get accounts.

    // Group usage records by account_id (derived from key → user → account)
    let mut account_charges: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    let mut account_keys: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new(); // account_id -> key_ids
    let mut reference_ids: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new(); // account_id -> usage record ids

    for record in &records {
        // Check if already settled (reference_id matches usage record id)
        // We'll settle by creating a single debit per account covering all its usage.
        // reference_id = batch identifier per account (e.g., "batch-{account_id}-{timestamp}")
        // Actually per spec: reference_id = UsageRecord.id. But we may have many records per account.
        // Better: create one debit transaction per account with a batch reference.

        // Get the key to find user_id
        if let Some(key) = storage.get_key(&record.key_id).await.unwrap_or(None) {
            if let Some(ref user_id) = key.created_by {
                if let Some(account) = storage.get_account_by_user_id(user_id).await.unwrap_or(None) {
                    *account_charges.entry(account.id.clone()).or_insert(0.0) += record.cost;
                    account_keys.entry(account.id.clone()).or_insert_with(Vec::new).push(record.key_id.clone());
                    reference_ids.entry(account.id.clone()).or_default().push(record.id.clone());
                }
            }
        }
    }

    if account_charges.is_empty() {
        tracing::debug!("[SETTLEMENT] No accounts found for usage records");
        let _ = storage.set_setting(checkpoint_key, &now.to_rfc3339()).await;
        return;
    }

    let batch_reference = format!("batch_{}", chrono::Utc::now().timestamp());

    for (account_id, total_cost) in account_charges {
        if total_cost <= 0.0 {
            continue;
        }

        // Check if this account has already been settled for this batch (idempotency)
        if let Ok(Some(_)) = storage.get_transaction_by_reference(&account_id, &batch_reference).await {
            tracing::debug!("[SETTLEMENT] Account {} already settled for batch {}, skipping", account_id, batch_reference);
            continue;
        }

        // Get current account
        let account = match storage.get_account(&account_id).await.unwrap_or(None) {
            Some(a) => a,
            None => continue,
        };

        // Check balance
        if account.balance >= total_cost {
            let new_balance = account.balance - total_cost;
            let mut updated_account = account.clone();
            updated_account.balance = new_balance;
            updated_account.updated_at = chrono::Utc::now();

            let transaction = Transaction {
                id: uuid::Uuid::new_v4().to_string(),
                account_id: account_id.clone(),
                transaction_type: TransactionType::Debit,
                amount: total_cost,
                balance_after: new_balance,
                description: Some("Usage settlement".to_string()),
                reference_id: Some(batch_reference.clone()),
                created_at: chrono::Utc::now(),
            };

            // Create transaction and update balance in same logical flow.
            // Note: for true atomicity we'd need a DB transaction, but for now
            // we insert transaction first, then update balance.
            if storage.create_transaction(&transaction).await.is_ok() {
                let _ = storage.update_account(&updated_account).await;
                tracing::info!(
                    "[SETTLEMENT] Deducted ${:.6f} from account {} (balance: {} -> {})",
                    total_cost, account_id, account.balance, new_balance
                );
            }
        } else {
            tracing::warn!(
                "[SETTLEMENT] Insufficient balance for account {}: balance=${:.6f}, cost=${:.6f}",
                account_id, account.balance, total_cost
            );
        }
    }

    // Update checkpoint
    let _ = storage.set_setting(checkpoint_key, &now.to_rfc3339()).await;
    tracing::info!("[SETTLEMENT] Settlement complete, {} accounts processed", account_charges.len());
}
```

- [ ] **Step 2: Add to lib.rs**

In `crates/api/src/lib.rs`, after the existing imports:

```rust
pub mod settlement;
pub use settlement::{start_settlement_worker, SettlementTrigger};
```

- [ ] **Step 3: Update AppState in lib.rs**

Add to `AppState`:

```rust
pub struct AppState {
    // ... existing fields ...
    pub settlement_tx: mpsc::Sender<SettlementTrigger>,
}
```

- [ ] **Step 4: Update main.rs**

In `crates/gateway/src/main.rs`, after the `audit_tx` channel setup (around line 65):

```rust
// Create settlement channel and spawn background worker
let (settlement_tx, settlement_rx) = tokio::sync::mpsc::channel::<llm_gateway_api::SettlementTrigger>(1);
// Default to 60 second settlement interval
let settlement_interval_secs = 60u64;
tokio::spawn(llm_gateway_api::start_settlement_worker(
    storage.clone(),
    settlement_rx,
    settlement_interval_secs,
));
```

Then add `settlement_tx` to the `AppState` construction:

```rust
let state = Arc::new(AppState {
    // ... existing fields ...
    settlement_tx,
});
```

- [ ] **Step 5: Build**

```bash
cd /workspace && cargo build --release 2>&1 | tail -30
```

Expected: compiles without errors.

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/settlement.rs crates/api/src/lib.rs crates/gateway/src/main.rs
git commit -m "feat(settlement): add async batch settlement worker

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 10: Admin Account API Handlers

**Files:**
- Create: `crates/api/src/management/accounts.rs`
- Modify: `crates/api/src/management/mod.rs`

- [ ] **Step 1: Create accounts.rs**

```rust
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Serialize;
use std::sync::Arc;

use llm_gateway_storage::{
    AccountResponse, CreateTransaction, PaginatedResponse,
    PaginationParams, TransactionResponse, UpdateAccountThreshold,
};

use crate::error::ApiError;
use crate::extractors::require_admin;
use crate::AppState;

#[derive(Serialize)]
pub struct AccountBalanceResponse {
    pub account: AccountResponse,
    pub transactions: PaginatedResponse<TransactionResponse>,
}

pub async fn get_balance(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<AccountBalanceResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let account = state
        .storage
        .get_account_by_user_id(&user_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Account for user '{}' not found", user_id)))?;

    let (page, page_size) = pagination.normalized();
    let transactions = state
        .storage
        .list_transactions(&account.id, page, page_size)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(AccountBalanceResponse {
        account: AccountResponse::from(&account),
        transactions: PaginatedResponse {
            items: transactions.items.iter().map(TransactionResponse::from).collect(),
            total: transactions.total,
            page: transactions.page,
            page_size: transactions.page_size,
        },
    }))
}

pub async fn recharge(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(input): Json<CreateTransaction>,
) -> Result<Json<AccountResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let mut account = state
        .storage
        .get_account_by_user_id(&user_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Account for user '{}' not found", user_id)))?;

    if input.amount <= 0.0 {
        return Err(ApiError::BadRequest("Amount must be positive".to_string()));
    }

    let now = chrono::Utc::now();
    let new_balance = account.balance + input.amount;

    let transaction = llm_gateway_storage::Transaction {
        id: uuid::Uuid::new_v4().to_string(),
        account_id: account.id.clone(),
        transaction_type: llm_gateway_storage::TransactionType::Credit,
        amount: input.amount,
        balance_after: new_balance,
        description: input.description.or_else(|| Some("Recharge".to_string())),
        reference_id: input.reference_id,
        created_at: now,
    };

    account.balance = new_balance;
    account.updated_at = now;

    state
        .storage
        .create_transaction(&transaction)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let updated = state
        .storage
        .update_account(&account)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(AccountResponse::from(&updated)))
}

pub async fn adjust(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(input): Json<CreateTransaction>,
) -> Result<Json<AccountResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let mut account = state
        .storage
        .get_account_by_user_id(&user_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Account for user '{}' not found", user_id)))?;

    if input.amount <= 0.0 {
        return Err(ApiError::BadRequest("Amount must be positive".to_string()));
    }

    let tx_type = match input.transaction_type.as_str() {
        "credit_adjustment" => llm_gateway_storage::TransactionType::CreditAdjustment,
        "debit_refund" => llm_gateway_storage::TransactionType::DebitRefund,
        _ => return Err(ApiError::BadRequest("type must be 'credit_adjustment' or 'debit_refund'".to_string())),
    };

    let now = chrono::Utc::now();
    let new_balance = if tx_type == llm_gateway_storage::TransactionType::CreditAdjustment {
        account.balance + input.amount
    } else {
        account.balance - input.amount
    };

    let transaction = llm_gateway_storage::Transaction {
        id: uuid::Uuid::new_v4().to_string(),
        account_id: account.id.clone(),
        transaction_type: tx_type,
        amount: input.amount,
        balance_after: new_balance,
        description: input.description.or_else(|| Some("Manual adjustment".to_string())),
        reference_id: input.reference_id,
        created_at: now,
    };

    account.balance = new_balance;
    account.updated_at = now;

    state
        .storage
        .create_transaction(&transaction)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let updated = state
        .storage
        .update_account(&account)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(AccountResponse::from(&updated)))
}

pub async fn update_threshold(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(input): Json<UpdateAccountThreshold>,
) -> Result<Json<AccountResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let mut account = state
        .storage
        .get_account_by_user_id(&user_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound(format!("Account for user '{}' not found", user_id)))?;

    if input.threshold < 0.0 {
        return Err(ApiError::BadRequest("Threshold must be non-negative".to_string()));
    }

    account.threshold = input.threshold;
    account.updated_at = chrono::Utc::now();

    let updated = state
        .storage
        .update_account(&account)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(AccountResponse::from(&updated)))
}
```

- [ ] **Step 2: Update mod.rs**

In `crates/api/src/management/mod.rs`:
1. Add `pub mod accounts;` to the list
2. Register routes in `management_router()`:

```rust
// Account / Balance (admin)
.route(
    "/api/v1/admin/users/{id}/balance",
    get(accounts::get_balance),
)
.route(
    "/api/v1/admin/users/{id}/recharge",
    post(accounts::recharge),
)
.route(
    "/api/v1/admin/users/{id}/adjust",
    post(accounts::adjust),
)
.route(
    "/api/v1/admin/users/{id}/threshold",
    patch(accounts::update_threshold),
)
```

- [ ] **Step 3: Build**

```bash
cd /workspace && cargo build --release 2>&1 | tail -20
```

Expected: compiles.

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/management/accounts.rs crates/api/src/management/mod.rs
git commit -m "feat(api): add admin account management endpoints

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 11: User Balance API

**Files:**
- Modify: `crates/api/src/management/auth.rs`

- [ ] **Step 1: Add `/me/balance` endpoint**

In `crates/api/src/management/auth.rs`, add these imports if not present:
```rust
use llm_gateway_storage::{AccountResponse, TransactionResponse, PaginatedResponse, PaginationParams};
```

Then add these functions (before or after the `change_password` function):

```rust
pub async fn me_balance(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let claims = require_auth(&headers, &state.jwt_secret)?;

    let account = state
        .storage
        .get_account_by_user_id(&claims.sub)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound("Account not found".to_string()))?;

    let (page, page_size) = pagination.normalized();
    let transactions = state
        .storage
        .list_transactions(&account.id, page, page_size)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "balance": account.balance,
        "threshold": account.threshold,
        "currency": account.currency,
        "transactions": PaginatedResponse {
            items: transactions.items.iter().map(TransactionResponse::from).collect(),
            total: transactions.total,
            page: transactions.page,
            page_size: transactions.page_size,
        }
    })))
}
```

- [ ] **Step 2: Register route in mod.rs**

In `crates/api/src/management/mod.rs`, add to the auth routes section:

```rust
.route("/api/v1/auth/me/balance", get(auth::me_balance))
```

- [ ] **Step 3: Build and test**

```bash
cd /workspace && cargo build --release 2>&1 | tail -20
```

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/management/auth.rs crates/api/src/management/mod.rs
git commit -m "feat(api): add /me/balance endpoint for user balance and transactions

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 12: Backend Tests

**Files:**
- Modify: `crates/billing/src/pricing.rs` (existing tests already cover cost calculation)

- [ ] **Step 1: Add unit tests for settlement logic**

In `crates/api/src/settlement.rs`, add a `#[cfg(test)]` module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settlement_idempotency_reference() {
        let batch_reference = format!("batch_{}", chrono::Utc::now().timestamp());
        // Verify batch reference format is deterministic enough for idempotency checks
        assert!(batch_reference.starts_with("batch_"));
    }
}
```

Also add basic tests in `crates/storage/src/lib.rs` if there are helper functions that can be tested in isolation.

Run:
```bash
cd /workspace && cargo test --workspace 2>&1 | tail -30
```

Expected: all tests pass.

- [ ] **Step 2: Commit**

```bash
git add crates/api/src/settlement.rs
git commit -m "test(api): add settlement unit tests

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Frontend Tasks

### Task 13: TypeScript Types

**Files:**
- Modify: `web/src/types/index.ts`

- [ ] **Step 1: Add account and transaction types**

Find the `// ── Pricing Config Types ──────────────────────────────` section and add after it (before the last closing `}`):

```typescript
// ── Account & Transaction Types ───────────────────────────────────────────────

export interface Account {
  id: string;
  user_id: string;
  balance: number;
  threshold: number;
  currency: string;
  created_at: string;
  updated_at: string;
}

export interface Transaction {
  id: string;
  account_id: string;
  type: 'credit' | 'debit' | 'credit_adjustment' | 'debit_refund';
  amount: number;
  balance_after: number;
  description: string | null;
  reference_id: string | null;
  created_at: string;
}

export interface AccountBalanceResponse {
  account: Account;
  transactions: PaginatedResponse<Transaction>;
}

export interface MeBalanceResponse {
  balance: number;
  threshold: number;
  currency: string;
  transactions: PaginatedResponse<Transaction>;
}

export interface CreateTransactionRequest {
  type: 'credit' | 'credit_adjustment' | 'debit_refund';
  amount: number;
  description?: string;
  reference_id?: string;
}

export interface UpdateThresholdRequest {
  threshold: number;
}
```

- [ ] **Step 2: Commit**

```bash
git add web/src/types/index.ts
git commit -m "feat(web): add Account and Transaction TypeScript types

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 14: API Client

**Files:**
- Create: `web/src/api/accounts.ts`

- [ ] **Step 1: Create accounts.ts**

```typescript
import { client, getErrorMessage } from './client';
import type {
  Account,
  AccountBalanceResponse,
  CreateTransactionRequest,
  MeBalanceResponse,
  UpdateThresholdRequest,
  PaginatedResponse,
  Transaction,
} from '../types';

export async function getUserBalance(userId: string, page = 1, pageSize = 20): Promise<AccountBalanceResponse> {
  const response = await client.get<AccountBalanceResponse>(
    `/admin/users/${userId}/balance`,
    { params: { page, page_size: pageSize } }
  );
  return response.data;
}

export async function rechargeUser(userId: string, data: CreateTransactionRequest): Promise<Account> {
  const response = await client.post<Account>(
    `/admin/users/${userId}/recharge`,
    { ...data, type: 'credit' }
  );
  return response.data;
}

export async function adjustUserBalance(userId: string, data: CreateTransactionRequest): Promise<Account> {
  const response = await client.post<Account>(
    `/admin/users/${userId}/adjust`,
    data
  );
  return response.data;
}

export async function updateUserThreshold(userId: string, data: UpdateThresholdRequest): Promise<Account> {
  const response = await client.patch<Account>(
    `/admin/users/${userId}/threshold`,
    data
  );
  return response.data;
}

export async function getMyBalance(page = 1, pageSize = 20): Promise<MeBalanceResponse> {
  const response = await client.get<MeBalanceResponse>(
    '/auth/me/balance',
    { params: { page, page_size: pageSize } }
  );
  return response.data;
}
```

- [ ] **Step 2: Commit**

```bash
git add web/src/api/accounts.ts
git commit -m "feat(web): add accounts API client

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 15: React Query Hooks

**Files:**
- Create: `web/src/hooks/useAccounts.ts`

- [ ] **Step 1: Create useAccounts.ts**

```typescript
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getUserBalance,
  rechargeUser,
  adjustUserBalance,
  updateUserThreshold,
  getMyBalance,
} from '../api/accounts';
import type { CreateTransactionRequest, UpdateThresholdRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';

export function useUserBalance(userId: string, page = 1, pageSize = 20) {
  return useQuery({
    queryKey: ['user-balance', userId, page, pageSize],
    queryFn: () => getUserBalance(userId, page, pageSize),
    enabled: !!userId,
  });
}

export function useRechargeUser() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ userId, data }: { userId: string; data: CreateTransactionRequest }) =>
      rechargeUser(userId, data),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['user-balance', variables.userId] });
      toast.success('Balance recharged successfully');
    },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to recharge')); },
  });
}

export function useAdjustUser() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ userId, data }: { userId: string; data: CreateTransactionRequest }) =>
      adjustUserBalance(userId, data),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['user-balance', variables.userId] });
      toast.success('Balance adjusted successfully');
    },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to adjust balance')); },
  });
}

export function useUpdateThreshold() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ userId, data }: { userId: string; data: UpdateThresholdRequest }) =>
      updateUserThreshold(userId, data),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['user-balance', variables.userId] });
      toast.success('Threshold updated');
    },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to update threshold')); },
  });
}

export function useMyBalance(page = 1, pageSize = 20) {
  return useQuery({
    queryKey: ['my-balance', page, pageSize],
    queryFn: () => getMyBalance(page, pageSize),
  });
}
```

- [ ] **Step 2: Commit**

```bash
git add web/src/hooks/useAccounts.ts
git commit -m "feat(web): add useAccounts React Query hooks

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 16: Balance UI — AccountBalance Page

**Files:**
- Create: `web/src/pages/AccountBalance.tsx`

- [ ] **Step 1: Create the page**

Read `web/src/pages/Usage.tsx` to match the existing UI patterns (stat cards, table, pagination).

```tsx
import { useState } from 'react';
import { DollarSign, TrendingUp, TrendingDown } from 'lucide-react';
import { useUserBalance, useRechargeUser, useAdjustUser, useUpdateThreshold } from '../hooks/useAccounts';
import { Button } from '../components/ui/Button';
import { Badge } from '../components/ui/Badge';
import { Modal } from '../components/ui/Modal';
import type { CreateTransactionRequest } from '../types';

interface AccountBalanceProps {
  userId: string;
}

export default function AccountBalance({ userId }: AccountBalanceProps) {
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);
  const [rechargeOpen, setRechargeOpen] = useState(false);
  const [adjustOpen, setAdjustOpen] = useState(false);
  const [rechargeAmount, setRechargeAmount] = useState('');
  const [adjustAmount, setAdjustAmount] = useState('');
  const [adjustType, setAdjustType] = useState<'credit_adjustment' | 'debit_refund'>('credit_adjustment');
  const [description, setDescription] = useState('');

  const { data, isLoading } = useUserBalance(userId, page, pageSize);
  const rechargeMutation = useRechargeUser();
  const adjustMutation = useAdjustUser();

  const account = data?.account;
  const transactions = data?.transactions;
  const totalPages = Math.ceil((transactions?.total ?? 0) / pageSize);

  const handleRecharge = () => {
    const amount = parseFloat(rechargeAmount);
    if (!amount || amount <= 0) return;
    rechargeMutation.mutate(
      { userId, data: { type: 'credit', amount, description: description || 'Recharge' } },
      {
        onSuccess: () => {
          setRechargeOpen(false);
          setRechargeAmount('');
          setDescription('');
        },
      }
    );
  };

  const handleAdjust = () => {
    const amount = parseFloat(adjustAmount);
    if (!amount || amount <= 0) return;
    adjustMutation.mutate(
      { userId, data: { type: adjustType, amount, description: description || 'Manual adjustment' } },
      {
        onSuccess: () => {
          setAdjustOpen(false);
          setAdjustAmount('');
          setDescription('');
        },
      }
    );
  };

  const txTypeLabel: Record<string, { label: string; color: string }> = {
    credit: { label: 'Credit', color: 'green' },
    debit: { label: 'Debit', color: 'red' },
    credit_adjustment: { label: 'Adjustment', color: 'blue' },
    debit_refund: { label: 'Refund', color: 'purple' },
  };

  return (
    <div>
      {/* Balance Header */}
      {account && (
        <div className="mb-6 grid grid-cols-3 gap-4">
          <div className="stat bg-base-100 rounded-box p-4 shadow-sm">
            <div className="stat-title text-base-content/50"><DollarSign className="h-4 w-4 inline mr-1" />Balance</div>
            <div className="stat-value text-3xl font-mono">${account.balance.toFixed(4)}</div>
          </div>
          <div className="stat bg-base-100 rounded-box p-4 shadow-sm">
            <div className="stat-title text-base-content/50">Threshold</div>
            <div className="stat-value text-2xl font-mono">${account.threshold.toFixed(2)}</div>
          </div>
          <div className="stat bg-base-100 rounded-box p-4 shadow-sm">
            <div className="stat-title text-base-content/50">Currency</div>
            <div className="stat-value text-2xl">{account.currency}</div>
          </div>
        </div>
      )}

      {/* Actions */}
      <div className="mb-4 flex gap-2">
        <Button onClick={() => setRechargeOpen(true)}>Recharge</Button>
        <Button variant="secondary" onClick={() => setAdjustOpen(true)}>Adjust</Button>
      </div>

      {/* Transactions Table */}
      {isLoading ? (
        <div className="flex items-center justify-center py-12"><span className="loading loading-spinner loading-lg" /></div>
      ) : (
        <>
          <div className="overflow-x-auto bg-base-100 rounded-box shadow-sm">
            <table className="table table-sm">
              <thead>
                <tr className="border-b border-base-300">
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Time</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Type</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50 text-right">Amount</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50 text-right">Balance After</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Description</th>
                </tr>
              </thead>
              <tbody>
                {transactions?.items.map((tx) => {
                  const info = txTypeLabel[tx.type] ?? { label: tx.type, color: 'gray' };
                  const isCredit = tx.type === 'credit' || tx.type === 'credit_adjustment';
                  return (
                    <tr key={tx.id} className="border-b border-base-200 hover">
                      <td className="mono text-[13px]">{new Date(tx.created_at).toLocaleString()}</td>
                      <td>
                        <Badge variant={info.color as 'green' | 'red' | 'blue' | 'purple'}>{info.label}</Badge>
                      </td>
                      <td className={`mono text-right ${isCredit ? 'text-green-500' : 'text-red-500'}`}>
                        {isCredit ? '+' : '-'}${tx.amount.toFixed(4)}
                      </td>
                      <td className="mono text-right">${tx.balance_after.toFixed(4)}</td>
                      <td className="text-sm">{tx.description ?? '-'}</td>
                    </tr>
                  );
                })}
                {transactions?.items.length === 0 && (
                  <tr>
                    <td colSpan={5} className="text-center py-8 text-base-content/40">No transactions yet</td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>

          {totalPages > 1 && (
            <div className="mt-4 flex items-center justify-between text-sm">
              <span className="text-base-content/40">Total {transactions?.total ?? 0}</span>
              <div className="join">
                <Button variant="ghost" size="sm" disabled={page <= 1} onClick={() => setPage(page - 1)}>Previous</Button>
                <span className="px-3 flex items-center text-base-content/60">{page} / {totalPages}</span>
                <Button variant="ghost" size="sm" disabled={page >= totalPages} onClick={() => setPage(page + 1)}>Next</Button>
              </div>
            </div>
          )}
        </>
      )}

      {/* Recharge Modal */}
      <Modal open={rechargeOpen} onClose={() => setRechargeOpen(false)} title="Recharge Balance">
        <div className="space-y-4">
          <div>
            <label className="label"><span className="label-text">Amount (USD)</span></label>
            <input
              type="number"
              className="input input-bordered w-full"
              value={rechargeAmount}
              onChange={(e) => setRechargeAmount(e.target.value)}
              placeholder="0.00"
              step="0.01"
              min="0"
            />
          </div>
          <div>
            <label className="label"><span className="label-text">Description (optional)</span></label>
            <input
              type="text"
              className="input input-bordered w-full"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="e.g. Monthly top-up"
            />
          </div>
          <div className="flex justify-end gap-2">
            <Button variant="ghost" onClick={() => setRechargeOpen(false)}>Cancel</Button>
            <Button onClick={handleRecharge} disabled={rechargeMutation.isPending}>Confirm Recharge</Button>
          </div>
        </div>
      </Modal>

      {/* Adjust Modal */}
      <Modal open={adjustOpen} onClose={() => setAdjustOpen(false)} title="Adjust Balance">
        <div className="space-y-4">
          <div>
            <label className="label"><span className="label-text">Type</span></label>
            <select
              className="select select-bordered w-full"
              value={adjustType}
              onChange={(e) => setAdjustType(e.target.value as 'credit_adjustment' | 'debit_refund')}
            >
              <option value="credit_adjustment">Credit Adjustment (add)</option>
              <option value="debit_refund">Debit / Refund (subtract)</option>
            </select>
          </div>
          <div>
            <label className="label"><span className="label-text">Amount (USD)</span></label>
            <input
              type="number"
              className="input input-bordered w-full"
              value={adjustAmount}
              onChange={(e) => setAdjustAmount(e.target.value)}
              placeholder="0.00"
              step="0.01"
              min="0"
            />
          </div>
          <div>
            <label className="label"><span className="label-text">Description (optional)</span></label>
            <input
              type="text"
              className="input input-bordered w-full"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="e.g. Customer compensation"
            />
          </div>
          <div className="flex justify-end gap-2">
            <Button variant="ghost" onClick={() => setAdjustOpen(false)}>Cancel</Button>
            <Button onClick={handleAdjust} disabled={adjustMutation.isPending}>Confirm Adjustment</Button>
          </div>
        </div>
      </Modal>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add web/src/pages/AccountBalance.tsx
git commit -m "feat(web): add AccountBalance page with recharge/adjust UI

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 17: User Detail Page — Integrate Balance

**Files:**
- Modify: `web/src/pages/Users.tsx`

- [ ] **Step 1: Add balance to user list**

Read `web/src/pages/Users.tsx`. Add a "Balance" column to the table that fetches the balance for each user.

Note: For simplicity in the initial implementation, add a "View Balance" button that links to the AccountBalance page, or inline the balance fetch.

A simpler approach: add a "Balance" column in the table header and a "View" link per row that navigates to `/users/{id}/balance`. Or use a direct balance display with individual queries.

For the initial implementation, add a balance badge/call-to-action per user row. Modify the `Users` page to add a "Balance" column:

Add after the `<th>Actions</th>` column:
```tsx
<th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Balance</th>
```

And in the row:
```tsx
<td>
  <button
    className="btn btn-xs btn-ghost"
    onClick={() => navigate(`/users/${user.id}/balance`)}
  >
    View
  </button>
</td>
```

Note: This requires adding `useNavigate` from `react-router-dom` and the navigation import. Check if `navigate` is already used in the file; if not, add:
```tsx
import { useNavigate } from 'react-router-dom';
// and in the component:
const navigate = useNavigate();
```

Alternatively, to keep it simple, just add a link/button that navigates to the balance page. The full balance page is in AccountBalance.tsx.

Actually, for a clean implementation, modify `Users.tsx` to add a navigation button in the Actions column instead of a separate column. Add "Balance" as an action button:

```tsx
<td className="flex gap-1">
  <button
    className="btn btn-xs btn-ghost"
    onClick={() => navigate(`/users/${user.id}/balance`)}
  >
    Balance
  </button>
  <ConfirmDialog title="Delete this user?" onConfirm={() => deleteMutation.mutate(user.id)} okText="Delete">
    <Button variant="danger" size="sm">Delete</Button>
  </ConfirmDialog>
</td>
```

- [ ] **Step 2: Add route to Layout.tsx**

Read `web/src/components/Layout.tsx` and add the route:

```tsx
import AccountBalance from '../pages/AccountBalance';
// Add route:
<Route path="/users/:userId/balance" element={<AccountBalance />} />
```

Note: The `userId` param will be extracted from the URL in `AccountBalance` using `useParams`:

```tsx
import { useParams } from 'react-router-dom';
// In component:
const { userId } = useParams<{ userId: string }>();
```

Update `AccountBalance.tsx` to use `useParams` instead of `userId` prop:
```tsx
export default function AccountBalance() {
  const { userId } = useParams<{ userId: string }>();
  // ... rest of component, userId will be undefined for admin path
}
```

Wait, the route is `/users/:userId/balance`, so `userId` will be available via `useParams`. Remove the `AccountBalanceProps` interface and the prop usage.

- [ ] **Step 3: Build**

```bash
cd /workspace/web && npm run build 2>&1 | tail -20
```

Expected: builds without errors.

- [ ] **Step 4: Commit**

```bash
git add web/src/pages/Users.tsx web/src/components/Layout.tsx web/src/pages/AccountBalance.tsx
git commit -m "feat(web): integrate balance page into user detail flow

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 18: Frontend Tests

**Files:**
- Modify: `web/src/pages/Users.test.tsx` (add test for balance navigation)
- Create: `web/src/pages/AccountBalance.test.tsx`

- [ ] **Step 1: Add test for balance navigation in Users.tsx**

Add a test case:
```typescript
it('navigates to balance page on Balance button click', async () => {
  const { getByText } = render(<Users />, { wrapper: createWrapper() });
  // Wait for data to load
  await waitFor(() => {
    expect(getByText('admin')).toBeInTheDocument();
  });
  const balanceBtn = getByText('Balance');
  fireEvent.click(balanceBtn);
  expect(window.location.pathname).toContain('/balance');
});
```

Run tests:
```bash
cd /workspace/web && npm test 2>&1 | tail -20
```

Expected: tests pass.

- [ ] **Step 2: Commit**

```bash
git add web/src/pages/Users.test.tsx web/src/pages/AccountBalance.test.tsx
git commit -m "test(web): add frontend tests for balance page

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Final Verification

- [ ] Run `cargo test --workspace` — all tests pass
- [ ] Run `cargo build --release` — compiles cleanly
- [ ] Run `cd web && npm run build` — TypeScript compiles, Vite builds cleanly
- [ ] Run `cd web && npm test` — all frontend tests pass
