---
name: Browser timezone for available hours
date: 2026-05-24
---

## Summary

Display and edit channel available hours in the user's browser timezone instead of raw UTC. Frontend-only change — backend continues to store and compare times in UTC.

## Current behavior

- `TimeSlot` stores `start`/`end` as HH:MM strings in UTC (e.g., `start: "01:00"` = 1am UTC)
- Frontend `<input type="time">` shows raw UTC values — confusing for users in non-UTC timezones
- `isAvailableNow` in `Channels.tsx` hardcodes UTC comparison
- Backend `is_available_now` uses `chrono::Utc::now()` — unchanged

## New behavior

### Display

- Show timezone label next to "Available Hours" header, e.g. `(UTC+8, Asia/Shanghai)`
- Convert UTC HH:MM to local HH:MM when displaying time slots
- Adjust day when conversion crosses midnight (UTC Mon 23:00 → Shanghai Tue 07:00)

### Edit

- `<input type="time">` shows local times
- On save, convert local HH:MM back to UTC before sending to API
- Adjust day in reverse when local time crosses midnight backwards

### Availability check

- `isAvailableNow` in `Channels.tsx` uses browser timezone instead of hardcoded UTC

## Implementation

### New utility: `web/src/lib/timezone.ts`

- `utcToLocalTime(utcHHMM, tz) → localHHMM` — e.g., "01:00" UTC → "09:00" Asia/Shanghai
- `localToUtcTime(localHHMM, tz) → utcHHMM` — reverse
- `utcDayToLocalDay(utcDay, utcHHMM, tz) → localDay` — adjust day on midnight cross
- `localDayToUtcDay(localDay, localHHMM, tz) → utcDay` — reverse
- `isAvailableNow(slots, tz) → boolean` — availability check in given timezone

All functions use `Intl.DateTimeFormat` with `timeZone` option — no external libraries needed.

### Changes to `ChannelDetail.tsx`

- Detect browser timezone via `Intl.DateTimeFormat().resolvedOptions().timeZone`
- Show timezone label next to Available Hours card header
- Convert slot times UTC→local when rendering the display card
- Convert slot times local→UTC when saving from the edit modal
- Convert default new slot times to local (currently hardcoded "09:00-17:00" UTC)

### Changes to `Channels.tsx`

- Replace hardcoded UTC logic in `isAvailableNow` with timezone-aware version
- Display local times in channel row availability section

### No backend changes

Backend `TimeSlot` struct, `is_available_now`, database storage, and API contracts remain unchanged. All times are still UTC on the wire and in storage.
