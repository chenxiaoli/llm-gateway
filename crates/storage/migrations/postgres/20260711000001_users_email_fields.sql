-- Phase 4: email support on users.
--
-- `email` is NULL for pre-Phase-4 users; required for new registrations.
-- `email_verified_at` is NULL until the user clicks the verification link.
-- `requires_email_verification` distinguishes "new signup that must verify
--   before login" (TRUE) from "existing user who added an email post-hoc"
--   (FALSE). Register endpoint sets TRUE; POST /auth/me/email does not.
-- `password_changed_at` starts at NOW() for existing users so existing
--   refresh tokens remain valid; updated on every successful password reset.
--
-- The partial unique index on LOWER(email) enforces case-insensitive
-- uniqueness among users with an email. The WHERE email IS NOT NULL clause
-- keeps the index small and lets legacy users coexist.

ALTER TABLE users
    ADD COLUMN email                       TEXT,
    ADD COLUMN email_verified_at           TIMESTAMPTZ,
    ADD COLUMN requires_email_verification BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN password_changed_at         TIMESTAMPTZ NOT NULL DEFAULT NOW();

CREATE UNIQUE INDEX users_email_unique_idx
    ON users (LOWER(email))
    WHERE email IS NOT NULL;
