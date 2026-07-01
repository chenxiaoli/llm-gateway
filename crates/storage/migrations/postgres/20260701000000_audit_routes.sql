-- Add routes JSONB array to audit_logs. Each entry records one upstream
-- attempt (model, channel, status, error, latency, started_at).
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS routes JSONB;