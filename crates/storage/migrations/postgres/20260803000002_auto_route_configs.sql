CREATE TABLE IF NOT EXISTS auto_route_configs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    config TEXT NOT NULL,
    created_by TEXT,
    created_at TIMESTAMPTZ NOT NULL
);

ALTER TABLE api_keys ADD COLUMN IF NOT EXISTS auto_route_id TEXT REFERENCES auto_route_configs(id);
