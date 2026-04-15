-- Add updated_at column to models table
ALTER TABLE models ADD COLUMN updated_at TEXT NOT NULL DEFAULT (datetime('now'));
