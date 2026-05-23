---
name: Request ID visibility and filtering
date: 2026-05-23
---

## Summary

Show request IDs on the `/console/usage` page and `/admin/logs` page, with copy buttons and a request ID filter on the logs page.

## Feature 1: Request ID on /console/usage

- Add `request_id` column to the usage table as the first column (before Time).
- Display first 8 characters in monospace (`font-mono text-xs`).
- Copy icon button next to the truncated ID using the existing pattern from `Providers.tsx`: `navigator.clipboard.writeText()` + `sonner` toast, icon toggles from `Copy` to `Check` for 1.5 seconds.
- Add `request_id` to the `UsageRecord` TypeScript type.
- Verify the backend `GET /api/v1/usage` endpoint returns `request_id` in each record. If not, add it to the SQL query and response struct.

## Feature 2: Request ID filter + column on /admin/logs

- Add `request_id` text input to the existing filter card, alongside date and dropdown filters.
- Exact match only — sends `request_id` as a query param to the backend (`WHERE request_id = $1`).
- Add `request_id` column to the logs table, showing first 8 chars in monospace with the same copy button as Feature 1.
- Fix type gap: add `request_id` to the `AuditLogSummary` TypeScript type (backend already sends it).
- Add `request_id` to `LogFilter` type and the `queryLogs` API call params.
- Add `request_id` filter param to the backend `GET /api/v1/admin/logs` endpoint.

## Cross-cutting

- Extract a reusable `CopyButton` component to avoid duplicating the copy-with-feedback pattern across both pages.
- Add i18n translation keys for the toast message, column header, and filter label (English + Chinese).
