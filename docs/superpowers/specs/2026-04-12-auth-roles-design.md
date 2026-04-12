# User Authentication & Role-Based Access Design

**Date:** 2026-04-12
**Status:** Approved

## Overview

Replace the single static admin token with a proper username/password authentication system using JWT sessions. Two roles (`admin` and `user`) control access to management pages and API scopes. The sidebar groups pages into "Console" (shared) and "Admin" (admin-only).

## Data Model

### New `users` table

```sql
CREATE TABLE IF NOT EXISTS users (
    id          TEXT PRIMARY KEY,
    username    TEXT NOT NULL UNIQUE,
    password    TEXT NOT NULL,
    role        TEXT NOT NULL DEFAULT 'user' CHECK(role IN ('admin', 'user')),
    enabled     BOOLEAN NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
```

- `password` stores bcrypt hash
- `role` is either `'admin'` or `'user'`
- First registered user always gets `role = 'admin'`

### Modified `api_keys` table

Add `created_by` column to link keys to their creator:

```sql
ALTER TABLE api_keys ADD COLUMN created_by TEXT REFERENCES users(id);
```

- Nullable for backward compatibility with existing keys
- Set automatically on key creation based on authenticated user

### Config changes

Remove `[admin]` section. Add `[auth]` section:

```toml
[auth]
jwt_secret = "change-me-in-production"
allow_registration = true
```

- `jwt_secret`: secret key for signing JWT tokens
- `allow_registration`: system setting that controls whether `/api/v1/auth/register` accepts new registrations. Admin can toggle this from the Settings page.

### AppState changes

Replace `admin_token: String` with:
- `jwt_secret: String` — for JWT signing/verification
- `allow_registration: bool` — cached registration setting (read from DB or config)

## Backend API

### Auth endpoints (no auth required)

**`POST /api/v1/auth/login`**
- Request: `{ "username": string, "password": string }`
- Response: `{ "token": string, "user": { "id": string, "username": string, "role": "admin"|"user" } }`
- Validates credentials against bcrypt hash, returns signed JWT

**`POST /api/v1/auth/register`**
- Request: `{ "username": string, "password": string }`
- Response: `{ "token": string, "user": { "id": string, "username": string, "role": "admin"|"user" } }`
- Only accepted if `allow_registration` is true
- First user in the system always gets `admin` role regardless of setting
- Subsequent users get `user` role
- Returns 403 if registration is disabled

**`GET /api/v1/auth/config`** (public, no auth required)
- Response: `{ "allow_registration": boolean }`
- Used by login page to show/hide "Register" link and by register page to show disabled message

**`GET /api/v1/auth/me`**
- Response: `{ "id": string, "username": string, "role": "admin"|"user", "allow_registration": boolean }`
- Requires valid JWT
- Returns current user info and system settings

### User management endpoints (admin only)

**`GET /api/v1/users`**
- Response: array of `{ "id": "string", "username": string, "role": "admin"|"user", "enabled": boolean, "created_at": string, "updated_at": string }`

**`PATCH /api/v1/users/{id}`**
- Request: `{ "role"?: "admin"|"user", "enabled"?: boolean }`
- Response: updated user object
- Cannot disable the last admin user (return 400)

**`DELETE /api/v1/users/{id}`**
- Returns 204
- Cannot delete the last admin user (return 400)

### Settings endpoints (admin only)

**`GET /api/v1/settings`**
- Response: `{ "allow_registration": boolean }`

**`PATCH /api/v1/settings`**
- Request: `{ "allow_registration": boolean }`
- Response: updated settings

### Modified key endpoints (role-scoped)

**`GET /api/v1/keys`**
- Admin: returns all keys (current behavior)
- User: returns only keys where `created_by = current_user_id`

**`POST /api/v1/keys`**
- Sets `created_by = current_user_id` automatically

**`PATCH /api/v1/keys/{id}`** and **`DELETE /api/v1/keys/{id}`**
- Admin: can modify/delete any key
- User: can only modify/delete keys where `created_by = current_user_id` (returns 403 otherwise)

### Modified provider/usage/logs endpoints

- Providers, Logs: admin only (no change to current access pattern, just switch from token to JWT)
- Usage: admin sees all; user sees only their own keys' usage

## JWT

**Payload:**
```json
{ "sub": "user-id", "role": "admin", "exp": 1234567890 }
```

**Expiration:** 24 hours

**Implementation:** Use the `jsonwebtoken` crate. The `auth` crate gains:
- `hash_password(plain: &str) -> String` — bcrypt hash
- `verify_password(plain: &str, hash: &str) -> bool` — bcrypt verify
- `create_jwt(user_id: &str, role: &str, secret: &str) -> String` — sign token
- `verify_jwt(token: &str, secret: &str) -> Result<Claims, Error>` — validate and extract claims

## Frontend

### Login page (`/admin/login`)

Replace single token input with username + password form. Same card layout. On success, store JWT in localStorage (same `llm_gateway_admin_token` key). Add link to register page if registration is open.

### Register page (`/admin/register`)

New page with username + password + confirm password form. Shows "Registration is disabled" message and disables form if `allow_registration` is false (checked via `GET /api/v1/auth/me` or a public endpoint). On success, redirect to dashboard.

### Sidebar — two groups

```
Console
  Dashboard       /admin/dashboard    (any authenticated user)
  API Keys        /admin/keys         (any authenticated user)
  Usage           /admin/usage        (any authenticated user, scoped)

Admin                                        (admin only)
  Providers       /admin/providers
  Users           /admin/users
  Settings        /admin/settings
  Logs            /admin/logs
```

Uses Ant Design `Menu` with `items` containing `children` groups (sub-menus) or `type: 'group'` labels.

### Layout changes

- Store current user info (id, username, role) via `useQuery(['me'])` calling `GET /api/v1/auth/me`
- Sidebar reads `role` from user data to conditionally render Admin group
- Header shows current username + logout link
- `RequireAuth` component checks for JWT and redirects to login if missing
- Add React context (`AuthContext`) to provide user info and role to all components

### New pages

**Users (`/admin/users`)** — admin only
- Table listing all users with columns: username, role, enabled, created_at
- Actions: toggle enabled, change role (dropdown), delete user
- Uses `GET /api/v1/users`, `PATCH /api/v1/users/{id}`, `DELETE /api/v1/users/{id}`

**Settings (`/admin/settings`)** — admin only
- Toggle switch for `allow_registration`
- Displays current JWT secret (masked) and server info
- Uses `GET /api/v1/settings`, `PATCH /api/v1/settings`

### Modified pages

**Keys** — no visual change. Backend scopes results by role.
**Usage** — no visual change. Backend scopes results by role.

## Migration

- New migration file `migrations/002_users.sql` runs after `init.sql`
- Switch from single `INIT_SQL` to numbered migrations applied in order
- Add `users` table and `created_by` column to `api_keys`
- Existing `admin.token` in config.toml is deprecated. If present, log a warning at startup.
- **Breaking change:** After this update, token-based auth no longer works. First visitor must register to create the admin account.

## Files to modify

**Backend:**
- `crates/auth/Cargo.toml` — add `bcrypt`, `jsonwebtoken` dependencies
- `crates/auth/src/lib.rs` — add password hashing and JWT functions
- `crates/storage/src/types.rs` — add `User`, `CreateUser`, `UpdateUser`, `Settings` types; modify `AppConfig`
- `crates/storage/src/lib.rs` — add user CRUD and settings methods to `Storage` trait
- `crates/storage/src/sqlite.rs` — implement user and settings storage
- `crates/storage/src/migrations.rs` — switch to numbered migrations, add `002_users.sql`
- `crates/api/src/lib.rs` — change `AppState` (replace `admin_token` with `jwt_secret`)
- `crates/api/src/extractors.rs` — replace `verify_admin_token` with `verify_jwt` returning user info
- `crates/api/src/management/mod.rs` — add auth, users, settings routes
- `crates/api/src/management/keys.rs` — scope by role
- `crates/api/src/management/usage.rs` — scope by role
- `crates/api/src/management/logs.rs` — admin only
- `crates/gateway/src/main.rs` — update config loading, remove admin_token from state

**Frontend:**
- `web/src/api/client.ts` — add JWT token management, update interceptors
- `web/src/api/auth.ts` — new: login, register, getMe, getAuthConfig API functions
- `web/src/api/users.ts` — new: listUsers, updateUser, deleteUser
- `web/src/api/settings.ts` — new: getSettings, updateSettings
- `web/src/contexts/AuthContext.tsx` — new: provide user info and role
- `web/src/components/Layout.tsx` — sidebar groups, header with username
- `web/src/pages/Login.tsx` — username/password form
- `web/src/pages/Register.tsx` — new: registration form
- `web/src/pages/Users.tsx` — new: user management table
- `web/src/pages/Settings.tsx` — new: registration toggle
- `web/src/pages/Keys.tsx` — no change needed (backend scopes)
- `web/src/pages/Usage.tsx` — no change needed (backend scopes)
- `web/src/App.tsx` — wrap with AuthContext, add new routes

**Config:**
- `config.toml` — replace `[admin]` with `[auth]`
