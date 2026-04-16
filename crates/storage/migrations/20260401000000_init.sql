CREATE TABLE IF NOT EXISTS api_keys (
    id             TEXT PRIMARY KEY,
    name           TEXT NOT NULL,
    key_hash       TEXT NOT NULL UNIQUE,
    rate_limit     INTEGER,
    budget_monthly REAL,
    enabled        BOOLEAN NOT NULL DEFAULT 1,
    created_by     TEXT,
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS providers (
    id                 TEXT PRIMARY KEY,
    name               TEXT NOT NULL,
    api_key            TEXT NOT NULL,
    openai_base_url    TEXT,
    anthropic_base_url TEXT,
    enabled            BOOLEAN NOT NULL DEFAULT 1,
    created_at         TEXT NOT NULL,
    updated_at         TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS models (
    name          TEXT PRIMARY KEY,
    provider_id   TEXT NOT NULL REFERENCES providers(id),
    billing_type  TEXT NOT NULL CHECK(billing_type IN ('token', 'request')),
    input_price   REAL NOT NULL DEFAULT 0,
    output_price  REAL NOT NULL DEFAULT 0,
    request_price REAL NOT NULL DEFAULT 0,
    enabled       BOOLEAN NOT NULL DEFAULT 1,
    created_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS key_model_rate_limits (
    key_id     TEXT NOT NULL REFERENCES api_keys(id),
    model_name TEXT NOT NULL REFERENCES models(name),
    rpm        INTEGER NOT NULL,
    tpm        INTEGER NOT NULL,
    PRIMARY KEY (key_id, model_name)
);

CREATE TABLE IF NOT EXISTS usage_records (
    id            TEXT PRIMARY KEY,
    key_id        TEXT NOT NULL REFERENCES api_keys(id),
    model_name    TEXT NOT NULL,
    provider_id   TEXT NOT NULL,
    protocol      TEXT NOT NULL,
    input_tokens  INTEGER,
    output_tokens INTEGER,
    cost          REAL NOT NULL,
    created_at    TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_usage_key_date ON usage_records(key_id, created_at);
CREATE INDEX IF NOT EXISTS idx_usage_model_date ON usage_records(model_name, created_at);

CREATE TABLE IF NOT EXISTS audit_logs (
    id            TEXT PRIMARY KEY,
    key_id        TEXT NOT NULL REFERENCES api_keys(id),
    model_name    TEXT NOT NULL,
    provider_id   TEXT NOT NULL,
    channel_id    TEXT,
    protocol      TEXT NOT NULL,
    stream       INTEGER NOT NULL DEFAULT 0,
    request_body  TEXT NOT NULL,
    response_body TEXT NOT NULL,
    status_code   INTEGER NOT NULL,
    latency_ms    INTEGER NOT NULL,
    input_tokens  INTEGER,
    output_tokens INTEGER,
    created_at    TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_audit_key_date ON audit_logs(key_id, created_at);
CREATE INDEX IF NOT EXISTS idx_audit_model_date ON audit_logs(model_name, created_at);

CREATE TABLE IF NOT EXISTS rate_limit_counters (
    key_id     TEXT NOT NULL,
    model_name TEXT NOT NULL,
    window     TEXT NOT NULL,
    count      INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (key_id, model_name, window)
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
