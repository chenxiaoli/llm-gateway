# Available Hours Timezone Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Display and edit channel available hours in the user's browser timezone instead of raw UTC.

**Architecture:** Create a pure utility module `web/src/lib/timezone.ts` using the built-in `Intl.DateTimeFormat` API for UTC↔local time conversions. ChannelDetail.tsx converts times for display and edit; Channels.tsx uses timezone-aware availability check. Backend is untouched.

**Tech Stack:** TypeScript, Intl.DateTimeFormat (no external libraries), React 18

---

### Task 1: Create timezone utility module

**Files:**
- Create: `web/src/lib/timezone.ts`
- Test: `web/src/lib/timezone.test.ts`

- [ ] **Step 1: Write the failing test**

```typescript
// web/src/lib/timezone.test.ts
import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  utcToLocalTime,
  localToUtcTime,
  utcDayToLocalDay,
  localDayToUtcDay,
  isAvailableNow,
  getTimezoneLabel,
  DAY_ORDER,
} from './timezone';

// Mock Intl.DateTimeFormat to force a known timezone
const originalDateTimeFormat = Intl.DateTimeFormat;

beforeEach(() => {
  vi.restoreAllMocks();
});

describe('utcToLocalTime / localToUtcTime', () => {
  it('converts UTC to local and back', () => {
    // These tests run in the node timezone; use round-trip to verify symmetry
    const utc = '01:00';
    const local = utcToLocalTime(utc, 'Asia/Shanghai');
    // UTC 01:00 → Shanghai 09:00
    expect(local).toBe('09:00');
    expect(localToUtcTime(local, 'Asia/Shanghai')).toBe('01:00');
  });

  it('handles midnight cross', () => {
    // UTC 23:00 → Shanghai next day 07:00
    expect(utcToLocalTime('23:00', 'Asia/Shanghai')).toBe('07:00');
    // Local 07:00 Shanghai → UTC 23:00 previous day
    expect(localToUtcTime('07:00', 'Asia/Shanghai')).toBe('23:00');
  });

  it('handles negative offset', () => {
    // UTC 01:00 → New York previous day 21:00 (UTC-4 in EDT)
    const local = utcToLocalTime('01:00', 'America/New_York');
    // Exact result depends on DST, just verify round-trip
    expect(localToUtcTime(local, 'America/New_York')).toBe('01:00');
  });

  it('pads single-digit hours', () => {
    const result = utcToLocalTime('00:00', 'Asia/Shanghai');
    expect(result).toMatch(/^\d{2}:\d{2}$/);
  });
});

describe('utcDayToLocalDay / localDayToUtcDay', () => {
  it('shifts day forward when crossing midnight', () => {
    // UTC Monday 23:00 → Shanghai Tuesday 07:00
    expect(utcDayToLocalDay('mon', '23:00', 'Asia/Shanghai')).toBe('tue');
  });

  it('shifts day backward for negative offset', () => {
    // UTC Tuesday 01:00 → New York Monday 21:00 (EDT)
    const result = utcDayToLocalDay('tue', '01:00', 'America/New_York');
    expect(result).toBe('mon');
  });

  it('keeps same day when no midnight cross', () => {
    // UTC 09:00 → Shanghai 17:00, same day
    expect(utcDayToLocalDay('wed', '09:00', 'Asia/Shanghai')).toBe('wed');
  });

  it('round-trips correctly', () => {
    const localDay = utcDayToLocalDay('thu', '23:00', 'Asia/Shanghai');
    const localTime = utcToLocalTime('23:00', 'Asia/Shanghai');
    expect(localDayToUtcDay(localDay, localTime, 'Asia/Shanghai')).toBe('thu');
  });

  it('wraps mon→sun backward', () => {
    expect(utcDayToLocalDay('mon', '01:00', 'America/New_York')).toBe('sun');
  });

  it('wraps sun→mon forward', () => {
    expect(utcDayToLocalDay('sun', '23:00', 'Asia/Shanghai')).toBe('mon');
  });
});

describe('isAvailableNow', () => {
  it('returns true when no slots', () => {
    expect(isAvailableNow([], 'UTC')).toBe(true);
    expect(isAvailableNow(null, 'UTC')).toBe(true);
  });

  it('returns true when slot matches current time', () => {
    // We'll test the logic by mocking Date
    const fixedDate = new Date('2026-01-05T10:30:00Z'); // Monday 10:30 UTC
    vi.useFakeTimers();
    vi.setSystemTime(fixedDate);

    // UTC+8 → local time is Mon 18:30
    // Slot: mon 18:00–19:00 in Shanghai → should be available
    const slots = [{ days: ['mon'], start: '02:00', end: '03:00' }]; // UTC 02:00–03:00 = Shanghai 10:00–11:00
    expect(isAvailableNow(slots, 'UTC')).toBe(false); // 10:30 not in 02:00–03:00 UTC

    const slots2 = [{ days: ['mon'], start: '10:00', end: '11:00' }]; // UTC 10:00–11:00
    expect(isAvailableNow(slots2, 'UTC')).toBe(true); // 10:30 in range

    vi.useRealTimers();
  });
});

describe('getTimezoneLabel', () => {
  it('returns formatted label', () => {
    const label = getTimezoneLabel('Asia/Shanghai');
    expect(label).toMatch(/UTC\+8/);
    expect(label).toContain('Asia/Shanghai');
  });

  it('handles negative offset', () => {
    const label = getTimezoneLabel('America/New_York');
    expect(label).toMatch(/UTC/);
  });

  it('handles UTC', () => {
    const label = getTimezoneLabel('UTC');
    expect(label).toContain('UTC');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /workspace/llm-gateway/web && npm test -- src/lib/timezone.test.ts 2>&1`
Expected: FAIL — module `./timezone` not found

- [ ] **Step 3: Write the implementation**

```typescript
// web/src/lib/timezone.ts
import type { TimeSlot } from '../types';

const DAYS = ['sun', 'mon', 'tue', 'wed', 'thu', 'fri', 'sat'] as const;
export const DAY_ORDER = ['mon', 'tue', 'wed', 'thu', 'fri', 'sat', 'sun'] as const;

/**
 * Get UTC offset in minutes for a timezone at a reference date.
 * Positive = east of UTC (e.g., Asia/Shanghai = +480).
 */
function getUtcOffsetMinutes(tz: string, refDate: Date = new Date()): number {
  const utcStr = refDate.toLocaleString('en-US', { timeZone: 'UTC' });
  const tzStr = refDate.toLocaleString('en-US', { timeZone: tz });
  const utcDate = new Date(utcStr);
  const tzDate = new Date(tzStr);
  return (tzDate.getTime() - utcDate.getTime()) / 60000;
}

/** Parse "HH:MM" to minutes since midnight */
function hhmmToMinutes(hhmm: string): number {
  const [h, m] = hhmm.split(':').map(Number);
  return (h || 0) * 60 + (m || 0);
}

/** Minutes since midnight → "HH:MM" */
function minutesToHHMM(minutes: number): string {
  const m = ((minutes % 1440) + 1440) % 1440;
  const h = Math.floor(m / 60);
  const min = m % 60;
  return `${String(h).padStart(2, '0')}:${String(min).padStart(2, '0')}`;
}

/** Convert UTC HH:MM to local HH:MM in given timezone */
export function utcToLocalTime(utcHHMM: string, tz: string): string {
  const offset = getUtcOffsetMinutes(tz);
  return minutesToHHMM(hhmmToMinutes(utcHHMM) + offset);
}

/** Convert local HH:MM to UTC HH:MM in given timezone */
export function localToUtcTime(localHHMM: string, tz: string): string {
  const offset = getUtcOffsetMinutes(tz);
  return minutesToHHMM(hhmmToMinutes(localHHMM) - offset);
}

/** Get the number of days the offset shifts (−1, 0, or +1) */
function getDayShift(utcHHMM: string, tz: string): number {
  const offset = getUtcOffsetMinutes(tz);
  const localMinutes = hhmmToMinutes(utcHHMM) + offset;
  if (localMinutes >= 1440) return 1;
  if (localMinutes < 0) return -1;
  return 0;
}

/** Convert UTC day + time to local day, handling midnight cross */
export function utcDayToLocalDay(utcDay: string, utcHHMM: string, tz: string): string {
  const shift = getDayShift(utcHHMM, tz);
  if (shift === 0) return utcDay;
  const idx = DAYS.indexOf(utcDay as typeof DAYS[number]);
  return DAYS[(idx + shift + 7) % 7];
}

/** Convert local day + time to UTC day, handling midnight cross in reverse */
export function localDayToUtcDay(localDay: string, localHHMM: string, tz: string): string {
  const offset = getUtcOffsetMinutes(tz);
  const utcMinutes = hhmmToMinutes(localHHMM) - offset;
  let shift = 0;
  if (utcMinutes >= 1440) shift = 1;
  else if (utcMinutes < 0) shift = -1;
  if (shift === 0) return localDay;
  const idx = DAYS.indexOf(localDay as typeof DAYS[number]);
  return DAYS[(idx + shift + 7) % 7];
}

/** Timezone-aware availability check */
export function isAvailableNow(slots: TimeSlot[] | null | undefined, tz: string): boolean {
  if (!slots || slots.length === 0) return true;
  const now = new Date();
  const day = now.toLocaleDateString('en-US', { weekday: 'short', timeZone: tz }).toLowerCase();
  const nowLocalMinutes = (() => {
    const parts = new Intl.DateTimeFormat('en-US', {
      hour: 'numeric',
      minute: 'numeric',
      hour12: false,
      timeZone: tz,
    }).formatToParts(now);
    const h = parseInt(parts.find(p => p.type === 'hour')!.value, 10);
    const m = parseInt(parts.find(p => p.type === 'minute')!.value, 10);
    return h * 60 + m;
  })();

  return slots.some(slot => {
    if (!slot.days.includes(day)) return false;
    const start = hhmmToMinutes(slot.start);
    const end = hhmmToMinutes(slot.end);
    return nowLocalMinutes >= start && nowLocalMinutes < end;
  });
}

/** Get a human-readable timezone label, e.g., "UTC+8 (Asia/Shanghai)" */
export function getTimezoneLabel(tz: string): string {
  const offset = getUtcOffsetMinutes(tz);
  const sign = offset >= 0 ? '+' : '-';
  const absOffset = Math.abs(offset);
  const hours = Math.floor(absOffset / 60);
  const minutes = absOffset % 60;
  const offsetStr = minutes > 0 ? `${sign}${hours}:${String(minutes).padStart(2, '0')}` : `${sign}${hours}`;
  return `UTC${offsetStr} (${tz})`;
}

/** Detect browser timezone */
export function getBrowserTimezone(): string {
  return Intl.DateTimeFormat().resolvedOptions().timeZone;
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /workspace/llm-gateway/web && npm test -- src/lib/timezone.test.ts 2>&1`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
cd /workspace/llm-gateway
git add web/src/lib/timezone.ts web/src/lib/timezone.test.ts
git commit -m "feat(web): add timezone utility for available hours conversion"
```

---

### Task 2: Update ChannelDetail.tsx — display and edit in local timezone

**Files:**
- Modify: `web/src/pages/ChannelDetail.tsx:384-411` (display card)
- Modify: `web/src/pages/ChannelDetail.tsx:847-914` (edit modal)

This task converts the Available Hours display card to show local times with a timezone label, and converts the edit modal to work in local time (converting back to UTC on save).

- [ ] **Step 1: Add timezone imports and helpers to ChannelDetail.tsx**

At the top of `ChannelDetail.tsx`, add these imports after the existing ones:

```typescript
import { utcToLocalTime, localToUtcTime, utcDayToLocalDay, localDayToUtcDay, getBrowserTimezone, getTimezoneLabel } from '../lib/timezone';
```

Inside the `ChannelDetail` component function body (before the return), add:

```typescript
const browserTz = getBrowserTimezone();
const tzLabel = getTimezoneLabel(browserTz);

// Convert UTC slots → local slots for display
const localSlots = (slots: TimeSlot[] | null | undefined): TimeSlot[] | null => {
  if (!slots || slots.length === 0) return slots;
  return slots.map(s => ({
    days: s.days.map(d => utcDayToLocalDay(d, s.start, browserTz)),
    start: utcToLocalTime(s.start, browserTz),
    end: utcToLocalTime(s.end, browserTz),
  }));
};

// Convert local slots → UTC slots for saving
const toUtcSlots = (local: TimeSlot[]): TimeSlot[] =>
  local.map(s => ({
    days: s.days.map(d => localDayToUtcDay(d, s.start, browserTz)),
    start: localToUtcTime(s.start, browserTz),
    end: localToUtcTime(s.end, browserTz),
  }));
```

- [ ] **Step 2: Update the Available Hours display card (lines 384-411)**

Change the header to include the timezone label:

```tsx
<h2 className="text-sm font-semibold text-base-content/60">{t('channelDetail.availableHours')} <span className="text-xs font-normal text-base-content/30">{tzLabel}</span></h2>
```

Change the slot rendering to use `localSlots(channel.available_hours)`:

```tsx
{(() => {
  const displayed = localSlots(channel.available_hours);
  return displayed && displayed.length > 0 ? (
    <div className="space-y-2">
      {displayed.map((slot, i) => (
        <div key={i} className="flex items-center gap-3 p-3 bg-base-200/50 rounded-lg">
          <Clock className="h-4 w-4 text-primary shrink-0" />
          <div className="flex-1">
            <span className="font-mono text-sm text-base-content/80">{slot.start} – {slot.end}</span>
          </div>
          <div className="flex gap-1 flex-wrap justify-end">
            {slot.days.map(d => (
              <span key={d} className="px-2 py-0.5 bg-primary/10 text-primary/80 rounded text-xs font-medium capitalize">{d}</span>
            ))}
          </div>
        </div>
      ))}
    </div>
  ) : (
    <div className="text-sm text-base-content/30 italic">{t('channelDetail.alwaysAvailable')}</div>
  );
})()}
```

- [ ] **Step 3: Update the edit modal to work in local time**

Change the "Edit" button `onClick` (line 388) to convert UTC→local when opening:

```tsx
onClick={() => {
  const local = localSlots(channel.available_hours ?? []);
  setHoursSlots(local ?? []);
  setEditingHours(true);
}}
```

Change the form submit (line 853) to convert local→UTC before saving:

```tsx
await updateMutation.mutateAsync({
  id: channel.id,
  input: { available_hours: toUtcSlots(hoursSlots) },
});
```

Change the default new slot (line 901) to use local time defaults:

```tsx
onClick={() => setHoursSlots([...hoursSlots, { days: ['mon','tue','wed','thu','fri'], start: utcToLocalTime('09:00', browserTz), end: utcToLocalTime('17:00', browserTz) }])}
```

- [ ] **Step 4: Verify the app builds**

Run: `cd /workspace/llm-gateway/web && npm run build 2>&1 | tail -5`
Expected: Build succeeds

- [ ] **Step 5: Commit**

```bash
cd /workspace/llm-gateway
git add web/src/pages/ChannelDetail.tsx
git commit -m "feat(web): display and edit available hours in browser timezone"
```

---

### Task 3: Update Channels.tsx — timezone-aware availability check and display

**Files:**
- Modify: `web/src/pages/Channels.tsx:22-35` (isAvailableNow helper)
- Modify: `web/src/pages/Channels.tsx:495-523` (channel row availability display)

- [ ] **Step 1: Replace the hardcoded UTC isAvailableNow with timezone-aware import**

Add import at the top of the file:

```typescript
import { isAvailableNow, utcToLocalTime, utcDayToLocalDay, getBrowserTimezone, getTimezoneLabel } from '../lib/timezone';
```

Delete the entire `isAvailableNow` function (lines 22-35). The imported version from `timezone.ts` replaces it.

- [ ] **Step 2: Update the channel row availability display to use local times**

Add at the top of the component function body:

```typescript
const browserTz = getBrowserTimezone();
```

Update the availability check call (line 496) to pass the timezone:

```tsx
const available = isAvailableNow(channel.available_hours, browserTz);
```

Update the time slot display (line 505-515) to convert to local time:

```tsx
{channel.available_hours.map((slot, i) => {
  const localStart = utcToLocalTime(slot.start, browserTz);
  const localEnd = utcToLocalTime(slot.end, browserTz);
  const localDays = slot.days.map(d => utcDayToLocalDay(d, slot.start, browserTz));
  return (
    <div key={i} className="flex items-center gap-1.5">
      <Clock className="h-3 w-3 text-base-content/35 shrink-0" />
      <span className="text-md font-mono text-base-content/50 whitespace-nowrap">{localStart}–{localEnd}</span>
      <div className="flex gap-0.5">
        {localDays.map(d => (
          <span key={d} className="text-sm font-medium text-primary/70 bg-primary/8 px-1 rounded">{d.slice(0, 3)}</span>
        ))}
      </div>
    </div>
  );
})}
```

- [ ] **Step 3: Verify the app builds**

Run: `cd /workspace/llm-gateway/web && npm run build 2>&1 | tail -5`
Expected: Build succeeds

- [ ] **Step 4: Commit**

```bash
cd /workspace/llm-gateway
git add web/src/pages/Channels.tsx
git commit -m "feat(web): use timezone-aware availability check in channel list"
```

---

### Task 4: Update i18n strings

**Files:**
- Modify: `web/src/i18n/en.json`
- Modify: `web/src/i18n/zh.json`

- [ ] **Step 1: Update English translations**

In `en.json`, update the `editHoursModal.description` value (around line 562):

Change:
```json
"description": "Restrict this channel to specific days and times. Leave empty to make it always available. All times are in UTC.",
```

To:
```json
"description": "Restrict this channel to specific days and times. Leave empty to make it always available. Times are shown in your browser timezone.",
```

- [ ] **Step 2: Update Chinese translations**

In `zh.json`, find the same `editHoursModal.description` key and update:

Change (the existing Chinese equivalent):
```json
"description": "限制此通道在特定日期和时间可用。留空表示始终可用。所有时间均为 UTC。",
```

To:
```json
"description": "限制此通道在特定日期和时间可用。留空表示始终可用。时间以您的浏览器时区显示。",
```

- [ ] **Step 3: Verify build**

Run: `cd /workspace/llm-gateway/web && npm run build 2>&1 | tail -5`
Expected: Build succeeds

- [ ] **Step 4: Commit**

```bash
cd /workspace/llm-gateway
git add web/src/i18n/en.json web/src/i18n/zh.json
git commit -m "feat(web): update i18n strings for timezone-aware available hours"
```

---

### Task 5: Run all tests and verify

- [ ] **Step 1: Run the full frontend test suite**

Run: `cd /workspace/llm-gateway/web && npm test 2>&1`
Expected: All tests pass

- [ ] **Step 2: Run frontend build**

Run: `cd /workspace/llm-gateway/web && npm run build 2>&1`
Expected: Build succeeds with no errors

- [ ] **Step 3: Final commit if any fixes needed**

Only if tests or build required fixes. Otherwise skip this step.
