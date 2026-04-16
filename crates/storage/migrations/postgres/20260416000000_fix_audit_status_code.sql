-- Fix status_code column type (TEXT -> INTEGER)
-- This migration corrects a schema mismatch where status_code was TEXT instead of INTEGER

-- For PostgreSQL: simple ALTER COLUMN
ALTER TABLE audit_logs ALTER COLUMN status_code TYPE INTEGER USING status_code::integer;
ALTER TABLE audit_logs ALTER COLUMN status_code SET NOT NULL;

-- Also ensure stream column exists (may be missing in older databases)
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS stream BOOLEAN NOT NULL DEFAULT false;