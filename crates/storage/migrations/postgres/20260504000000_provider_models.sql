CREATE TABLE IF NOT EXISTS provider_models (
    provider_id TEXT NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
    model_id    TEXT NOT NULL REFERENCES models(id) ON DELETE CASCADE,
    upstream_name TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(provider_id, model_id)
);
