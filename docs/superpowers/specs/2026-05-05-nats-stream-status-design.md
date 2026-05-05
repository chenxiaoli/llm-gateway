# NATS Stream Status for Admins — Design Spec

**Date:** 2026-05-05
**Status:** Approved

## Goal

Show admins the real-time status of the two NATS JetStream streams (`LLM_GATEWAY_USAGE`, `LLM_GATEWAY_AUDIT`) in the Settings System tab.

## Design

### Backend: `GET /api/v1/admin/nats/status`

Admin-only endpoint. Queries both streams via `NatsPublisher::js_context()`:

```
For each stream (LLM_GATEWAY_USAGE, LLM_GATEWAY_AUDIT):
  stream = js.get_stream(name)
  info = stream.info()
  return { name, messages, bytes, consumer_count, first_sequence, last_sequence, max_age, max_messages }
```

Response shape:

```json
{
  "streams": [
    {
      "name": "LLM_GATEWAY_USAGE",
      "messages": 12345,
      "bytes": 6789012,
      "consumer_count": 1,
      "first_sequence": 1,
      "last_sequence": 12345,
      "max_messages": 1000000,
      "max_age_secs": 604800
    },
    {
      "name": "LLM_GATEWAY_AUDIT",
      "messages": 54321,
      "bytes": 9876543,
      "consumer_count": 1,
      "first_sequence": 1,
      "last_sequence": 54321,
      "max_messages": 5000000,
      "max_age_secs": 2592000
    }
  ]
}
```

If NATS is unreachable or stream not found, returns 503 with error message.

### Frontend: Settings System Tab

Add a "NATS Streams" section below the existing system info grid. Each stream gets a card showing:
- Stream name
- Messages (formatted with locale number separator)
- Size (human-readable bytes: KB, MB, GB)
- Consumers
- Retention (max_age as "7 days" / "30 days")
- Max messages (formatted)

New files:
- `web/src/api/settings.ts` — `getNatsStatus()` function
- `web/src/hooks/useSettings.ts` — `useNatsStatus()` hook
- `web/src/types/index.ts` — `NatsStreamInfo` and `NatsStatusResponse` types

Modified files:
- `web/src/pages/Settings.tsx` — add NATS Streams section to System tab
- `web/src/i18n/en.json`, `web/src/i18n/zh.json` — translation keys

## Not Changed

- NATS publisher (only uses existing `js_context()` method)
- Database (no new tables or columns)
- Other pages or endpoints
