-- Add updated_at column to models table
-- Note: SQLite doesn't support non-constant DEFAULT, so we add column without default
-- and will set it via application logic or separate UPDATE
ALTER TABLE models ADD COLUMN updated_at TEXT NOT NULL DEFAULT '';
