-- Add stream column to audit_logs
ALTER TABLE audit_logs ADD COLUMN stream INTEGER NOT NULL DEFAULT 0;