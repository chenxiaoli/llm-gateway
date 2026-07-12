ALTER TABLE invitations DROP CONSTRAINT IF EXISTS invitations_pending_need_recipient;
ALTER TABLE invitations DROP COLUMN IF EXISTS recipient_email;
