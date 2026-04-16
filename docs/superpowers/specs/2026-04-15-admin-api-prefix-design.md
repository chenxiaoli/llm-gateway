# Admin API Prefix Design

> **Date:** 2026-04-15
> **Feature:** Admin API Prefix

## Goal

Add `/api/v1/admin` prefix to all admin-level management endpoints. Admin-only routes (providers, channels, models, channel_models, users, settings, logs) move under this prefix while maintaining existing `require_admin` authorization.

## Architecture

### Route Changes

| Original Route | New Route |
|----------------|-----------|
| `/api/v1/providers` | `/api/v1/admin/providers` |
| `/api/v1/providers/{id}` | `/api/v1/admin/providers/{id}` |
| `/api/v1/providers/{id}/channels` | `/api/v1/admin/providers/{id}/channels` |
| `/api/v1/providers/{id}/models` | `/api/v1/admin/providers/{id}/models` |
| `/api/v1/providers/{id}/models/{model_name}` | `/api/v1/admin/providers/{id}/models/{model_name}` |
| `/api/v1/providers/{id}/sync-models` | `/api/v1/admin/providers/{id}/sync-models` |
| `/api/v1/channels` | `/api/v1/admin/channels` |
| `/api/v1/channels/{id}` | `/api/v1/admin/channels/{id}` |
| `/api/v1/models` | `/api/v1/admin/models` |
| `/api/v1/providers/{id}/channel-models` | `/api/v1/admin/providers/{id}/channel-models` |
| `/api/v1/channel-models/{id}` | `/api/v1/admin/channel-models/{id}` |
| `/api/v1/users` | `/api/v1/admin/users` |
| `/api/v1/users/{id}` | `/api/v1/admin/users/{id}` |
| `/api/v1/settings` | `/api/v1/admin/settings` |
| `/api/v1/logs` | `/api/v1/admin/logs` |

### Authentication

- All moved endpoints use existing `require_admin` authorization
- No additional permission checks required
- Public endpoints (`/api/v1/auth/*`) remain unchanged
- User endpoints (`/api/v1/keys`, `/api/v1/usage`) remain unchanged

## Implementation Notes

- Backend: Update router registration in `crates/api/src/management/mod.rs`
- Frontend: Update API calls to use new paths
- No database changes required
- JWT token structure unchanged