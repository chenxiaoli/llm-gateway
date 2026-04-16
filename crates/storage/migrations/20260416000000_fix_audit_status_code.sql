-- Fix status_code column type (TEXT -> INTEGER)
-- This migration corrects a schema mismatch where status_code was TEXT instead of INTEGER

-- For SQLite: need to recreate the table
-- SQLite doesn't support ALTER TABLE MODIFY COLUMN, so we rename and recreate
ALTER TABLE audit_logs RENAME TO audit_logs_old;

CREATE TABLE IF NOT EXISTS audit_logs (
    id            TEXT PRIMARY KEY,
    key_id        TEXT NOT NULL,
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

-- Copy data, converting status_code from TEXT to INTEGER
INSERT INTO audit_logs (id, key_id, model_name, provider_id, channel_id, protocol, stream, request_body, response_body, status_code, latency_ms, input_tokens, output_tokens, created_at)
SELECT id, key_id, model_name, provider_id, channel_id, protocol, COALESCE(stream, 0), request_body, response_body,
       CAST(status_code AS INTEGER), latency_ms, input_tokens, output_tokens, created_at
FROM audit_logs_old;

DROP TABLE audit_logs_old;

-- Recreate indexes
CREATE INDEX IF NOT EXISTS idx_audit_key_date ON audit_logs(key_id, created_at);
CREATE INDEX IF NOT EXISTS idx_audit_model_date ON audit_logs(model_name, created_at);