# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
