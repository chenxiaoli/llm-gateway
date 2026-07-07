BEGIN;

-- 1. New tables
CREATE TABLE orgs (
    id          TEXT PRIMARY KEY,
    slug        TEXT NOT NULL UNIQUE CHECK (slug ~ '^[a-z0-9-]{3,64}$'),
    name        TEXT NOT NULL,
    owner_id    TEXT,  -- FK added later to break the users↔orgs cycle
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_orgs_owner ON orgs(owner_id);

CREATE TABLE members (
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    org_id      TEXT NOT NULL REFERENCES orgs(id) ON DELETE CASCADE,
    role        TEXT NOT NULL DEFAULT 'member' CHECK(role IN ('owner','admin','member')),
    group_id    TEXT,  -- FK added later (groups needs org_id first)
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by  TEXT,
    PRIMARY KEY (user_id, org_id)
);
CREATE INDEX idx_members_org ON members(org_id);
CREATE INDEX idx_members_system_impersonation ON members(org_id) WHERE created_by = 'system';

CREATE TABLE org_settings (
    org_id TEXT NOT NULL REFERENCES orgs(id) ON DELETE CASCADE,
    key    TEXT NOT NULL,
    value  TEXT NOT NULL,
    PRIMARY KEY (org_id, key)
);

-- 2. Rename settings → platform_settings (data preserved)
ALTER TABLE settings RENAME TO platform_settings;

-- 3. Add org_id (nullable first) to tenant tables
ALTER TABLE users ADD COLUMN current_org_id TEXT;
ALTER TABLE users ADD COLUMN platform_role TEXT
    CHECK (platform_role IS NULL OR platform_role = 'platform_admin');

ALTER TABLE channels             ADD COLUMN org_id TEXT;
ALTER TABLE channel_models       ADD COLUMN org_id TEXT;
ALTER TABLE api_keys             ADD COLUMN org_id TEXT;
ALTER TABLE usage_records        ADD COLUMN org_id TEXT;
ALTER TABLE audit_logs           ADD COLUMN org_id TEXT;
ALTER TABLE audit_logs           ADD COLUMN actor_is_platform_admin BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE accounts             ADD COLUMN org_id TEXT;
ALTER TABLE transactions         ADD COLUMN org_id TEXT;
ALTER TABLE key_model_rate_limits ADD COLUMN org_id TEXT;
ALTER TABLE groups               ADD COLUMN org_id TEXT;

-- 3a. Add owner_org_id (nullable, stays NULL = platform-level) to catalog tables
ALTER TABLE providers        ADD COLUMN owner_org_id TEXT;
ALTER TABLE models           ADD COLUMN owner_org_id TEXT;
ALTER TABLE pricing_policies ADD COLUMN owner_org_id TEXT;
ALTER TABLE provider_models  ADD COLUMN owner_org_id TEXT;

-- 4. Create default org with placeholder owner (fixed in step 7)
INSERT INTO orgs (id, slug, name, owner_id, created_at, updated_at)
VALUES ('org_default', 'default', 'Default Org', NULL, NOW(), NOW());

-- 5. Backfill: existing admin users → default org owners + platform_admin
INSERT INTO members (user_id, org_id, role, created_by)
SELECT id, 'org_default', 'owner', id FROM users WHERE role = 'admin';
UPDATE users SET platform_role = 'platform_admin' WHERE role = 'admin';

-- 6. Backfill: existing regular users → default org members
INSERT INTO members (user_id, org_id, role, created_by)
SELECT id, 'org_default', 'member', id FROM users WHERE role = 'user';

-- 7. Fix default org's owner_id (earliest admin user)
UPDATE orgs SET owner_id = (
    SELECT user_id FROM members
    WHERE org_id = 'org_default' AND role = 'owner'
    ORDER BY created_at LIMIT 1
) WHERE id = 'org_default';

-- 8. Backfill users.current_org_id
UPDATE users SET current_org_id = 'org_default';

-- 9. Backfill users.group_id → members.group_id (group is per-membership now)
UPDATE members m
SET group_id = u.group_id
FROM users u
WHERE m.user_id = u.id
  AND m.org_id = 'org_default'
  AND u.group_id IS NOT NULL;

-- 10. Backfill tenant tables' org_id to default org
-- (catalog tables stay owner_org_id = NULL — they are platform-level by default)
UPDATE channels             SET org_id = 'org_default' WHERE org_id IS NULL;
UPDATE channel_models       SET org_id = 'org_default' WHERE org_id IS NULL;
UPDATE api_keys             SET org_id = 'org_default' WHERE org_id IS NULL;
UPDATE usage_records        SET org_id = 'org_default' WHERE org_id IS NULL;
UPDATE audit_logs           SET org_id = 'org_default' WHERE org_id IS NULL;
UPDATE accounts             SET org_id = 'org_default' WHERE org_id IS NULL;
UPDATE transactions         SET org_id = 'org_default' WHERE org_id IS NULL;
UPDATE key_model_rate_limits SET org_id = 'org_default' WHERE org_id IS NULL;

-- 11. Backfill existing groups → default org
UPDATE groups SET org_id = 'org_default' WHERE org_id IS NULL;

-- 12. Tighten NOT NULL constraints on tenant tables
ALTER TABLE channels             ALTER COLUMN org_id SET NOT NULL;
ALTER TABLE channel_models       ALTER COLUMN org_id SET NOT NULL;
ALTER TABLE api_keys             ALTER COLUMN org_id SET NOT NULL;
ALTER TABLE usage_records        ALTER COLUMN org_id SET NOT NULL;
ALTER TABLE audit_logs           ALTER COLUMN org_id SET NOT NULL;
ALTER TABLE accounts             ALTER COLUMN org_id SET NOT NULL;
ALTER TABLE transactions         ALTER COLUMN org_id SET NOT NULL;
ALTER TABLE key_model_rate_limits ALTER COLUMN org_id SET NOT NULL;
ALTER TABLE groups               ALTER COLUMN org_id SET NOT NULL;

-- 13. Add FK constraints: org_id on tenant tables, owner_org_id on catalog tables
ALTER TABLE channels             ADD CONSTRAINT channels_org_fk             FOREIGN KEY (org_id) REFERENCES orgs(id) ON DELETE CASCADE;
ALTER TABLE channel_models       ADD CONSTRAINT channel_models_org_fk       FOREIGN KEY (org_id) REFERENCES orgs(id) ON DELETE CASCADE;
ALTER TABLE api_keys             ADD CONSTRAINT api_keys_org_fk             FOREIGN KEY (org_id) REFERENCES orgs(id) ON DELETE CASCADE;
ALTER TABLE usage_records        ADD CONSTRAINT usage_records_org_fk        FOREIGN KEY (org_id) REFERENCES orgs(id) ON DELETE CASCADE;
ALTER TABLE audit_logs           ADD CONSTRAINT audit_logs_org_fk           FOREIGN KEY (org_id) REFERENCES orgs(id) ON DELETE CASCADE;
ALTER TABLE accounts             ADD CONSTRAINT accounts_org_fk             FOREIGN KEY (org_id) REFERENCES orgs(id) ON DELETE CASCADE;
ALTER TABLE transactions         ADD CONSTRAINT transactions_org_fk        FOREIGN KEY (org_id) REFERENCES orgs(id) ON DELETE CASCADE;
ALTER TABLE key_model_rate_limits ADD CONSTRAINT key_model_rate_limits_org_fk FOREIGN KEY (org_id) REFERENCES orgs(id) ON DELETE CASCADE;
ALTER TABLE groups               ADD CONSTRAINT groups_org_fk              FOREIGN KEY (org_id) REFERENCES orgs(id) ON DELETE CASCADE;

ALTER TABLE providers        ADD CONSTRAINT providers_owner_org_fk        FOREIGN KEY (owner_org_id) REFERENCES orgs(id) ON DELETE CASCADE;
ALTER TABLE models           ADD CONSTRAINT models_owner_org_fk           FOREIGN KEY (owner_org_id) REFERENCES orgs(id) ON DELETE CASCADE;
ALTER TABLE pricing_policies ADD CONSTRAINT pricing_policies_owner_org_fk FOREIGN KEY (owner_org_id) REFERENCES orgs(id) ON DELETE CASCADE;
ALTER TABLE provider_models  ADD CONSTRAINT provider_models_owner_org_fk  FOREIGN KEY (owner_org_id) REFERENCES orgs(id) ON DELETE CASCADE;

-- 14. Index reworks

-- 14a. Tenant tables: org_id first
CREATE INDEX idx_channels_org_enabled ON channels(org_id, enabled);
DROP INDEX IF EXISTS idx_channels_enabled;

CREATE INDEX idx_channel_models_org_channel ON channel_models(org_id, channel_id);

CREATE INDEX idx_api_keys_org ON api_keys(org_id);

CREATE INDEX idx_usage_org_date       ON usage_records(org_id, created_at);
CREATE INDEX idx_usage_org_key_date   ON usage_records(org_id, key_id, created_at);
DROP INDEX IF EXISTS idx_usage_key_date;
DROP INDEX IF EXISTS idx_usage_model_date;  -- replaced by org-scoped query via models.owner_org_id

CREATE INDEX idx_audit_org_date       ON audit_logs(org_id, created_at);
CREATE INDEX idx_audit_org_key_date   ON audit_logs(org_id, key_id, created_at);
DROP INDEX IF EXISTS idx_audit_key_date;
DROP INDEX IF EXISTS idx_audit_model_date;

CREATE INDEX idx_transactions_org ON transactions(org_id);

-- accounts: drop old unique(user_id), add unique(org_id, user_id)
ALTER TABLE accounts DROP CONSTRAINT IF EXISTS accounts_user_id_key;
ALTER TABLE accounts ADD CONSTRAINT accounts_org_user_unique UNIQUE (org_id, user_id);

-- 14b. Catalog tables: drop global UNIQUE(name/slug), add partial UNIQUEs + owner_org_id lookup indexes
ALTER TABLE models DROP CONSTRAINT IF EXISTS models_name_key;
CREATE UNIQUE INDEX models_platform_name_unique ON models(name) WHERE owner_org_id IS NULL;
CREATE UNIQUE INDEX models_org_name_unique      ON models(owner_org_id, name) WHERE owner_org_id IS NOT NULL;
CREATE INDEX models_owner_org_idx               ON models(owner_org_id) WHERE owner_org_id IS NOT NULL;

ALTER TABLE providers DROP CONSTRAINT IF EXISTS providers_slug_key;
CREATE UNIQUE INDEX providers_platform_slug_unique ON providers(slug) WHERE owner_org_id IS NULL;
CREATE UNIQUE INDEX providers_org_slug_unique      ON providers(owner_org_id, slug) WHERE owner_org_id IS NOT NULL;
CREATE INDEX providers_owner_org_idx               ON providers(owner_org_id) WHERE owner_org_id IS NOT NULL;

ALTER TABLE pricing_policies DROP CONSTRAINT IF EXISTS pricing_policies_name_key;
CREATE UNIQUE INDEX pricing_policies_platform_name_unique ON pricing_policies(name) WHERE owner_org_id IS NULL;
CREATE UNIQUE INDEX pricing_policies_org_name_unique      ON pricing_policies(owner_org_id, name) WHERE owner_org_id IS NOT NULL;
CREATE INDEX pricing_policies_owner_org_idx               ON pricing_policies(owner_org_id) WHERE owner_org_id IS NOT NULL;

CREATE INDEX provider_models_owner_org_idx ON provider_models(owner_org_id) WHERE owner_org_id IS NOT NULL;

-- 15. groups name per-org unique
ALTER TABLE groups DROP CONSTRAINT IF EXISTS groups_name_key;
ALTER TABLE groups ADD CONSTRAINT groups_org_name_unique UNIQUE (org_id, name);

-- 16. members.group_id FK
ALTER TABLE members ADD CONSTRAINT members_group_fk
    FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE SET NULL;

-- 17. Circular FKs: users.current_org_id and orgs.owner_id
ALTER TABLE users ADD CONSTRAINT users_current_org_fk
    FOREIGN KEY (current_org_id) REFERENCES orgs(id) ON DELETE SET NULL
    DEFERRABLE INITIALLY IMMEDIATE;
ALTER TABLE orgs ADD CONSTRAINT orgs_owner_fk
    FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE SET NULL
    DEFERRABLE INITIALLY IMMEDIATE;

-- 18. Drop legacy columns
ALTER TABLE users DROP COLUMN role;
ALTER TABLE users DROP COLUMN group_id;

-- 19. Self-check (fail → rollback entire migration)
DO $$
DECLARE
    orphan_keys INTEGER;
    orphan_users INTEGER;
    orphan_channels INTEGER;
    catalog_owner_mismatch INTEGER;
    users_without_membership INTEGER;
BEGIN
    SELECT COUNT(*) INTO orphan_keys FROM api_keys WHERE org_id IS NULL;
    IF orphan_keys > 0 THEN RAISE EXCEPTION 'orphan api_keys: %', orphan_keys; END IF;

    SELECT COUNT(*) INTO orphan_users FROM users WHERE current_org_id IS NULL;
    IF orphan_users > 0 THEN RAISE EXCEPTION 'users without current_org_id: %', orphan_users; END IF;

    SELECT COUNT(*) INTO users_without_membership FROM users u
    WHERE NOT EXISTS (SELECT 1 FROM members m WHERE m.user_id = u.id);
    IF users_without_membership > 0 THEN
        RAISE EXCEPTION 'users without any membership: %', users_without_membership;
    END IF;

    SELECT COUNT(*) INTO orphan_channels FROM channels WHERE org_id IS NULL;
    IF orphan_channels > 0 THEN RAISE EXCEPTION 'orphan channels: %', orphan_channels; END IF;

    -- Catalog invariant: provider_models.owner_org_id must match providers.owner_org_id (and models.owner_org_id) at platform level or same org.
    -- A row linking a platform-level provider to an org-private model (or vice versa) is rejected at insert time; this check just guards against manual SQL drift.
    SELECT COUNT(*) INTO catalog_owner_mismatch FROM provider_models pm
    JOIN providers p ON p.id = pm.provider_id
    JOIN models m ON m.id = pm.model_id
    WHERE COALESCE(pm.owner_org_id, p.owner_org_id, m.owner_org_id) IS NOT NULL
      AND NOT (pm.owner_org_id IS NULL AND p.owner_org_id IS NULL AND m.owner_org_id IS NULL)
      AND NOT (pm.owner_org_id IS NOT NULL
               AND pm.owner_org_id = p.owner_org_id
               AND pm.owner_org_id = m.owner_org_id);
    IF catalog_owner_mismatch > 0 THEN
        RAISE EXCEPTION 'provider_models with inconsistent owner_org_id across provider/model/junction: %', catalog_owner_mismatch;
    END IF;
END $$;

COMMIT;
