# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.13.3] - 2026-05-02

### Fixed
- Add placeholder migration for `20260424000000` — fixes `VersionMissing` crash on startup for databases that already had this migration applied

## [0.13.2] - 2026-05-02

### Added
- `created_by` column on channels table — tracks which admin user created each channel
- `created_by` field in channel API responses

## [0.13.1] - 2026-05-02

### Added
- Channel usage summary API endpoint (`GET /api/v1/usage/channel-summary`) — server-side aggregation of usage_records by channel_id with channel names
- `ChannelUsageSummaryRecord` storage type and `query_channel_usage_summary` method for SQLite and PostgreSQL
- Frontend `useChannelUsageSummary` hook and API client

### Changed
- Admin dashboard channel usage section now uses server-side aggregation instead of client-side aggregation from 200 audit log entries

## [0.13.0] - 2026-05-02

### Added
- Admin dashboard — system status, metrics, top models, channel usage breakdown, recent requests
- Provider models management — add/edit/remove models per provider via modals (pricing policy, upstream name)
- `PUT /api/v1/admin/providers/{id}/models` endpoint for updating provider model assignments
- Pricing policy column in provider_models table (migration: `20260505000000_provider_models_pricing`)
- Channel usage section on admin dashboard showing per-channel request distribution, latency, and error rate
- Dashboard nav item in admin sidebar

### Changed
- Provider cards now show models as clickable badges (click to edit) with pricing indicator dots
- ChannelDetail crash on non-array API responses fixed with `Array.isArray()` guard

## [0.12.0] - 2026-05-02

### Added
- Provider models catalog — new `provider_models` table records which models each provider supports
- Model dropdown in "Add Channel Model" modal now filters by channel's provider
- Upstream model name auto-filled from provider catalog when selecting a model
- `GET /api/v1/admin/providers/{id}/models` endpoint for provider's model catalog
- Seed data populates provider_models for all built-in providers

## [0.11.0] - 2026-05-01

### Added
- Weighted round-robin channel routing — channels at the same priority tier distribute traffic proportionally by weight (default 100)
- Weight configuration on channel create/edit forms
- Weight display on channel list and detail pages

## [0.10.7] - 2026-05-01

### Added
- Real-time availability indicator on channel list page — shows "Available" (green) or "Outside Hours" (gray) based on current UTC time against channel schedule

## [0.10.6] - 2026-05-01

### Fixed
- Channel list API now includes `available_hours` in response (was missing from `ChannelWithModels`, causing list to always show "24/7")
- Improved model badge and day abbreviation font sizes on channel list page for readability

## [0.10.5] - 2026-05-01

### Fixed
- Channel list page now shows detailed available hours (time ranges and days) instead of schedule count

## [0.10.4] - 2026-05-01

### Added
- Show available hours indicator on channel list page (schedule count or 24/7)

## [0.10.3] - 2026-05-01

### Added
- Show channel name on audit log list page and detail drawer (LEFT JOIN channels instead of showing truncated UUIDs)

### Fixed
- Model card pricing now correctly converts from subunits to USD
- Channel detail page refreshes after editing available hours

## [0.10.2] - 2026-05-01

### Fixed
- Home page curl example now correctly displays full URL with configured server host

## [0.10.1] - 2026-05-01

### Fixed
- Validate database driver at startup — unknown values now fail with clear error instead of silently falling back to SQLite
- About tab no longer hardcodes "SQLite", reads actual driver from config

## [0.10.0] - 2026-05-01

### Added
- Channel Available Hours — restrict channels to specific days and time ranges (UTC), with routing automatically filtering out channels outside their scheduled hours
- Frontend Available Hours card on Channel Detail page with day toggles and time inputs

### Fixed
- Clear schedule now works correctly (send `[]` to clear — `Option<Option<Vec<TimeSlot>>>` bug fixed to single Option)

## [0.9.7] - 2026-04-30

### Fixed
- Seed pricing policies deserialization — camelCase JSON keys now match `#[serde(rename_all = "camelCase")]`
- Pricing policy seeding decoupled from model seeding (independent table check)
- Reduced seed pricing policies to glm-5.1, minimax-m2.7, minimax-m2.7-highspeed only

## [0.9.6] - 2026-04-30

### Fixed
- Seed models loaded independently from providers (N:N model-provider architecture)

## [0.9.5] - 2026-04-30

### Fixed
- Version passed explicitly via build arg in Dockerfile
- Settings test updated for version-agnostic matching

## [0.9.4] - 2026-04-30

### Added
- Monetary integer subunits — all money values stored as integer microdollars (1 USD = 1,000,000 units) to eliminate floating-point errors
- `money` module with `usd_to_units` / `units_to_usd` / `bps_to_ratio` / `ratio_to_bps` conversion helpers
- SQLite and PostgreSQL migrations to convert existing monetary columns to INTEGER/BIGINT
- API boundary conversion: management handlers accept/return USD floats, storage layer uses i64 integers
- Billing, settlement, and workers updated to integer arithmetic throughout

### Fixed
- PostgreSQL type compatibility (BIGINT for SUM aggregates, TIMESTAMPTZ for timestamps)
- PostgreSQL migrations made idempotent for existing databases
- Context-tiered billing support in frontend pricing display

## [0.9.3] - 2026-04-29

### Fixed
- Quote reserved keyword `window` in PostgreSQL rate_limit_counters query

## [0.9.2] - 2026-04-29

### Fixed
- Correct PostgreSQL 18 data path in production docker-compose
- Use list form for `depends_on` in docker-compose

## [0.9.1] - 2026-04-29

### Fixed
- Add version field to all docker-compose files

## [0.9.0] - 2026-04-29

### Added
- Cache creation pricing support
- Key prefix display in API key list
- Sidebar improvements and font switch (Outfit + JetBrains Mono)

## [0.8.3] - 2026-04-28

### Changed
- Redesigned Audit Logs page with consistent design patterns: animated header, section cards, improved filter bar with active count badge, proper table styling, and detail drawer with structured sections

## [0.8.2] - 2026-04-28

### Added
- Admin Settings page rebuilt with tabbed layout (General, Security & Audit, System Info, About)
- System Info tab showing infrastructure configuration reference
- About tab with version and GitHub link

### Fixed
- Version display showing "vv0.8.0" instead of "v0.8.0" in sidebar, header footer, home page, and settings

## [0.8.0] - 2026-04-27

### Added
- `user_id` column on `usage_records` and `audit_logs` tables (denormalized from `api_keys.created_by`)
- Account balance card on Dashboard with low-balance warning
- User-scoped audit log queries (non-admin users can now view their own logs)
- `query_usage_cost_by_user` storage method for efficient settlement

### Changed
- Usage API (`/api/v1/usage`, `/api/v1/usage/summary`) now properly scopes data to the current user for non-admin requests (was returning all users' data)
- Settlement worker replaced N+1 key-lookup loop with single `GROUP BY user_id` query
- Usage page key filter dropdown now only shows keys belonging to the current user

## [0.7.0] - 2026-04-27

### Added
- Runtime database driver selection (PostgreSQL or SQLite via `config.toml`)
- Docker image build and push on CI release (GHCR with semver tags)
- Production docker-compose with PostgreSQL 18
- `useReducedMotion` hook — respects `prefers-reduced-motion` system preference
- Global CSS reduced-motion media query
- GLM seed data with Anthropic and OpenAI endpoint URLs
- Keyboard navigation and focus-visible rings on model cards

### Changed
- **Home page redesign**: fixed nav, value-driven hero, 3-step flow, terminal-style quick start, CTA section
- **Dashboard redesign**: animated metric cards, server-side usage summary (replaces client-side aggregation), loading skeletons, status pills
- **Models page**: active card redesign — clean neutral styling, emerald status badge, clickable cards with keyboard support, form label accessibility
- Body text across Home/Dashboard/Models bumped to 16px for readability

### Fixed
- PostgreSQL storage module synced with current data model (removed stale fields)
- Removed background glow animations that bypassed reduced-motion

## [0.6.1] - 2026-04-26

### Fixed
- Update EndpointsEditor test to expect `default` as first protocol

## [0.6.0] - 2026-04-26

### Added
- Provider proxy URL — route upstream requests through configurable HTTP proxy (`proxy_url` field on providers)
- Audit log detail endpoint (`GET /api/v1/admin/logs/{id}`) for fetching full request/response bodies on demand
- Git flow workflow documented in CLAUDE.md

### Changed
- Audit log list API now returns `AuditLogSummary` (excludes `request_body` and `response_body`) for performance
- EndpointsEditor protocol options: `default`, `openai`, `anthropic` (removed azure, google, custom)
- Token storage normalized across protocols (see CLAUDE.md "Token Storage Convention")

### Fixed
- Return `ProviderWithEndpoints` from create/update provider handlers (endpoints were blank after save)
- Add `default` endpoint key to provider forms
- Proxy `/v1` routes in Vite dev server
- SQLite compatibility and font scaling improvements

## [0.5.1] - 2026-04-24

### Fixed
- Use `type` column name in transactions SQL for SQLite compat

## [0.5.0] - 2026-04-22

### Added
- Initial release with OpenAI and Anthropic compatible endpoints
- API key management, provider/channel configuration, billing, rate limiting
- React frontend with dashboard, logs, usage tracking
