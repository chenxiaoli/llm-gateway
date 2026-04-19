-- PostgreSQL migration: Add request_path and upstream_url to audit_logs
ALTER TABLE audit_logs ADD COLUMN request_path TEXT;
ALTER TABLE audit_logs ADD COLUMN upstream_url TEXT;
