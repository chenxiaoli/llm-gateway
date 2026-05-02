ALTER TABLE provider_models ADD COLUMN pricing_policy_id TEXT REFERENCES pricing_policies(id) ON DELETE SET NULL;
