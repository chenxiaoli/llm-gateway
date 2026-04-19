-- PostgreSQL migration: Add model override fields to audit_logs
ALTER TABLE audit_logs ADD COLUMN original_model TEXT;
ALTER TABLE audit_logs ADD COLUMN upstream_model TEXT;
ALTER TABLE audit_logs ADD COLUMN model_override_reason TEXT;