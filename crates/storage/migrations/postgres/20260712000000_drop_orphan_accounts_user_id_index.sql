-- idx_accounts_user_id was left behind when 20260708000000_saas_orgs.sql
-- added accounts_org_user_unique (the per-membership UNIQUE constraint).
-- The plain user_id index is now redundant — every account lookup goes
-- through (org_id, user_id) via the unique constraint's btree.
DROP INDEX IF EXISTS idx_accounts_user_id;
