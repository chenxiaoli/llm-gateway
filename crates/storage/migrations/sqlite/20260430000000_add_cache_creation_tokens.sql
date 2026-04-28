-- SQLite migration: Add cache_creation_tokens to usage_records
ALTER TABLE usage_records ADD COLUMN cache_creation_tokens INTEGER;
