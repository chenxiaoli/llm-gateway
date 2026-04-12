CREATE TABLE IF NOT EXISTS channels (
    id          TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    api_key     TEXT NOT NULL,
    base_url    TEXT,
    priority    INTEGER NOT NULL DEFAULT 0,
    enabled     BOOLEAN NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_channels_provider ON channels(provider_id);

INSERT INTO channels (id, provider_id, name, api_key, base_url, priority, enabled, created_at, updated_at)
SELECT lower(hex(randomblob(4))), id, 'default', api_key, NULL, 0, 1, created_at, updated_at
FROM providers WHERE api_key IS NOT NULL AND api_key != '';

ALTER TABLE usage_records ADD COLUMN channel_id TEXT REFERENCES channels(id);
ALTER TABLE audit_logs ADD COLUMN channel_id TEXT REFERENCES channels(id);

ALTER TABLE providers DROP COLUMN api_key;
