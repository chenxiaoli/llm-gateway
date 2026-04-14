-- Add new fields to channels table
ALTER TABLE channels ADD COLUMN rpm_limit INTEGER;
ALTER TABLE channels ADD COLUMN tpm_limit INTEGER;
ALTER TABLE channels ADD COLUMN balance REAL;
ALTER TABLE channels ADD COLUMN weight INTEGER DEFAULT 100;