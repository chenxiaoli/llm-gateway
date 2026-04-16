# System Settings & Audit Log Config

**Date:** 2026-04-14

## Overview

Add server host setting and configurable audit logging (request/response body toggle) to the system settings.

## Requirements

1. **server_host** - Store server address in settings, used in Quick Start display
2. **audit_log_request** - Toggle to enable/disable request body logging
3. **audit_log_response** - Toggle to enable/disable response body logging

## Data Model

### Settings Table (existing)

Add new key-value pairs:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| server_host | string | "" | Server address for Quick Start |
| audit_log_request | boolean | true | Whether to log request body |
| audit_log_response | boolean | true | Whether to log response body |

## API

### GET /api/v1/settings

```json
{
  "allow_registration": true,
  "server_host": "https://api.example.com",
  "audit_log_request": true,
  "audit_log_response": true
}
```

### PATCH /api/v1/settings

```json
{
  "allow_registration": true,
  "server_host": "https://api.example.com",
  "audit_log_request": false,
  "audit_log_response": true
}
```

## Audit Logger Behavior

When logging requests:

- If `audit_log_request = false`: set request_body = "{}"
- If `audit_log_response = false`: set response_body = "{}"

## Frontend

### Settings Page
- Add audit logging toggles (request/response body)

### Home/Quick Start
- Read server_host from settings API
- Display in cURL examples instead of localhost

## Implementation Notes

- Settings stored in key-value table, no migration needed
- Audit logger needs to read settings per-request or cache them
- Home page needs to fetch settings on load