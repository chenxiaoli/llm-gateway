-- PostgreSQL migration: Add request_headers and response_headers to audit_logs
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS request_headers TEXT;
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS response_headers TEXT;
