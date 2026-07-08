-- Add members.last_seen for platform-admin impersonation janitor.
-- The janitor deletes stale temp rows (created_by='system') based on last_seen.
-- Existing members get last_seen = NOW() as a one-time backfill; their next
-- request will update last_seen (Tasks 2-4 wire that up).
ALTER TABLE members ADD COLUMN last_seen TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Backfill existing rows to NOW() — their next request will update last_seen.
UPDATE members SET last_seen = NOW() WHERE last_seen IS NULL;

CREATE INDEX idx_members_system_impersonation_last_seen
    ON members(last_seen)
    WHERE created_by = 'system';
