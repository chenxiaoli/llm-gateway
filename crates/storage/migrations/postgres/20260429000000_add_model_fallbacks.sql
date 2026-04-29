CREATE TABLE IF NOT EXISTS model_fallbacks (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    config TEXT NOT NULL,
    created_by TEXT,
    created_at TIMESTAMPTZ NOT NULL
);

ALTER TABLE api_keys ADD COLUMN IF NOT EXISTS model_fallback_id TEXT REFERENCES model_fallbacks(id);

-- Fix existing table if created_at is TEXT
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'model_fallbacks' AND column_name = 'created_at' AND data_type = 'text') THEN
        ALTER TABLE model_fallbacks ALTER COLUMN created_at TYPE TIMESTAMPTZ USING created_at::TIMESTAMPTZ;
    END IF;
END $$;
