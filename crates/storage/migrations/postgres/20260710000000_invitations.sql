-- Phase 3: invitations table for magic-link org invitations.
--
-- One row per minted invitation. `token` is the lookup key (opaque 32-byte
-- random, base64url). `role` excludes 'owner' at the DB level — owner is a
-- self-promotion flow inside an org, not assignable by invitation.
--
-- Lifecycle:
--   mint     → row inserted, accepted_at + revoked_at NULL
--   accept   → accepted_at + accepted_by set (single-transaction with members insert)
--   revoke   → revoked_at set (admin action, irreversible)
--
-- Cleanup: rows are retained indefinitely for audit. A future janitor can
-- prune >1-year-old rows; not in Phase 3.
--
-- NOTE: orgs.id and users.id are TEXT (see 20260415000000_initial.sql and
-- 20260708000000_saas_orgs.sql), so the FK columns below are TEXT. The
-- invitations.id PK is a server-generated UUID (gen_random_uuid()).

CREATE TABLE invitations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token           TEXT NOT NULL UNIQUE,
    org_id          TEXT NOT NULL REFERENCES orgs(id) ON DELETE CASCADE,
    role            TEXT NOT NULL DEFAULT 'member' CHECK(role IN ('member','admin')),
    created_by      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ NOT NULL,
    accepted_at     TIMESTAMPTZ,
    accepted_by     TEXT REFERENCES users(id) ON DELETE SET NULL,
    revoked_at      TIMESTAMPTZ,
    CONSTRAINT invitations_expires_after_created CHECK (expires_at > created_at)
);

-- Speed up the admin "pending invitations" list. Partial index keeps it small
-- even after the table accumulates accepted/expired history.
CREATE INDEX invitations_org_pending_idx
    ON invitations (org_id)
    WHERE accepted_at IS NULL AND revoked_at IS NULL;
