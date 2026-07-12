DROP INDEX IF EXISTS users_email_unique_idx;
ALTER TABLE users
    DROP COLUMN IF EXISTS password_changed_at,
    DROP COLUMN IF EXISTS requires_email_verification,
    DROP COLUMN IF EXISTS email_verified_at,
    DROP COLUMN IF EXISTS email;
