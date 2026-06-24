# User Groups Design

**Date:** 2026-06-24
**Status:** Approved by user (in conversation) — pending writing-plans

## Goal

Add a `groups` concept to the system so that an admin can constrain which channels a user can access. Currently channels have a free-form `group` tag with no relation to users. This adds a single canonical `groups` table referenced by both users and channels, with the access rule "a user can only access channels in the same group".

The existing channel-grouping UX (filter, badge, edit modal) is preserved — only the data shape changes from a free-form text column to a foreign key.

## Decisions locked from brainstorming

1. **Single `groups` table** (one source of truth, not two parallel tables for users vs channels).
2. **Refactor `channels.group` from TEXT to FK** — one-time migration creates `groups` rows from existing distinct values.
3. **Access model:** `users.group_id == channels.group_id` (and both non-null). Multi-group membership is not in scope.
4. **Default behavior:** "unset = unrestricted". User with no group sees all channels; channel with no group is accessible to all users.
5. **Admin role bypasses the filter.**
6. **Cache-hit and cache-miss routing paths both apply the filter.**
7. **Delete-group is permissive** — references become NULL via `ON DELETE SET NULL`, response returns counts so the UI can warn.

## Data model

### New table: `groups`

| Column        | Type        | Notes                                |
| ------------- | ----------- | ------------------------------------ |
| `id`          | TEXT PK     | UUID v4 string                       |
| `name`        | TEXT UNIQUE NOT NULL | Display name; case-sensitive (PostgreSQL default for TEXT equality). |
| `description` | TEXT NULL   |                                      |
| `created_at`  | TIMESTAMPTZ |                                      |
| `updated_at`  | TIMESTAMPTZ |                                      |

### `users` table changes

- Add `group_id TEXT NULL REFERENCES groups(id) ON DELETE SET NULL`

### `channels` table changes

- Drop existing `group TEXT` column
- Add `group_id TEXT NULL REFERENCES groups(id) ON DELETE SET NULL`

### Migration (`migrations/postgres/<ts>_user_groups.sql`)

Single transaction. Requires PostgreSQL 13+ (for built-in `gen_random_uuid()`). Steps in order:

1. `CREATE TABLE groups (...)`
2. Backfill from existing channel-group values (idempotent via `ON CONFLICT (name) DO NOTHING`, so re-running is safe):
   ```sql
   INSERT INTO groups (id, name)
   SELECT gen_random_uuid()::text, "group"
   FROM (SELECT DISTINCT "group" FROM channels WHERE "group" IS NOT NULL) t
   ON CONFLICT (name) DO NOTHING;
   ```
3. `ALTER TABLE channels ADD COLUMN group_id TEXT REFERENCES groups(id);`
4. Backfill (case-sensitive equality, matching PostgreSQL TEXT default):
   ```sql
   UPDATE channels c SET group_id = g.id
   FROM groups g
   WHERE c."group" = g.name;
   ```
5. Verify row counts match:
   ```sql
   -- Must return true (or 0 mismatches):
   SELECT COUNT(*) FROM channels WHERE "group" IS NOT NULL
     AND group_id IS NULL;
   ```
6. `ALTER TABLE channels DROP COLUMN "group";`
7. `ALTER TABLE users ADD COLUMN group_id TEXT REFERENCES groups(id) ON DELETE SET NULL;` (no backfill — existing users default to unrestricted).

## Storage layer (`crates/storage/src/`)

### `types.rs`

- New: `Group { id, name, description, created_at, updated_at }`, `CreateGroup { name, description }`, `UpdateGroup { name?, description? }`
- `User` gains `group_id: Option<String>`. `UserWithBalance` gains `group_id: Option<String>` and `group_name: Option<String>`.
- `Channel` replaces `group: Option<String>` with `group_id: Option<String>`. `CreateChannel` and `UpdateChannel` likewise.
- `CreateUser` / `UpdateUser` gain optional `group_id` (None = unset on create; `Option<Option<String>>` on update for keep/clear/set semantics — consistent with existing patterns like `pricing_policy_id`).

### `lib.rs` (Storage trait)

New methods:

- `list_groups() -> Vec<Group>`
- `get_group(id) -> Option<Group>`
- `create_group(input) -> Group` (returns error on duplicate name)
- `update_group(id, input) -> Group`
- `delete_group(id) -> DeleteGroupResult { cleared_users: i64, cleared_channels: i64 }`
- `get_user_group_id(user_id) -> Option<String>` — hot-path lookup, returns the user's `group_id` (None if user has no group or doesn't exist).

Modified methods:

- `update_user` accepts optional `group_id: Option<Option<String>>`
- `update_channel` accepts optional `group_id: Option<Option<String>>`
- `list_users` / `get_user` return `group_name` joined from `groups`
- `list_channels` / `get_channel` return `group_name` joined from `groups`
- `list_channels` keeps returning the full set; the routing filter is applied above storage.

### `postgres.rs`

- All group CRUD SQL
- `get_user_group_id` as a single-row SELECT
- Join in user/channel queries to populate `group_name`
- DROP the `channels.group` from SELECT/INSERT/UPDATE statements; add `group_id` instead
- The reserved-word quoting `"group"` is no longer needed and should be removed

## API layer (`crates/api/src/`)

### New module: `management/groups.rs`

Registered in `management/mod.rs`. Endpoints (all behind existing admin auth middleware):

```
GET    /admin/groups
POST   /admin/groups
GET    /admin/groups/:id
PATCH  /admin/groups/:id
DELETE /admin/groups/:id
```

Response shapes follow existing patterns (`{ item }`, `{ items, total }`).

### Modified existing endpoints

- `PATCH /admin/users/:id` — body accepts optional `group_id: string | null`. `null` clears.
- `PATCH /admin/channels/:id` — body accepts optional `group_id` (replaces legacy `group` field). Legacy `group` is removed.
- `GET /admin/users` and `GET /admin/users/:id` — response includes `group_id` and `group_name`.
- `GET /admin/channels` and `GET /admin/channels/:id` — response includes `group_id` and `group_name` (replaces `group`).

### Routing filter (`crates/api/src/proxy.rs`)

Add a `RequestContext { user_id: Option<String>, is_admin: bool }` on request extensions (built in the auth middleware, populated from JWT claims and `api_key.created_by`).

In `try_routing`, after `available_channels` is built:

```rust
if let Some(ctx) = request_context {
    if !ctx.is_admin {
        if let Some(allowed) = state.storage.get_user_group_id(&ctx.user_id).await {
            available_channels.retain(|(_, ch)| {
                ch.group_id.is_none() || ch.group_id.as_deref() == Some(&allowed)
            });
        }
    }
}
```

Apply to **both** the cache-hit path (~line 940) and the cache-miss path (~line 975).

`ResolvedChannel` (proxy.rs:227) gains `pub group_id: Option<String>`, populated in `do_reload` so the cached path has the data without a storage roundtrip.

## Frontend (`web/src/`)

### New page: `pages/Groups.tsx`

- Table: name, description, # users, # channels, actions (edit, delete)
- Create drawer: `name` (required, unique), `description` (optional)
- Edit drawer: same fields, shows "Used by N users / M channels"
- Delete confirmation: shows the counts, warns that references will be cleared
- i18n keys: `groups.{title, description, table.{name,description,users,channels,actions}, createModal.*, editModal.*, deleteConfirm}`

### `components/Layout.tsx`

Add a "Groups" entry to the admin sidebar.

### `pages/Users.tsx`

Add group selector inside the existing `UserDrawer` (keeps the table narrow). Drawer header shows "Group: <name>" with an inline `Select` (options = all groups + "None"). On change → `PATCH /admin/users/:id` with `{ group_id }` or `{ group_id: null }`.

### `pages/Channels.tsx` + `pages/ChannelDetail.tsx`

Replace the free-form group text input with a `Select` of existing groups + "None". Badge on `ChannelRow` reads from `channel.group_name`.

### Shared infra

- New hook: `hooks/useGroups.ts` (mirrors `useUsers.ts` shape: `useGroups()`, `useCreateGroup`, `useUpdateGroup`, `useDeleteGroup`)
- New API file: `api/groups.ts` (mirror `api/auth.ts`)
- `types.ts`: add `Group`, `CreateGroupRequest`, `UpdateGroupRequest`. Update `User`, `Channel`, `CreateUserRequest`, `UpdateUserRequest`, `CreateChannelRequest`, `UpdateChannelRequest`: replace `group` with `group_id` + `group_name`.

## Error handling & edge cases

- **Duplicate group name** on create/update → `409 Conflict`
- **Setting non-existent `group_id`** on user/channel PATCH → `400 BadRequest` with "group not found"
- **Deleting a group in use** → succeeds, response includes `cleared_users` and `cleared_channels` counts
- **Routing hot path storage failure** on `get_user_group_id` → log warning, fail-open (no filter applied). Same philosophy as `do_reload` failure handling.
- **No candidates after filter** → same `404 "No enabled channels for model 'X'"` as today. No new error type.
- **`api_key.created_by = None`** → no filter (backward compatible with legacy admin-created keys).
- **Admin role** → bypass filter.

## Testing strategy

### Backend (`crates/api/tests/`)

- `test_user_groups.rs`:
  - Group CRUD: create, list, get, update, delete
  - Duplicate name → 409
  - Delete group → references become NULL, response includes counts
  - User PATCH with `group_id` and `group_id: null`
  - Channel PATCH with `group_id`
  - Setting non-existent `group_id` → 400
- Routing tests (extend existing routing tests or add new file):
  - User in group X can only access channels with `group_id = X` (and ungrouped channels)
  - User with no group sees all channels
  - User in group with no matching channels sees only ungrouped channels
  - Admin user sees all channels regardless of group
  - User in group X accessing model only available in group Y → 404
- Migration test: fixture with pre-existing `channels.group` values → migration produces correct `groups` rows, `group_id` backfill matches, old column dropped.

### Frontend (`web/src/`)

- `pages/Groups.test.tsx`:
  - Renders list
  - Create drawer requires unique name; duplicate shows backend error
  - Edit drawer updates name
  - Delete shows confirmation with usage counts
- Extend `pages/Users.test.tsx`:
  - Changing group via drawer sends correct PATCH (mock + assert request body)
- Extend `pages/Channels.test.tsx` and `pages/ChannelDetail.test.tsx`:
  - Group badge displays `group_name`
  - Edit modal uses Select (not free-form input)

### Release / manual

- Migration dry-run on a copy of production DB before tagging
- End-to-end smoke: log in as non-admin user in group X → API call → verify it lands on a channel in group X; switch user to group Y → same model returns a different channel or 404
- Regression: rerun full frontend test suite + `cargo test --workspace`

## Out of scope

- Multi-group membership per user (1 user = 1 group)
- Multi-group membership per channel (1 channel = 1 group)
- Per-key or per-model access control
- Group-level rate limits or quotas
- Group-level audit/usage report rollups (future enhancement; current usage/audit records still record user_id)
- Renaming existing groups via cascade (handled as a normal PATCH — admin updates the row)