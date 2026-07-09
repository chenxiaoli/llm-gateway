-- Phase 4: password_resets table. Same shape as email_verifications.
-- 1-hour expiry (vs. 24h for verifications) — resets are higher-stakes.

CREATE TABLE password_resets (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token        TEXT NOT NULL UNIQUE,
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at   TIMESTAMPTZ NOT NULL,
    consumed_at  TIMESTAMPTZ,
    CONSTRAINT password_resets_expires_after_created
        CHECK (expires_at > created_at),
    CONSTRAINT password_resets_consumed_after_created
        CHECK (consumed_at IS NULL OR consumed_at > created_at)
);

CREATE INDEX password_resets_user_idx
    ON password_resets (user_id)
    WHERE consumed_at IS NULL;
