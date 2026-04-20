-- SQLite migration: Add request_headers and response_headers to audit_logs
ALTER TABLE audit_logs ADD COLUMN request_headers TEXT;
ALTER TABLE audit_logs ADD COLUMN response_headers TEXT;
