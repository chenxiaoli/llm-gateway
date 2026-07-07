-- Restore users.role and users.group_id from members/platform_role
ALTER TABLE users ADD COLUMN role TEXT DEFAULT 'user' CHECK(role IN ('admin','user'));
ALTER TABLE users ADD COLUMN group_id TEXT;

UPDATE users SET role = 'admin' WHERE platform_role = 'platform_admin';

-- Restore users.group_id from members (default org only)
UPDATE users u SET group_id = m.group_id
FROM members m
WHERE m.user_id = u.id AND m.org_id = 'org_default';

-- Drop org_id columns from tenant tables
ALTER TABLE channels             DROP COLUMN org_id;
ALTER TABLE channel_models       DROP COLUMN org_id;
ALTER TABLE api_keys             DROP COLUMN org_id;
ALTER TABLE usage_records        DROP COLUMN org_id;
ALTER TABLE audit_logs           DROP COLUMN org_id;
ALTER TABLE accounts             DROP COLUMN org_id;
ALTER TABLE transactions         DROP COLUMN org_id;
ALTER TABLE key_model_rate_limits DROP COLUMN org_id;
ALTER TABLE groups               DROP COLUMN org_id;

-- Drop owner_org_id columns from catalog tables (and discard org-private entries —
-- see warning below)
DELETE FROM provider_models  WHERE owner_org_id IS NOT NULL;
DELETE FROM models           WHERE owner_org_id IS NOT NULL;
DELETE FROM providers        WHERE owner_org_id IS NOT NULL;
DELETE FROM pricing_policies WHERE owner_org_id IS NOT NULL;
ALTER TABLE provider_models  DROP COLUMN owner_org_id;
ALTER TABLE models           DROP COLUMN owner_org_id;
ALTER TABLE providers        DROP COLUMN owner_org_id;
ALTER TABLE pricing_policies DROP COLUMN owner_org_id;

-- Drop platform-admin audit flag
ALTER TABLE audit_logs DROP COLUMN actor_is_platform_admin;

-- Drop users.current_org_id and users.platform_role (must precede DROP TABLE orgs,
-- otherwise users_current_org_fk blocks the drop)
ALTER TABLE users DROP COLUMN current_org_id;
ALTER TABLE users DROP COLUMN platform_role;

-- Restore settings table name
ALTER TABLE platform_settings RENAME TO settings;

-- Drop members, orgs, org_settings
DROP TABLE members;
DROP TABLE org_settings;
DROP TABLE orgs;

-- Restore groups global name uniqueness.
-- Note: groups_org_name_unique and groups_org_fk were already auto-dropped when
-- groups.org_id was dropped above (Postgres cascades column drops to dependent
-- constraints), hence the IF EXISTS guard.
ALTER TABLE groups DROP CONSTRAINT IF EXISTS groups_org_name_unique;
ALTER TABLE groups ADD CONSTRAINT groups_name_key UNIQUE (name);
