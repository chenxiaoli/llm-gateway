-- PostgreSQL migration: Add request_path and upstream_url to audit_logs
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS request_path TEXT;
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS upstream_url TEXT;
