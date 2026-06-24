-- User Groups: add canonical groups table, normalize channels.group to FK, add users.group_id

CREATE TABLE groups (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Backfill groups from existing distinct channel.group values (idempotent on re-run)
INSERT INTO groups (id, name)
SELECT gen_random_uuid()::text, "group"
FROM (SELECT DISTINCT "group" FROM channels WHERE "group" IS NOT NULL) t
ON CONFLICT (name) DO NOTHING;

-- Add channels.group_id column
ALTER TABLE channels ADD COLUMN group_id TEXT REFERENCES groups(id) ON DELETE SET NULL;

-- Backfill channels.group_id by matching the legacy name
UPDATE channels c
SET group_id = g.id
FROM groups g
WHERE c."group" = g.name;

-- Verify all non-null legacy groups got backfilled (should be 0 rows)
DO $$
DECLARE
    unbackfilled_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO unbackfilled_count
    FROM channels WHERE "group" IS NOT NULL AND group_id IS NULL;
    IF unbackfilled_count > 0 THEN
        RAISE EXCEPTION 'Backfill failed: % channels had a group but no matching groups row', unbackfilled_count;
    END IF;
END$$;

-- Drop legacy column
ALTER TABLE channels DROP COLUMN "group";

-- Add users.group_id
ALTER TABLE users ADD COLUMN group_id TEXT REFERENCES groups(id) ON DELETE SET NULL;
