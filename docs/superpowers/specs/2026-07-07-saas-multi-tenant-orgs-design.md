# SaaS Multi-Tenant (Orgs) Design

**Date:** 2026-07-07
**Status:** Draft (pending user review)
**Targets release:** v1.9.0 (Phase 1) → v1.10.0 (Phase 2) → v1.11.0 (Phase 3)

## Problem

Today the gateway is single-tenant: providers, channels, models, api_keys, usage, audit, accounts are all shared across every user. The only user-scoping primitive is `users.group_id` (recently added in v1.8.x), which limits channel visibility within a single shared dataset.

To serve multiple companies on a shared deployment — the standard B2B SaaS model — we need a tenant boundary. Each company's data must be isolated: their own providers, channels, models, keys, usage, audit, billing. Users move between tenants (personal org + work org) without re-registering.

## Goal

Add an `org` concept as the top-level tenant boundary. Each org owns its full data slice. Users belong to one or more orgs via a `members` junction table. Routing identifies the org via URL path on the management API, and via API key on the proxy API.

Ship in three phases so each release is independently shippable and reversible:

- **Phase 1** — schema + backend org-awareness. Behavior externally unchanged.
- **Phase 2** — multi-org support, URL routing, org switcher.
- **Phase 3** — signup auto-creates personal org, invitations.

## Non-Goals

- Subdomain routing (`acme.llmgateway.com`) — explicitly out; path-based routing only. No wildcard TLS, no per-customer DNS.
- Per-org username uniqueness — usernames stay globally unique. M:N model removes the conflict that would have required per-org uniqueness anyway.
- SCIM / SSO / SAML enterprise identity — future work.
- Cross-org billing aggregation — each org has independent accounts; no platform-wide invoice consolidation in scope.
- Per-org rate-limit config in `org_settings` — Phase 1 leaves the existing global config alone; per-org override is a Phase 4+ item if needed.
- Slug-resolve caching — every request hits the DB to resolve `slug → org_id` (~1ms on indexed UNIQUE). Cache later if it shows up in profiles.

## Decisions Locked (from brainstorming)

1. **User↔Org is M:N via a `members` junction table** — not 1:1. Marginal cost over 1:1 is small (~15%), and M:N removes the subdomain burden that 1:1 would impose.
2. **`users.current_org_id`** records the org a user is currently operating in. Token encodes it; switching org reissues token.
3. **Tenant-scoped business tables get NOT NULL `org_id`** — channels, channel_models, api_keys, usage_records, audit_logs, accounts, transactions, key_model_rate_limits, groups. Strict isolation: a row in org A is never visible to org B.
14. **Catalog tables (`providers`, `models`, `pricing_policies`, `provider_models`) use a hybrid model** — nullable `owner_org_id` column. `NULL` = platform-level entry visible to all orgs (e.g., the OpenAI provider, GPT-4 model). Non-NULL = org-private entry visible only to that org (e.g., a self-hosted LLM, a fine-tune, a custom pricing policy). Storage layer encapsulates the visibility filter `(owner_org_id IS NULL OR owner_org_id = $1)` so it cannot be forgotten at a call site. Org cannot create an entry whose name collides with a platform-level entry (anti-shadowing guard, enforced in the storage trait).
4. **Management API/UI routing via `/{org_slug}/...`** — explicit path segment. Proxy API (`/v1/chat/completions`, `/v1/messages`) unchanged; org resolved from API key.
5. **Two-layer role model**:
   - `members.role IN ('owner', 'admin', 'member')` — org-scoped permissions.
   - `users.platform_role` — `NULL` or `'platform_admin'` for hosting-company staff who can cross org boundaries.
6. **Migration: bootstrap default org** — all existing data moves into a single default org. Existing `role='admin'` users become default-org owners + `platform_role='platform_admin'`; existing `role='user'` users become default-org members.
7. **Signup auto-creates a personal org** for the new user, who becomes its owner. GitHub/Linear/Vercel pattern.
8. **Multiple owners per org allowed** — guard against "last owner leaving" is enforced at the application layer, not via DB constraint.
9. **`orgs.owner_id` uses `ON DELETE SET NULL`** — DB safety net only; app layer prevents reaching a zero-owner state.
10. **`settings` splits into `platform_settings` + `org_settings`** — no fallback chain between them. Each key's scope is decided in code at read site.
11. **`groups.name` is unique per org** — `UNIQUE (org_id, name)` replaces the existing `UNIQUE (name)`.
12. **New crate `org`** owns types, context, extractors, and access rules. Storage trait methods stay in `storage`.
13. **Platform admin impersonation via temporary `members` row** — when a platform_admin operates in an org, the system creates a `role='admin'` member row marked `created_by='system'`, used for the session, then deleted on exit. Avoids `if platform_admin { bypass }` branches scattered across handlers.

## Architecture

### New crate: `org`

```
crates/org/Cargo.toml
crates/org/src/
├── lib.rs              # pub use 子模块
├── types.rs            # Org, CreateOrg, UpdateOrg, Member, MemberRole, PlatformRole
├── context.rs          # OrgContext { org_id, org_slug, member_role, platform_role, group_id }
├── extractors.rs       # Axum extractors: require_member, require_org_admin, require_org_owner, require_platform_admin
├── access.rs           # pure functions: can_manage_org_settings, can_invite_members, can_delete_org, can_access_channel
└── error.rs            # OrgError (NotFound, NotMember, Forbidden, SlugTaken, LastOwner)
```

### Dependency graph

```
gateway ──> api ──> org ──> storage   (Org/Member types, Storage trait extension)
              │      └──> auth        (JWT claims, password hashing — unchanged)
              └──> storage, auth, billing, audit, provider, ratelimit, encryption, ...
```

`api` does not query `members` or `orgs` directly — it goes through `org`. `org` depends on `storage` for types but does not own the Storage trait.

### Request lifecycle (management API)

```
1. AuthLayer         — verify JWT → user_id, platform_role from claims
2. OrgResolveLayer   — extract {org_slug} from path → SELECT org WHERE slug=$1
                       not found → 404
3. MembershipLayer   — SELECT member WHERE user_id=$1 AND org_id=$2
                       not found and not platform_admin → 403
                       platform_admin → create temp member row (role='admin', created_by='system')
                       inject OrgContext { org_id, slug, member_role, platform_role, group_id }
4. Handler           — pull OrgContext from State, pass org_id to every Storage call
```

### Request lifecycle (proxy API)

```
1. ApiKeyAuth layer   — extract bearer/x-api-key → sha256 → SELECT api_key WHERE key_hash=$1
                        pull key.org_id
                        inject ApiKeyContext { key_id, org_id, user_id, rate_limit, ... }
2. Handler            — routing/usage/audit all use ApiKeyContext.org_id as WHERE condition
URL unchanged (/v1/chat/completions, /v1/messages)
```

## Data Model

### New table: `orgs`

| Column       | Type        | Notes                                                       |
| ------------ | ----------- | ----------------------------------------------------------- |
| `id`         | TEXT PK     | UUID v4 string                                              |
| `slug`       | TEXT UNIQUE NOT NULL | `^[a-z0-9-]{3,64}$`, globally unique (URL collision) |
| `name`       | TEXT NOT NULL | Display name (slug charset too restrictive for display)   |
| `owner_id`   | TEXT NOT NULL REFERENCES users(id) `ON DELETE SET NULL` `DEFERRABLE INITIALLY IMMEDIATE` | Primary owner; may be NULL only via DB safety net |
| `created_at` | TIMESTAMPTZ |                                                             |
| `updated_at` | TIMESTAMPTZ |                                                             |

```sql
CREATE INDEX idx_orgs_owner ON orgs(owner_id);
```

Multiple members may have `role='owner'`. The "last owner" guard is in the application.

### New table: `members`

| Column      | Type | Notes                                                                  |
| ----------- | ---- | --------------------------------------------------------------------- |
| `user_id`   | TEXT NOT NULL REFERENCES users(id) `ON DELETE CASCADE`                |
| `org_id`    | TEXT NOT NULL REFERENCES orgs(id) `ON DELETE CASCADE`                 |
| `role`      | TEXT NOT NULL DEFAULT 'member' CHECK(role IN ('owner','admin','member')) |
| `group_id`  | TEXT NULL REFERENCES groups(id) `ON DELETE SET NULL` — moved from `users.group_id` |
| `created_at`| TIMESTAMPTZ                                                             |
| `created_by`| TEXT NULL — `'system'` for platform_admin impersonation rows; otherwise the inviting user's id |
| PRIMARY KEY | `(user_id, org_id)`                                                     |

```sql
CREATE INDEX idx_members_org ON members(org_id);
CREATE INDEX idx_members_system_impersonation ON members(org_id) WHERE created_by = 'system';
```

The second index speeds up the platform-admin-impersonation cleanup (find temp rows quickly).

### `users` changes

```sql
ALTER TABLE users ADD COLUMN current_org_id TEXT REFERENCES orgs(id) ON DELETE SET NULL;
ALTER TABLE users ADD COLUMN platform_role TEXT
    CHECK(platform_role IS NULL OR platform_role = 'platform_admin');

ALTER TABLE users DROP COLUMN role;       -- migrated to members.role + users.platform_role
ALTER TABLE users DROP COLUMN group_id;   -- migrated to members.group_id
```

`current_org_id` is nullable only transiently during signup (after user-create, before org-create). Steady state: every user has a `current_org_id`.

### `groups` changes

```sql
ALTER TABLE groups ADD COLUMN org_id TEXT NOT NULL REFERENCES orgs(id) ON DELETE CASCADE;
ALTER TABLE groups DROP CONSTRAINT groups_name_key;
ALTER TABLE groups ADD UNIQUE (org_id, name);
```

### Tenant tables — add NOT NULL `org_id`

Strict isolation. Each table below gets:

```sql
ALTER TABLE <t> ADD COLUMN org_id TEXT NOT NULL REFERENCES orgs(id) ON DELETE CASCADE;
```

Plus an index rework to put `org_id` first in the most-used access patterns:

| Table                    | Existing index → replacement                                       |
| ------------------------ | ----------------------------------------------------------------- |
| `channels`               | `idx_channels_org_enabled (org_id, enabled)` replaces `idx_channels_enabled` |
| `channel_models`         | `idx_channel_models_org_channel (org_id, channel_id)`              |
| `api_keys`               | `idx_api_keys_org (org_id)` + `idx_api_keys_hash` stays (still global) |
| `usage_records`          | `idx_usage_org_date (org_id, created_at)` + `idx_usage_org_key_date (org_id, key_id, created_at)` |
| `audit_logs`             | `idx_audit_org_date (org_id, created_at)` + `idx_audit_org_key_date (org_id, key_id, created_at)` + new `actor_is_platform_admin BOOLEAN NOT NULL DEFAULT false` column |
| `accounts`               | unique on `(org_id, user_id)` (currently `user_id` alone)         |
| `transactions`           | `idx_transactions_org (org_id)`                                    |
| `key_model_rate_limits`  | no extra index (PK is `(key_id, model_id)`, both org-scoped via parent) |

### Catalog tables — add nullable `owner_org_id`

Hybrid: NULL = platform-level (visible to all orgs), non-NULL = org-private (visible only to that org).

```sql
ALTER TABLE providers        ADD COLUMN owner_org_id TEXT REFERENCES orgs(id) ON DELETE CASCADE;
ALTER TABLE models           ADD COLUMN owner_org_id TEXT REFERENCES orgs(id) ON DELETE CASCADE;
ALTER TABLE pricing_policies ADD COLUMN owner_org_id TEXT REFERENCES orgs(id) ON DELETE CASCADE;
ALTER TABLE provider_models  ADD COLUMN owner_org_id TEXT REFERENCES orgs(id) ON DELETE CASCADE;
```

All four stay nullable — existing rows default to NULL (platform-level), which matches reality (GPT-4, OpenAI as a provider, `per_token` policy are all universal catalog entries).

**Partial UNIQUE indexes** enforce name stability without allowing orgs to shadow platform entries:

```sql
-- Platform-level: name globally unique
CREATE UNIQUE INDEX models_platform_name_unique
    ON models(name) WHERE owner_org_id IS NULL;
CREATE UNIQUE INDEX providers_platform_slug_unique
    ON providers(slug) WHERE owner_org_id IS NULL;
CREATE UNIQUE INDEX pricing_policies_platform_name_unique
    ON pricing_policies(name) WHERE owner_org_id IS NULL;

-- Org-level: name unique within the org
CREATE UNIQUE INDEX models_org_name_unique
    ON models(owner_org_id, name) WHERE owner_org_id IS NOT NULL;
CREATE UNIQUE INDEX providers_org_slug_unique
    ON providers(owner_org_id, slug) WHERE owner_org_id IS NOT NULL;
CREATE UNIQUE INDEX pricing_policies_org_name_unique
    ON pricing_policies(owner_org_id, name) WHERE owner_org_id IS NOT NULL;
```

The existing global `UNIQUE(name)` / `UNIQUE(slug)` constraints on these tables are dropped (replaced by the partial indexes above).

**Anti-shadowing:** storage trait's create functions reject an org-level entry whose name matches an existing platform-level entry, returning `CatalogNameReserved`. Without this, an org could create a model named `gpt-4` that shadows the real one for its members.

**Visibility filter** (encapsulated in storage trait, never written by hand at call sites):

```rust
// crates/storage/src/lib.rs
fn catalog_scope(org_id: &str) -> String {
    // Used as: WHERE owner_org_id IS NULL OR owner_org_id = $1
    // Bind org_id as the parameter alongside the SQL fragment.
}
```

All `list_*` / `get_*` methods on catalog tables use this filter.

### Cross-references from tenant tables to catalog

`channels.provider_id`, `channel_models.model_id`, `channel_models.cost_policy_id`, `channel_models.pricing_policy_id`, and `models.pricing_policy_id` may now point to either a platform-level or org-private catalog row. The FK itself doesn't change (it's still on the `id` PK), but storage trait inserts validate a row-level invariant:

> `channel.org_id` must equal `model.owner_org_id`, **or** `model.owner_org_id` must be NULL.

Same for the other references. Enforced in code at insert/update time; a CHECK constraint can't express "matches my own org_id OR they are NULL" cleanly.


### `settings` split

```sql
-- existing table renamed
ALTER TABLE settings RENAME TO platform_settings;

-- new per-org table
CREATE TABLE org_settings (
    org_id TEXT NOT NULL REFERENCES orgs(id) ON DELETE CASCADE,
    key    TEXT NOT NULL,
    value  TEXT NOT NULL,
    PRIMARY KEY (org_id, key)
);
```

Existing settings rows (e.g., `allow_registration`, `currency`) stay in `platform_settings`. The current code reads from `settings`; rename requires updating the read-site code.

**Key→scope mapping (decided in code, not DB):**
- `platform_settings`: `allow_registration`, `currency`, `audit_retention_hours_default`
- `org_settings`: `audit_retention_hours`, `default_rate_limit_rpm`, any future per-org config

## Migration

### File: `migrations/postgres/20260708000000_saas_orgs.sql`

Single transaction. Failure on any step rolls back the entire migration.

```sql
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
```

### Rollback

Down migration `20260708000000_saas_orgs.down.sql`:

```sql
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

-- Restore settings table name
ALTER TABLE platform_settings RENAME TO settings;

-- Drop members, orgs, org_settings
DROP TABLE members;
DROP TABLE org_settings;
DROP TABLE orgs;

-- Restore groups global name uniqueness
ALTER TABLE groups DROP CONSTRAINT groups_org_name_unique;
ALTER TABLE groups ADD CONSTRAINT groups_name_key UNIQUE (name);
```

**Down migration is for emergencies only.** It loses all org-scoping; multi-org data cannot be merged back. Document this in the release notes.

## Storage Layer Changes

### `crates/storage/src/types.rs`

New types:
```rust
pub struct Org { id, slug, name, owner_id, created_at, updated_at }
pub struct CreateOrg { slug, name, owner_id }
pub struct UpdateOrg { name: Option<String>, slug: Option<String> }  // slug rename allowed if not taken

pub enum MemberRole { Owner, Admin, Member }
pub enum PlatformRole { PlatformAdmin }

pub struct Member { user_id, org_id, role, group_id, created_at, created_by }
pub struct Membership { org: Org, member: Member, user: User }  // for list responses

pub struct OrgContext {
    pub org_id: String,
    pub org_slug: String,
    pub member_role: MemberRole,
    pub platform_role: Option<PlatformRole>,
    pub group_id: Option<String>,
}
```

Modified types — split by scope:

**Tenant types** (every struct gains `org_id: String`):
- `Channel`, `ChannelModel`, `ApiKey`, `UsageRecord`, `AuditLog`, `Account`, `Transaction`, `KeyModelRateLimit`, `Group`
- Their `Create*` / `Update*` counterparts as appropriate (Create typically takes `org_id` explicitly; Update does not allow changing it).
- `AuditLog` additionally gains `actor_is_platform_admin: bool` to record when an action was performed under platform-admin impersonation.

**Catalog types** (every struct gains `owner_org_id: Option<String>`):
- `Provider`, `Model`, `PricingPolicy`, `ProviderModel`
- `None` = platform-level entry, `Some(org_id)` = org-private.
- `Create*` types take `owner_org_id: Option<String>` explicitly. `Update*` types may not change it (scope is immutable post-create — moving between platform and org-private would invalidate references).

`User` gains `current_org_id: Option<String>`, `platform_role: Option<PlatformRole>`. Loses `role: String` and `group_id: Option<String>`.

### `crates/storage/src/lib.rs` — Storage trait

**Tenant methods** all gain an `org_id: &str` parameter:

```rust
async fn list_channels(&self, org_id: &str) -> Result<Vec<Channel>>;
async fn get_channel(&self, org_id: &str, id: &str) -> Result<Option<Channel>>;
async fn create_channel(&self, org_id: &str, channel: CreateChannel) -> Result<Channel>;
// ... and so on for all 9 tenant tables
```

**Catalog methods** take an `org_id: &str` parameter that drives the visibility filter, plus distinguish platform vs. org-scoped writes:

```rust
// Visible entries: WHERE owner_org_id IS NULL OR owner_org_id = $1
async fn list_providers(&self, viewer_org_id: &str) -> Result<Vec<Provider>>;
async fn list_models(&self, viewer_org_id: &str) -> Result<Vec<Model>>;
async fn list_pricing_policies(&self, viewer_org_id: &str) -> Result<Vec<PricingPolicy>>;
async fn list_provider_models(&self, viewer_org_id: &str) -> Result<Vec<ProviderModel>>;

async fn get_model(&self, viewer_org_id: &str, id: &str) -> Result<Option<Model>>;
// ... get_* for the other three

// Writes: owner_org_id taken from the Create struct.
// Anti-shadowing check: org-private create rejected if name/slug matches a platform entry.
async fn create_model(&self, viewer_org_id: &str, model: CreateModel) -> Result<Model>;
async fn update_model(&self, viewer_org_id: &str, id: &str, updates: UpdateModel) -> Result<Model>;
async fn delete_model(&self, viewer_org_id: &str, id: &str) -> Result<()>;
// ... CUD for providers, pricing_policies, provider_models

// Permission helpers (not on Storage trait, but called from handlers):
// - is_platform_admin(ctx) -> can mutate platform-level entries
// - catalog_scope(org_id) -> SQL fragment "owner_org_id IS NULL OR owner_org_id = $1"
```

Catalog mutations are gated by `org` crate access functions:
- `can_create_platform_catalog(ctx)` — only `platform_admin`
- `can_create_org_catalog(ctx)` — `members.role` in [admin, owner] for the org
- `can_mutate_catalog_entry(ctx, entry)` — platform_admin if entry.owner_org_id is None; org admin+ if Some(ctx.org_id)

New methods for org/membership management:

```rust
// Orgs
async fn get_org_by_slug(&self, slug: &str) -> Result<Option<Org>>;
async fn get_org(&self, id: &str) -> Result<Option<Org>>;
async fn list_orgs_for_user(&self, user_id: &str) -> Result<Vec<Membership>>;
async fn create_org(&self, org: CreateOrg) -> Result<Org>;
async fn update_org(&self, id: &str, updates: UpdateOrg) -> Result<Org>;
async fn delete_org(&self, id: &str) -> Result<()>;

// Members
async fn get_member(&self, user_id: &str, org_id: &str) -> Result<Option<Member>>;
async fn list_members(&self, org_id: &str) -> Result<Vec<Member>>;
async fn upsert_member(&self, member: Member) -> Result<Member>;
async fn update_member_role(&self, user_id: &str, org_id: &str, role: MemberRole) -> Result<()>;
async fn delete_member(&self, user_id: &str, org_id: &str) -> Result<()>;
async fn count_owners(&self, org_id: &str) -> Result<i64>;  // for "last owner" guard

// Settings (split)
async fn get_platform_setting(&self, key: &str) -> Result<Option<String>>;
async fn set_platform_setting(&self, key: &str, value: &str) -> Result<()>;
async fn get_org_setting(&self, org_id: &str, key: &str) -> Result<Option<String>>;
async fn set_org_setting(&self, org_id: &str, key: &str, value: &str) -> Result<()>;
async fn list_org_settings(&self, org_id: &str) -> Result<Vec<(String, String)>>;
```

### `crates/auth/src/jwt.rs` — Claims

```rust
#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,             // user_id
    pub username: String,
    pub current_org_id: String,  // NEW — required (after Phase 2)
    pub platform_role: Option<String>,
    pub exp: usize,
}
```

`create_jwt` signature gains `current_org_id` and `platform_role` parameters.

## API Surface

### Global endpoints (no org context)

```
POST   /api/v1/auth/login              username+password → { token, refresh_token, user, current_org, orgs }
POST   /api/v1/auth/register           username+password → auto-creates personal org → { token, refresh_token, user, current_org }
POST   /api/v1/auth/refresh            refresh_token → { token, refresh_token }
GET    /api/v1/me                      current user + current_org + orgs list
POST   /api/v1/me/current-org          body: { org_id | org_slug } → switch, returns new tokens
GET    /api/v1/orgs                    list orgs the user is a member of
POST   /api/v1/orgs                    create new org (caller becomes owner)
```

### Org-scoped endpoints

All existing management endpoints move under `/api/v1/{org_slug}/...`:

```
GET    /api/v1/{org_slug}
PATCH  /api/v1/{org_slug}              update name/slug (admin+)
DELETE /api/v1/{org_slug}              delete org (owner only, requires password confirmation)

GET    /api/v1/{org_slug}/members
POST   /api/v1/{org_slug}/members      invite (by username)
PATCH  /api/v1/{org_slug}/members/{user_id}    change role
DELETE /api/v1/{org_slug}/members/{user_id}    remove member

GET    /api/v1/{org_slug}/keys         (was GET /api/v1/keys)
POST   /api/v1/{org_slug}/keys
...channels, providers, models, usage, audit, accounts, settings, groups...
```

### Proxy API (unchanged URL)

```
POST   /v1/chat/completions            Bearer or x-api-key → resolve key.org_id → all downstream uses org_id
POST   /v1/messages                    x-api-key → resolve key.org_id
POST   /v1/embeddings                  (if present)
```

### Permission helpers (`crates/org/src/access.rs`)

```rust
pub fn can_manage_org_settings(ctx: &OrgContext) -> bool {
    matches!(ctx.member_role, MemberRole::Owner | MemberRole::Admin)
        || ctx.platform_role == Some(PlatformRole::PlatformAdmin)
}

pub fn can_invite_members(ctx: &OrgContext) -> bool { /* admin+ or platform_admin */ }
pub fn can_delete_org(ctx: &OrgContext) -> bool { /* owner only or platform_admin */ }
pub fn can_manage_channels(ctx: &OrgContext) -> bool { /* admin+ or platform_admin */ }
pub fn can_access_channel(ctx: &OrgContext, channel_group_id: Option<&str>) -> bool {
    // admin/owner/platform_admin → all channels
    // member → channels in their group + ungrouped channels
}
```

### Org switch flow

```
POST /api/v1/me/current-org  { "org_slug": "acme" }

1. SELECT member JOIN org WHERE user_id=$1 AND org.slug=$2
   not found → 403 "not a member of this org"
2. UPDATE users SET current_org_id=$2 WHERE id=$1
3. Reissue access_token + refresh_token with new current_org_id claim
4. Return { token, refresh_token, current_org: {...} }
```

### Platform admin impersonation flow

```
When platform_admin hits /api/v1/{org_slug}/...:
1. MembershipLayer sees users.platform_role = 'platform_admin', no member row exists
2. INSERT INTO members (user_id, org_id, role='admin', created_by='system')
   ON CONFLICT DO UPDATE SET role='admin', created_by='system'
3. Inject OrgContext with member_role=admin, platform_role=Some(PlatformAdmin)
4. Handler executes normally — no special branches

On session end (token expiry or explicit logout):
   A janitor task periodically removes members WHERE created_by='system'
   AND no recent activity (last_seen older than threshold).
   For Phase 2 we accept that temp rows persist between requests in the same session.
```

**Audit log:** the `audit_logs.actor_is_platform_admin` column (added by the migration) records whether an action was performed under platform-admin impersonation. Audit writers set `actor_is_platform_admin = true` whenever `OrgContext.platform_role == Some(PlatformAdmin)`.

## Frontend Changes

### Routes (React Router)

```
/login                                  no org context
/register                               no org context
/:orgSlug                               → redirect to /:orgSlug/dashboard
/:orgSlug/dashboard
/:orgSlug/keys
/:orgSlug/providers
/:orgSlug/channels
/:orgSlug/models
/:orgSlug/usage
/:orgSlug/logs
/:orgSlug/members          ← new
/:orgSlug/settings         ← new (org settings)
/:orgSlug/accounts
/settings                  ← user personal settings (no org prefix)
```

`RouteGuard` component:
1. Checks `orgSlug` is in `useAuthStore.orgs` → else redirect to `/${currentOrgSlug}`
2. If `orgSlug !== currentOrg.slug` → call `setCurrentOrg(orgSlug)` first

### Auth store (Zustand)

```ts
interface AuthState {
  user: { id, username, platform_role } | null
  currentOrg: { id, slug, name, role } | null    // NEW
  orgs: Org[]                                     // NEW
  token, refreshToken

  setCurrentOrg(slug: string): Promise<void>     // calls /me/current-org, updates tokens
  refreshOrgs(): Promise<void>                    // calls /orgs
}
```

Login response shape changes from `{ token, refresh_token, user }` to `{ token, refresh_token, user, current_org, orgs }`.

### Org switcher component (sidebar top)

```
┌─────────────────────────┐
│ [logo] Acme Inc.    ▼   │
├─────────────────────────┤
│   Personal              │
│   Acme Inc.      ✓      │
│   ──────────────        │
│   + Create org          │
│   ⚙ Org settings        │
└─────────────────────────┘
```

Switching:
1. `POST /api/v1/me/current-org { org_slug }`
2. Update store (token, currentOrg)
3. `queryClient.clear()` — prevents cross-org data leakage
4. `navigate(`/${newSlug}/dashboard`)`

### API client

Explicit org prefix per call (no hidden interceptor):

```ts
function orgPrefix(): string {
  const slug = useAuthStore.getState().currentOrg?.slug
  if (!slug) throw new Error('no current org')
  return `/api/v1/${slug}`
}

export async function listKeys(): Promise<Key[]> {
  const { data } = await api.get(`${orgPrefix()}/keys`)
  return data
}
```

Global endpoints (`/api/v1/orgs`, `/api/v1/me/*`, `/api/v1/auth/*`) skip the prefix.

### React Query keys

Every key gets the org slug as the first element:

```ts
useQuery({ queryKey: [orgSlug, 'keys'], ... })
useQuery({ queryKey: [orgSlug, 'channels'], ... })
```

Even if `queryClient.clear()` is missed on org switch, mismatched keys prevent stale data from rendering.

### New pages

**`/:orgSlug/members`** — table of members (username, role, joined_at, last_active). Invite button (modal: enter username, choose role). Inline role-change and removal dropdowns. Last-owner removal is rejected with a clear error.

**`/:orgSlug/settings`** — org name/slug edit (admin+). Delete org button (owner only) requires typing the slug to confirm. Danger zone: transfer ownership.

**`/settings`** (no org prefix) — user's personal settings: change password, theme, language.

## Testing Strategy

### Backend unit tests

**`crates/org` access rules** — table-driven:

```rust
#[test_case(MemberRole::Owner,  Action::DeleteOrg,        true)]
#[test_case(MemberRole::Admin,  Action::DeleteOrg,        false)]
#[test_case(MemberRole::Member, Action::InviteUser,       false)]
#[test_case(MemberRole::Member, Action::ViewChannel,      true  /* ungrouped or own group */)]
#[test_case(MemberRole::Member, Action::ViewChannelOther, false /* different group */)]
#[test_case(MemberRole::Admin,  Action::ViewChannelOther, true)]
fn test_access(role: MemberRole, action: Action, expected: bool) { ... }
```

### Integration tests (sqlx::test fixtures)

**Org isolation** — two orgs, two keys, verify no cross-visibility:

```rust
#[sqlx::test(fixtures("two_orgs_seed"))]
async fn proxy_key_cannot_route_to_other_org_channels(state: AppState) {
    let key_a = create_api_key(org_a_id);
    let _channel_b_only = create_channel_in_org(org_b_id, model = "gpt-4");
    
    let resp = state.proxy()
        .bearer(key_a)
        .body(model = "gpt-4")
        .post("/v1/chat/completions").await;
    
    assert!(resp.routing_did_not_use_org_b_channel);
}
```

**Management API isolation** — user A's token cannot list org B's keys:

```rust
#[sqlx::test(fixtures("two_orgs_seed"))]
async fn list_keys_excludes_other_orgs(state: AppState) {
    let resp = state.api()
        .bearer(user_a_token)  // current_org_id = org_a
        .get(&format!("/api/v1/{}/keys", org_a_slug)).await;
    assert_eq!(resp.body, /* only org_a keys */);
    
    let resp = state.api()
        .bearer(user_a_token)  // not a member of org_b
        .get(&format!("/api/v1/{}/keys", org_b_slug)).await;
    assert_eq!(resp.status, 403);
}
```

**Platform admin impersonation**: platform_admin hits org-scoped endpoint → temp member row created → after logout, row cleaned up by janitor.

**Last-owner guard**: removing the only owner is rejected with `LastOwner` error.

**Org-private catalog isolation** — org A's private model is invisible to org B; both see platform-level models:

```rust
#[sqlx::test(fixtures("two_orgs_seed"))]
async fn org_private_model_invisible_to_other_orgs(state: AppState) {
    let _platform_model = create_model(owner_org_id = None, name = "gpt-4");
    let _private_a      = create_model(owner_org_id = org_a_id, name = "my-finetune");

    let visible_to_a = state.storage.list_models(org_a_id).await?;
    let visible_to_b = state.storage.list_models(org_b_id).await?;

    assert!(visible_to_a.iter().any(|m| m.name == "gpt-4"));
    assert!(visible_to_a.iter().any(|m| m.name == "my-finetune"));
    assert!(visible_to_b.iter().any(|m| m.name == "gpt-4"));
    assert!(!visible_to_b.iter().any(|m| m.name == "my-finetune"));
}
```

**Anti-shadowing**: org attempting to create a model named `gpt-4` when a platform-level `gpt-4` exists → `CatalogNameReserved` error.

**Migration safety**: run migration forward on a seeded test DB, assert no orphan rows. Roll back, verify original schema restored. Run forward again — idempotent for the schema parts.

### Frontend tests

- **Vitest unit** — OrgSwitcher component: clicking an org calls `setCurrentOrg`, triggers `queryClient.clear()` and navigate.
- **Vitest integration** — AuthStore: login populates `currentOrg` + `orgs`; switch updates tokens.
- **Playwright e2e** — login → dashboard → create second org → switch → verify empty dashboard (no cross-data) → invite a second user → second user can see the org in their switcher.

### Security review checklist (append to PR description)

- [ ] Every Storage call in handlers passes `org_id` from `OrgContext` / `ApiKeyContext`, never from request body.
- [ ] Proxy routing/usage/audit/billing paths filter `WHERE org_id = $1` using the resolved key's org.
- [ ] No `SELECT ... WHERE id = $1` without `AND org_id = $2` on tenant tables.
- [ ] Catalog queries all use the `(owner_org_id IS NULL OR owner_org_id = $1)` filter via the storage trait helper — no hand-written catalog SQL at handler level.
- [ ] Catalog mutations check `can_mutate_catalog_entry(ctx, entry)` before writing.
- [ ] Migration self-check `DO $$` block passes on a representative data sample.
- [ ] Refresh-token flow re-validates membership: if user was removed from `current_org_id` since the access token was issued, refresh is rejected and the client is forced to re-login.
- [ ] Platform-admin temp member rows include `created_by='system'`; audit log records both user_id and the fact of platform-admin action.
- [ ] Slug uniqueness enforced at DB level (UNIQUE constraint); race on concurrent org-create resolves via `ON CONFLICT DO NOTHING` + 409 response.
- [ ] Anti-shadowing: org-private catalog create rejected when name/slug matches an existing platform-level entry.

## Phasing

### Phase 1 — Schema + backend org-awareness (target: v1.9.0)

**Behavioral contract:** externally invisible. Existing frontend and existing API clients continue to work without changes.

**Deliverables:**
- Migration `20260708000000_saas_orgs.sql` (+ down migration).
- New crate `org` (types, context, extractors, access rules).
- Storage trait: tenant methods take `org_id`; catalog methods take `viewer_org_id` and apply the `(owner_org_id IS NULL OR owner_org_id = $1)` filter. For Phase 1, all callers pass the user's `current_org_id` (always the default org), and all catalog entries are platform-level (`owner_org_id IS NULL`).
- All API handlers read `org_id` from the authenticated user's `current_org_id` — no URL change yet.
- JWT claims include `current_org_id` + `platform_role`.
- New endpoints (under existing `/api/v1/...` paths): `/api/v1/orgs` (list), `/api/v1/me/current-org` (switch).
- Frontend: OrgSwitcher added but only one org is visible (the default). No new routes yet.

**Verification:** all existing tests pass (with updated fixtures). New tests: migration self-check, OrgContext injection, role helper functions.

### Phase 2 — Multi-org routing + URL switch (target: v1.10.0)

**Behavioral contract:** breaking change for API consumers. URLs change from `/api/v1/keys` to `/api/v1/{org_slug}/keys`. Frontend fully updated. Document in CHANGELOG with migration guide.

**Deliverables:**
- Management API moved to `/api/v1/{org_slug}/...`.
- Middleware chain: AuthLayer → OrgResolveLayer → MembershipLayer.
- Platform-admin impersonation via temp member row + janitor cleanup.
- Frontend: `/:orgSlug/*` routes, OrgSwitcher fully functional, React Query keys prefixed with orgSlug, API client uses `orgPrefix()` helper.
- `POST /api/v1/orgs` create-org endpoint.
- Members page + Org Settings page.
- Org-private catalog CRUD: org admin+ can create providers/models/pricing_policies with `owner_org_id = <their org>` via the existing endpoints. UI surfaces a "Platform" vs "Ours" filter on catalog listing pages.
- Anti-shadowing: storage trait rejects org-private creates whose name/slug matches an existing platform-level entry (`CatalogNameReserved`).
- Old `/api/v1/{resource}` URLs return 410 Gone with a pointer to the new path (not silent 404).

**Verification:** two-org integration tests pass. Platform admin can impersonate. Last-owner removal rejected. Org-private catalog entry is invisible to other orgs. Anti-shadowing reject works.

### Phase 3 — Signup auto-creates personal org + invitations (target: v1.11.0)

**Behavioral contract:** new users get a personal org on signup. Existing invitation flow.

**Deliverables:**
- `/api/v1/auth/register` creates a personal org (slug derived from username; on collision append `-2`, `-3`, etc.). User becomes owner. `current_org_id` set.
- `POST /api/v1/{org_slug}/members` invites by username. If user exists → add member. If not → return invite link (Phase 3 leaves email delivery out; in-app notification only).
- Onboarding wizard for first-time users (skip if `user.orgs.length > 0`).
- "Accept invitation" flow via deep link `/accept-invite?token=...`.

**Verification:** fresh signup lands in personal org dashboard. Invitation accepted → member appears in members list. Rejecting an invitation → token invalidated.

## Out of Scope / Future Work

- **Subdomain routing** (`acme.llmgateway.com`) — explicitly declined for v1; revisit if customers ask.
- **SCIM / SAML / SSO** — enterprise identity, separate spec.
- **Email delivery** — Phase 3 invitations use in-app notifications only.
- **Cross-org billing aggregation** — each org has independent accounts; no consolidated invoicing.
- **Per-org rate-limit overrides** — Phase 4+ if needed.
- **Org-level audit retention** — `org_settings` supports it but the implementation is left to a later change.
- **API key scopes** (`scope: "this org only"` vs `scope: "any org the user belongs to"`) — currently keys are strictly per-org; cross-org keys are a future feature.
- **Soft delete for orgs** — current design hard-deletes via `ON DELETE CASCADE`. Soft delete + audit trail is a separate concern.
