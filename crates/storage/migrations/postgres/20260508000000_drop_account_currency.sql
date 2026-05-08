-- Drop the unused currency column from accounts (now a system-level setting)
ALTER TABLE accounts DROP COLUMN IF EXISTS currency;
