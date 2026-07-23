-- Add optional `nickname` column to users.
-- Nullable, no UNIQUE, no index: nickname is a display label, not an
-- identifier (multiple users may share a nickname). NULL means "user
-- hasn't set one" — display code falls back via displayName().
ALTER TABLE users ADD COLUMN nickname TEXT;
