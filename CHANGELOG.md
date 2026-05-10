# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [1.3.14] - 2026-05-10

### Fixed
- Usage page (`/console/usage`) now always filters by current logged-in user (admins previously saw all users' data)
- Key filter on usage page now works correctly alongside user filter (was ignored due to `else if` logic)
- Added index on `usage_records(user_id, created_at)` for query performance

## [1.3.13] - 2026-05-08

### Added
- Multi-currency display support (USD/CNY) as a system-level setting
- Currency selector in Settings > General for admins
- `currency` field in `/auth/config` and settings API responses
- Frontend currency store (Zustand) with symbol-aware formatting across all pages

### Changed
- Removed per-account `currency` field in favor of global system currency
- All monetary amounts now display using the configured currency symbol

## [1.3.12] - 2026-05-07

### Added
- Retry next available channel when upstream returns 429 rate limit

## [1.3.11] - 2026-05-05

### Fixed
- NATS stream pending messages now shows stream message count when no consumers exist (was incorrectly 0)

## [1.3.10] - 2026-05-05

### Fixed
- Channel test now uses the same URL construction logic as proxy, fixing 404 on providers with non-standard version segments (e.g. /v4)

## [1.3.9] - 2026-05-05

### Changed
- Refactor integration tests to use `sqlx::test` pattern with per-test isolated databases

## [1.3.4] - 2026-05-05

### Added
- NATS stream status pills on admin dashboard showing USAGE and AUDIT pending message counts

## [1.3.3] - 2026-05-05

### Changed
- Remove estimated pending bytes from NATS stream status, keep only exact pending message count

## [1.3.2] - 2026-05-05

### Added
- Show pending message size (estimated bytes) alongside pending count in NATS stream status

## [1.3.1] - 2026-05-05

### Added
- Show unconsumed (pending) message count in NATS stream status cards

## [1.3.0] - 2026-05-04

### Added
- Channel group field for logical grouping of channels (backend migration, API, frontend forms/display)
- Per-endpoint test buttons on channel detail page with Anthropic protocol support
- NATS JetStream status endpoint (`GET /api/v1/admin/nats/status`) showing real-time stream stats
- NATS stream status cards in Settings System tab (messages, size, consumers, retention)

## [1.2.2] - 2026-05-05

### Fixed
- OpenAI-compatible providers with versioned base URLs (e.g. `/v4`, `/v1`) no longer produce doubled paths
- Anthropic-compatible providers on non-standard hosts correctly append `/v1/messages`

## [1.2.1] - 2026-05-05

### Fixed
- Show API key ID on audit log table rows and detail drawer
- Fix Register test placeholder capitalization after i18n migration

## [1.2.0] - 2026-05-04

### Added
- Per-request balance deduction replaces batch settlement — usage worker deducts immediately after recording each request
- Shared `request_id` across `usage_records`, `audit_logs`, and `transactions` for 1:1:1 traceability
- `GET /api/v1/admin/requests/:request_id` endpoint to look up usage record, audit log, and transaction by request_id
- Frontend transaction drill-down — click a debit transaction to see usage details (model, tokens, cost, latency)
- Gateway auto-injects `stream_options: { include_usage: true }` for OpenAI streaming requests missing the field

### Changed
- Batch settlement worker (`crates/api/src/settlement.rs`) removed — no more 60s interval aggregation

### Fixed
- OpenAI-compatible streaming requests without `stream_options` in the body no longer silently skip billing

## [1.1.0] - 2026-05-03

### Added
- Frontend internationalization (i18n) with English and Simplified Chinese support
- Language toggle (Globe icon) in sidebar header — instant switch, persists to localStorage
- Browser language auto-detection (falls back to English)
- 850 translation keys across 25 sections covering all pages, components, hooks, and toast messages
- `react-i18next` + `i18next` with bundled JSON translation files

### Changed
- **SQLite removed** — PostgreSQL is now the only database driver
- NATS JetStream is required (no mpsc fallback) — gateway fails to start without `[nats]` config
- NATS streams renamed from `GATEWAY_*` to `LLM_GATEWAY_*` (`LLM_GATEWAY_USAGE`, `LLM_GATEWAY_AUDIT`)
- Audit and usage workers extracted into independent binaries (`llm-gateway-usage-worker`, `llm-gateway-audit-worker`)
- Docker builds now produce 3 binaries with `entrypoint` override for worker services
- Production docker-compose includes NATS service with JetStream
- Integration tests use PostgreSQL service container instead of SQLite

### Fixed
- ResolveJsonModule added to tsconfig for JSON imports
- ConfirmDialog i18n defaults resolve at render time (not module load)
- Test render helper imports i18n for component test compatibility

## [1.0.0] - 2026-05-03

### Added
- NATS JetStream integration for decoupled audit and usage event processing
- `nats-publisher` crate with `UsageEvent` and `AuditEvent` types, stream management, and push consumers
- Two separate JetStream streams: `GATEWAY_USAGE` (7d retention) and `GATEWAY_AUDIT` (30d retention)
- In-process NATS consumers write to DB; external consumers can attach independently
- Backward compatible — when `[nats]` config is absent, falls back to in-process mpsc channel
- Console Models page — read-only model listing for all authenticated users with search, pricing display (per_token and context_tiered)
- `GET /api/v1/user/models` endpoint for console model data
- Channel Test button on admin Channels page — tests upstream connectivity with inline status feedback
- `POST /api/v1/admin/channels/{id}/test` endpoint for channel testing

### Fixed
- Normalize request model name to database canonical form for consistent usage/audit records
- Console Models page only shows live (available) models
- Price conversion (subunits → USD) for all billing types on console model cards
- Channel test endpoint upstream URL missing /v1 prefix
- Removed /v1 from seed provider endpoints to prevent URL path doubling

## [0.14.1] - 2026-05-03

### Fixed
- Normalize request model name to database canonical form for consistent usage/audit records regardless of request casing
- Console Models page now only shows live (available) models
- Add context_tiered pricing display with tier-by-tier breakdown on model cards
- Fix price conversion (subunits → USD) for all billing types on console model cards
- Channel test endpoint upstream URL was missing /v1 prefix
- Removed /v1 from seed provider endpoints to prevent URL path doubling

## [0.14.0] - 2026-05-03

### Added
- Console Models page — read-only model listing visible to all authenticated users, showing name, type, pricing, and availability status
- `GET /api/v1/user/models` endpoint for console model data (admin-only details excluded)
- Channel Test button on admin Channels page — sends a minimal chat completion request through the channel's upstream provider and reports success/failure with latency
- `POST /api/v1/admin/channels/{id}/test` endpoint for channel connectivity testing
- `ChannelTestResult` type (backend + frontend)

### Fixed
- Channel test endpoint upstream URL was missing `/v1` prefix, causing 404 errors on OpenAI-compatible providers
- Removed `/v1` from seed provider endpoints (OpenAI, MiniMax, Alibaba) to prevent URL path doubling
- Console Models page now handles non-array API responses gracefully

## [0.13.5] - 2026-05-02

### Fixed
- `apiClient` (used by keys, model-fallbacks, usage, accounts) was not attaching Bearer token to requests — all non-`/admin/*` authenticated endpoints returned 401

## [0.13.4] - 2026-05-02

### Fixed
- Channel usage summary query now groups by both `channel_id` and channel name (PostgreSQL GROUP BY requirement)

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
## [1.3.8] - 2026-05-05

### Fixed
- Fix compilation errors in integration tests (stale AppState fields)

## [1.3.7] - 2026-05-05

### Changed
- Language switch now shows current language code (EN/中) instead of bare icon

## [1.3.6] - 2026-05-05

### Fixed
- Admin pages no longer redirect to console dashboard on page refresh

## [1.3.5] - 2026-05-05

### Fixed
- Move NATS status pills from user dashboard to admin dashboard
