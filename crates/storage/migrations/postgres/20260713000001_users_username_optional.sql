-- Drop the NOT NULL on username so email-only registrations are valid.
-- Existing rows all have a username, so the change is permissive: no
-- backfill needed. UNIQUE constraint on username is preserved (duplicates
-- were never allowed and still aren't); the partial UNIQUE index on
-- LOWER(email) is unaffected.
--
-- The CHECK constraint enforces "at least one identifier" — a row with
-- both username and email NULL is rejected. Both being set is allowed
-- (e.g. legacy users who later add an email).

ALTER TABLE users ALTER COLUMN username DROP NOT NULL;

ALTER TABLE users ADD CONSTRAINT users_username_or_email_required
    CHECK (username IS NOT NULL OR email IS NOT NULL);
