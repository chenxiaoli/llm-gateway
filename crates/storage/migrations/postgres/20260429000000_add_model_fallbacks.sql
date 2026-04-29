CREATE TABLE IF NOT EXISTS model_fallbacks (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    config TEXT NOT NULL,
    created_by TEXT,
    created_at TEXT NOT NULL
);

ALTER TABLE api_keys ADD COLUMN IF NOT EXISTS model_fallback_id TEXT REFERENCES model_fallbacks(id);
