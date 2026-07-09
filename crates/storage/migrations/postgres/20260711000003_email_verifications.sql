-- Phase 4: email_verifications table.
--
-- One row per verification email dispatched. Token is the lookup key
-- (32-byte random, base64url-no-pad). 24-hour expiry.
--
-- Lifecycle:
--   mint     → row inserted, consumed_at NULL
--   consume  → consumed_at set (single-transaction with users.email_verified_at update)
--
-- `email` is denormalized from users.email so the row records *what* was
-- being verified, not just *who* requested it. Useful if we ever support
-- email changes.
--
-- NOTE: users.id is TEXT, so user_id FK is TEXT. PK is server-generated UUID.

CREATE TABLE email_verifications (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token        TEXT NOT NULL UNIQUE,
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    email        TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at   TIMESTAMPTZ NOT NULL,
    consumed_at  TIMESTAMPTZ,
    CONSTRAINT email_verifications_expires_after_created
        CHECK (expires_at > created_at),
    CONSTRAINT email_verifications_consumed_after_created
        CHECK (consumed_at IS NULL OR consumed_at > created_at)
);

CREATE INDEX email_verifications_user_idx
    ON email_verifications (user_id)
    WHERE consumed_at IS NULL;
