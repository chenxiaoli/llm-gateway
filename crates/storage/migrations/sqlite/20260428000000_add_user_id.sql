ALTER TABLE usage_records ADD COLUMN user_id TEXT;
ALTER TABLE audit_logs ADD COLUMN user_id TEXT;

UPDATE usage_records SET user_id = (
  SELECT created_by FROM api_keys WHERE api_keys.id = usage_records.key_id
);
UPDATE audit_logs SET user_id = (
  SELECT created_by FROM api_keys WHERE api_keys.id = audit_logs.key_id
);
