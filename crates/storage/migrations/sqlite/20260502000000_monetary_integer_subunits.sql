-- Convert monetary REAL columns to INTEGER subunits
-- Monetary values use 8 decimal places: ROUND(column * 100000000)
-- Markup ratios use basis points: ROUND(column * 10000)

-- ============================================================
-- api_keys: budget_monthly REAL -> INTEGER
-- ============================================================
ALTER TABLE api_keys RENAME TO api_keys_old;

CREATE TABLE api_keys (
    id               TEXT PRIMARY KEY,
    name             TEXT NOT NULL,
    key_hash         TEXT NOT NULL UNIQUE,
    rate_limit       INTEGER,
    budget_monthly   INTEGER,
    enabled          BOOLEAN NOT NULL DEFAULT 1,
    created_by       TEXT,
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL,
    key_prefix       TEXT,
    model_fallback_id TEXT REFERENCES model_fallbacks(id)
);

INSERT INTO api_keys (id, name, key_hash, rate_limit, budget_monthly, enabled, created_by, created_at, updated_at, key_prefix, model_fallback_id)
SELECT id, name, key_hash, rate_limit,
       CASE WHEN budget_monthly IS NULL THEN NULL ELSE ROUND(budget_monthly * 100000000) END,
       enabled, created_by, created_at, updated_at, key_prefix, model_fallback_id
FROM api_keys_old;

DROP TABLE api_keys_old;

-- ============================================================
-- usage_records: cost REAL -> INTEGER
-- ============================================================
ALTER TABLE usage_records RENAME TO usage_records_old;

CREATE TABLE usage_records (
    id                    TEXT PRIMARY KEY,
    key_id                TEXT NOT NULL REFERENCES api_keys(id),
    model_name            TEXT NOT NULL,
    provider_id           TEXT NOT NULL,
    channel_id            TEXT REFERENCES channels(id),
    protocol              TEXT NOT NULL,
    input_tokens          INTEGER,
    output_tokens         INTEGER,
    cost                  INTEGER NOT NULL,
    created_at            TEXT NOT NULL,
    cache_read_tokens     INTEGER,
    user_id               TEXT,
    cache_creation_tokens INTEGER
);

INSERT INTO usage_records (id, key_id, model_name, provider_id, channel_id, protocol, input_tokens, output_tokens, cost, created_at, cache_read_tokens, user_id, cache_creation_tokens)
SELECT id, key_id, model_name, provider_id, channel_id, protocol, input_tokens, output_tokens,
       ROUND(cost * 100000000),
       created_at, cache_read_tokens, user_id, cache_creation_tokens
FROM usage_records_old;

DROP TABLE usage_records_old;

CREATE INDEX idx_usage_key_date ON usage_records(key_id, created_at);
CREATE INDEX idx_usage_model_date ON usage_records(model_name, created_at);

-- ============================================================
-- channels: markup_ratio REAL -> INTEGER, balance REAL -> INTEGER
-- ============================================================
ALTER TABLE channels RENAME TO channels_old;

CREATE TABLE channels (
    id                 TEXT PRIMARY KEY,
    provider_id        TEXT NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
    name               TEXT NOT NULL,
    api_key            TEXT NOT NULL,
    base_url           TEXT,
    priority           INTEGER NOT NULL DEFAULT 0,
    pricing_policy_id  TEXT REFERENCES pricing_policies(id),
    markup_ratio       INTEGER NOT NULL DEFAULT 10000,
    rpm_limit          INTEGER,
    tpm_limit          INTEGER,
    balance            INTEGER,
    weight             INTEGER NOT NULL DEFAULT 100,
    enabled            BOOLEAN NOT NULL DEFAULT 1,
    created_at         TEXT NOT NULL,
    updated_at         TEXT NOT NULL
);

INSERT INTO channels (id, provider_id, name, api_key, base_url, priority, pricing_policy_id, markup_ratio, rpm_limit, tpm_limit, balance, weight, enabled, created_at, updated_at)
SELECT id, provider_id, name, api_key, base_url, priority, pricing_policy_id,
       ROUND(markup_ratio * 10000),
       rpm_limit, tpm_limit,
       CASE WHEN balance IS NULL THEN NULL ELSE ROUND(balance * 100000000) END,
       weight, enabled, created_at, updated_at
FROM channels_old;

DROP TABLE channels_old;

CREATE INDEX idx_channels_provider ON channels(provider_id);

-- ============================================================
-- channel_models: markup_ratio REAL -> INTEGER
-- ============================================================
ALTER TABLE channel_models RENAME TO channel_models_old;

CREATE TABLE channel_models (
    id                   TEXT PRIMARY KEY,
    channel_id           TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    model_id             TEXT NOT NULL REFERENCES models(id) ON DELETE CASCADE,
    upstream_model_name  TEXT,
    priority_override    INTEGER,
    markup_ratio         INTEGER NOT NULL DEFAULT 10000,
    enabled              INTEGER NOT NULL DEFAULT 1,
    created_at           TEXT NOT NULL,
    updated_at           TEXT NOT NULL,
    cache_read_price     REAL,
    pricing_policy_id    TEXT REFERENCES pricing_policies(id),
    UNIQUE(channel_id, model_id)
);

INSERT INTO channel_models (id, channel_id, model_id, upstream_model_name, priority_override, markup_ratio, enabled, created_at, updated_at, cache_read_price, pricing_policy_id)
SELECT id, channel_id, model_id, upstream_model_name, priority_override,
       ROUND(markup_ratio * 10000),
       enabled, created_at, updated_at, cache_read_price, pricing_policy_id
FROM channel_models_old;

DROP TABLE channel_models_old;

CREATE INDEX idx_channel_models_model ON channel_models(model_id);
CREATE INDEX idx_channel_models_channel ON channel_models(channel_id);

-- ============================================================
-- accounts: balance REAL -> INTEGER, threshold REAL -> INTEGER
-- ============================================================
ALTER TABLE accounts RENAME TO accounts_old;

CREATE TABLE accounts (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    balance     INTEGER NOT NULL DEFAULT 0,
    threshold   INTEGER NOT NULL DEFAULT 100000000,
    currency    TEXT NOT NULL DEFAULT 'USD',
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

INSERT INTO accounts (id, user_id, balance, threshold, currency, created_at, updated_at)
SELECT id, user_id,
       ROUND(balance * 100000000),
       ROUND(threshold * 100000000),
       currency, created_at, updated_at
FROM accounts_old;

DROP TABLE accounts_old;

CREATE INDEX idx_accounts_user_id ON accounts(user_id);

-- ============================================================
-- transactions: amount REAL -> INTEGER, balance_after REAL -> INTEGER
-- ============================================================
ALTER TABLE transactions RENAME TO transactions_old;

CREATE TABLE transactions (
    id             TEXT PRIMARY KEY,
    account_id     TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    type           TEXT NOT NULL CHECK(type IN ('credit','debit','credit_adjustment','debit_refund')),
    amount         INTEGER NOT NULL CHECK(amount > 0),
    balance_after  INTEGER NOT NULL,
    description    TEXT,
    reference_id   TEXT,
    created_at     TEXT NOT NULL
);

INSERT INTO transactions (id, account_id, type, amount, balance_after, description, reference_id, created_at)
SELECT id, account_id, type,
       ROUND(amount * 100000000),
       ROUND(balance_after * 100000000),
       description, reference_id, created_at
FROM transactions_old;

DROP TABLE transactions_old;

CREATE INDEX idx_transactions_account_id ON transactions(account_id);
CREATE INDEX idx_transactions_type ON transactions(type);
CREATE INDEX idx_transactions_reference_id ON transactions(reference_id);
CREATE INDEX idx_transactions_created_at ON transactions(created_at);
