-- Add disabled_until column to channels for auto-recovery from upstream errors
ALTER TABLE channels ADD COLUMN IF NOT EXISTS disabled_until TIMESTAMPTZ;