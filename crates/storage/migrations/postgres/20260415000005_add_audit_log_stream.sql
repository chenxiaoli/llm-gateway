-- Add stream column to audit_logs
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS stream BOOLEAN NOT NULL DEFAULT false;