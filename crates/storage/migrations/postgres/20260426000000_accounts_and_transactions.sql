-- Accounts and Transactions tables
CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    balance DOUBLE PRECISION NOT NULL DEFAULT 0,
    threshold DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    currency TEXT NOT NULL DEFAULT 'USD',
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_accounts_user_id ON accounts(user_id);

CREATE TABLE IF NOT EXISTS transactions (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    type TEXT NOT NULL CHECK(type IN ('credit','debit','credit_adjustment','debit_refund')),
    amount DOUBLE PRECISION NOT NULL CHECK(amount > 0),
    balance_after DOUBLE PRECISION NOT NULL,
    description TEXT,
    reference_id TEXT,
    created_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_transactions_account_id ON transactions(account_id);
CREATE INDEX IF NOT EXISTS idx_transactions_type ON transactions(type);
CREATE INDEX IF NOT EXISTS idx_transactions_reference_id ON transactions(reference_id);
CREATE INDEX IF NOT EXISTS idx_transactions_created_at ON transactions(created_at);