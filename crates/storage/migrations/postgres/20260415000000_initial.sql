-- Migration: PostgreSQL schema for LLM Gateway
-- Date: 2026-04-15

-- API Keys
CREATE TABLE IF NOT EXISTS api_keys (
    id             TEXT PRIMARY KEY,
    name           TEXT NOT NULL,
    key_hash       TEXT NOT NULL UNIQUE,
    rate_limit     INTEGER,
    budget_monthly REAL,
    enabled        BOOLEAN NOT NULL DEFAULT true,
    created_by     TEXT,
    created_at     TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at     TIMESTAMP WITH TIME ZONE NOT NULL
);

-- Providers
CREATE TABLE IF NOT EXISTS providers (
    id                 TEXT PRIMARY KEY,
    name               TEXT NOT NULL,
    base_url           TEXT,
    endpoints          TEXT,
    enabled            BOOLEAN NOT NULL DEFAULT true,
    created_at         TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at         TIMESTAMP WITH TIME ZONE NOT NULL
);

-- Models
CREATE TABLE IF NOT EXISTS models (
    id               TEXT PRIMARY KEY,
    name             TEXT NOT NULL UNIQUE,
    provider_id      TEXT NOT NULL REFERENCES providers(id),
    model_type       TEXT,
    pricing_policy_id TEXT REFERENCES pricing_policies(id),
    billing_type     TEXT NOT NULL DEFAULT 'per_token',
    input_price      REAL NOT NULL DEFAULT 0,
    output_price     REAL NOT NULL DEFAULT 0,
    request_price    REAL NOT NULL DEFAULT 0,
    enabled          BOOLEAN NOT NULL DEFAULT true,
    created_at       TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_models_provider ON models(provider_id);
CREATE INDEX IF NOT EXISTS idx_models_enabled ON models(enabled);

-- Key-Model Rate Limits
CREATE TABLE IF NOT EXISTS key_model_rate_limits (
    key_id     TEXT NOT NULL REFERENCES api_keys(id),
    model_id   TEXT NOT NULL REFERENCES models(id),
    rpm        INTEGER NOT NULL,
    tpm        INTEGER NOT NULL,
    PRIMARY KEY (key_id, model_id)
);

-- Usage Records
CREATE TABLE IF NOT EXISTS usage_records (
    id            TEXT PRIMARY KEY,
    key_id        TEXT NOT NULL REFERENCES api_keys(id),
    model_name    TEXT NOT NULL,
    provider_id   TEXT NOT NULL,
    channel_id    TEXT,
    protocol      TEXT NOT NULL,
    input_tokens  INTEGER,
    output_tokens INTEGER,
    cost          REAL NOT NULL,
    created_at    TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_usage_key_date ON usage_records(key_id, created_at);
CREATE INDEX IF NOT EXISTS idx_usage_model_date ON usage_records(model_name, created_at);

-- Audit Logs
CREATE TABLE IF NOT EXISTS audit_logs (
    id            TEXT PRIMARY KEY,
    key_id        TEXT NOT NULL REFERENCES api_keys(id),
    model_name    TEXT NOT NULL,
    provider_id   TEXT NOT NULL,
    channel_id    TEXT,
    protocol      TEXT NOT NULL,
    request_body  TEXT NOT NULL,
    response_body TEXT NOT NULL,
    status_code   INTEGER NOT NULL,
    latency_ms    BIGINT NOT NULL,
    input_tokens  INTEGER,
    output_tokens INTEGER,
    created_at    TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_audit_key_date ON audit_logs(key_id, created_at);
CREATE INDEX IF NOT EXISTS idx_audit_model_date ON audit_logs(model_name, created_at);

-- Rate Limit Counters
CREATE TABLE IF NOT EXISTS rate_limit_counters (
    key_id     TEXT NOT NULL,
    model_name TEXT NOT NULL,
    window     TEXT NOT NULL,
    count      INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (key_id, model_name, window)
);

-- Settings
CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Users
CREATE TABLE IF NOT EXISTS users (
    id           TEXT PRIMARY KEY,
    username     TEXT NOT NULL UNIQUE,
    password     TEXT NOT NULL,
    role         TEXT NOT NULL DEFAULT 'user' CHECK(role IN ('admin', 'user')),
    enabled      BOOLEAN NOT NULL DEFAULT true,
    refresh_token TEXT,
    created_at   TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at   TIMESTAMP WITH TIME ZONE NOT NULL
);

-- Channels
CREATE TABLE IF NOT EXISTS channels (
    id               TEXT PRIMARY KEY,
    provider_id      TEXT NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
    name             TEXT NOT NULL,
    api_key          TEXT NOT NULL,
    base_url         TEXT,
    priority         INTEGER NOT NULL DEFAULT 0,
    pricing_policy_id TEXT REFERENCES pricing_policies(id),
    markup_ratio     REAL NOT NULL DEFAULT 1.0,
    enabled          BOOLEAN NOT NULL DEFAULT true,
    rpm_limit        INTEGER,
    tpm_limit        INTEGER,
    balance          REAL,
    weight           INTEGER DEFAULT 100,
    created_at       TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at       TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_channels_provider ON channels(provider_id);
CREATE INDEX IF NOT EXISTS idx_channels_enabled ON channels(enabled);

-- Pricing Policies
CREATE TABLE IF NOT EXISTS pricing_policies (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    billing_type TEXT NOT NULL,
    config      TEXT NOT NULL,
    created_at  TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at  TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_pricing_policies_type ON pricing_policies(billing_type);

-- Channel Models (Junction Table)
CREATE TABLE IF NOT EXISTS channel_models (
    id                  TEXT PRIMARY KEY,
    channel_id          TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    model_id            TEXT NOT NULL REFERENCES models(id) ON DELETE CASCADE,
    upstream_model_name TEXT NOT NULL,
    priority_override   INTEGER,
    cost_policy_id      TEXT REFERENCES pricing_policies(id),
    markup_ratio        REAL NOT NULL DEFAULT 1.0,
    billing_type        TEXT,
    input_price         REAL,
    output_price        REAL,
    request_price       REAL,
    enabled             BOOLEAN NOT NULL DEFAULT true,
    created_at          TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at          TIMESTAMP WITH TIME ZONE NOT NULL,
    UNIQUE(channel_id, model_id)
);

CREATE INDEX IF NOT EXISTS idx_channel_models_model ON channel_models(model_id);
CREATE INDEX IF NOT EXISTS idx_channel_models_channel ON channel_models(channel_id);