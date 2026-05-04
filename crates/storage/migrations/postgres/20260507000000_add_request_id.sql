-- Add request_id to usage_records
ALTER TABLE usage_records ADD COLUMN request_id TEXT;
CREATE INDEX idx_usage_request_id ON usage_records(request_id);

-- Add request_id to audit_logs
ALTER TABLE audit_logs ADD COLUMN request_id TEXT;
CREATE INDEX idx_audit_request_id ON audit_logs(request_id);

-- Add request_id to transactions (nullable — manual credits don't have one)
ALTER TABLE transactions ADD COLUMN request_id TEXT;
CREATE INDEX idx_transactions_request_id ON transactions(request_id);
