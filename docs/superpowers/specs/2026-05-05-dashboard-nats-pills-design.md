# NATS Stream Status Pills on Admin Dashboard

## Goal

Show NATS stream health at a glance on the admin dashboard by adding two status pills for the USAGE and AUDIT streams.

## Design

Add two `StatusPill` components to the existing status pills row on the admin dashboard, one per NATS JetStream stream:

- **USAGE stream pill**: Shows "USAGE" label with pending message count. Green when pending = 0, amber when > 0.
- **AUDIT stream pill**: Shows "AUDIT" label with pending message count. Green when pending = 0, amber when > 0.

Uses the existing `useNatsStatus()` hook and `GET /api/v1/admin/nats/status` endpoint. No new backend work.

## Changes

- `web/src/pages/Dashboard.tsx`: Import `useNatsStatus`, render 2 additional `StatusPill` components in the existing flex-wrap row.
- `web/src/i18n/en.json`: Add `dashboard.natsUsage` and `dashboard.natsAudit` keys.
- `web/src/i18n/zh.json`: Add corresponding Chinese translations.

## No changes

- No new API endpoints or hooks.
- No backend changes.
