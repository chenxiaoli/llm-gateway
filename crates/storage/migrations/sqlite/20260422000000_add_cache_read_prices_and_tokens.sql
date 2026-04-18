-- SQLite migration: Add cache_read_price to channel_models and cache_read_tokens to usage_records
ALTER TABLE channel_models ADD COLUMN cache_read_price REAL;
ALTER TABLE usage_records ADD COLUMN cache_read_tokens INTEGER;
