-- Phase 4: invitations become email-bound.
--
-- Adds recipient_email TEXT (nullable for backward compat with already-accepted
-- rows). The CHECK constraint enforces "going forward, every pending invitation
-- must have a recipient_email" — accepted/revoked rows are grandfathered
-- through the OR arms.
--
-- Data migration: revoke all pending Phase 3 invitations (no recipient_email).
-- Admins who relied on the old generic-token flow will need to re-mint. This
-- is intentional — old invitations were effectively unauthenticated.

ALTER TABLE invitations ADD COLUMN recipient_email TEXT;

UPDATE invitations
SET revoked_at = NOW()
WHERE accepted_at IS NULL
  AND revoked_at IS NULL
  AND recipient_email IS NULL;

ALTER TABLE invitations
    ADD CONSTRAINT invitations_pending_need_recipient
    CHECK (
        accepted_at IS NOT NULL
        OR revoked_at IS NOT NULL
        OR recipient_email IS NOT NULL
    );
