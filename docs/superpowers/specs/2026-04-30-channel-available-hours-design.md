# Channel Available Hours

## Problem

Channels are always available for routing. For cost optimization, operators want to restrict certain channels to specific time windows — e.g., use a cheaper upstream only during off-peak hours.

## Solution

Add an `available_hours` field to channels. When set, the routing layer only considers the channel during the specified day-of-week + time ranges. When `NULL`, the channel is always available (backward compatible).

## Data Model

### Column

`available_hours TEXT` added to `channels` table. Nullable.

- `NULL` — channel always available (default)
- `[]` — channel never available
- Array of slot objects — channel available when any slot matches

### Slot format

```json
[
  {"days": ["mon","tue","wed","thu","fri"], "start": "09:00", "end": "18:00"},
  {"days": ["sat"], "start": "10:00", "end": "14:00"}
]
```

- `days` — array of 3-letter lowercase day abbreviations: `mon`, `tue`, `wed`, `thu`, `fri`, `sat`, `sun`
- `start`, `end` — `HH:MM` in UTC, 24h format. `end` of `"24:00"` represents midnight.
- Multiple slots use OR logic: channel available if ANY slot matches current time.

### Rust types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSlot {
    pub days: Vec<String>,
    pub start: String,
    pub end: String,
}
```

`Channel` struct gains field: `pub available_hours: Option<Vec<TimeSlot>>`.

## Migration

SQLite and PostgreSQL:

```sql
ALTER TABLE channels ADD COLUMN available_hours TEXT;
```

No data migration. Existing rows get `NULL` (always available).

All channel `SELECT` and `INSERT` queries updated to include `available_hours`. On read, `NULL` maps to `None`. On write, `None` stores `NULL`, `Some(slots)` stores JSON string.

## Routing

`ResolvedChannel` in `InMemoryChannelRegistry` gains `available_hours: Option<Vec<TimeSlot>>`.

During `resolve_by_model()`, after building the candidate channel list, filter out channels outside their time slots:

```
is_available_now(slots):
    if slots is None → true
    get current UTC day-of-week and time
    for each slot in slots:
        if today in slot.days AND current_time in [start, end) → true
    return false
```

Channels with `available_hours: None` always pass. The check runs at resolve time using cached slot data — no DB query.

## API

No new endpoints. Existing channel CRUD handles it.

### Request fields

`CreateChannelRequest` and `UpdateChannelRequest` gain:

```json
"availableHours": [{"days": ["mon","tue","wed","thu","fri"], "start": "09:00", "end": "18:00"}]
```

- Omit or `null` — always available
- Array of slots — scheduled availability

### Response

Channel responses include `availableHours` (camelCase). `null` when not set.

### Example

```json
PATCH /api/v1/admin/channels/{id}
{
  "availableHours": [
    {"days": ["mon","tue","wed","thu","fri"], "start": "09:00", "end": "18:00"}
  ]
}
```

Set to `null` to clear schedule.

## Frontend

New **"Available Hours"** card on `ChannelDetail.tsx` (same pattern as existing "Channel Models" card).

- `availableHours` is `null` → show "Always available" badge + "Add Schedule" button
- Slots exist → list as rows, e.g. "Mon–Fri 09:00–18:00 UTC", with edit/delete actions
- Add/Edit modal: multi-select for days, two time inputs for start/end (UTC labeled)
- "Clear Schedule" button to set back to `null`

No changes to channel list page or "Add Channel" drawer.
