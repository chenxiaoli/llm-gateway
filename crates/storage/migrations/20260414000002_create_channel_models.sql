CREATE TABLE channel_models (
    id                  TEXT PRIMARY KEY,
    channel_id          TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    model_id            TEXT NOT NULL REFERENCES models(id) ON DELETE CASCADE,
    upstream_model_name TEXT NOT NULL,
    priority_override   INTEGER,
    enabled             INTEGER NOT NULL DEFAULT 1,
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL,
    UNIQUE(channel_id, model_id)
);

CREATE INDEX idx_channel_models_model ON channel_models(model_id);
CREATE INDEX idx_channel_models_channel ON channel_models(channel_id);