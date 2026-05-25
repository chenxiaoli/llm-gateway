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

beforeEach(() => {
  vi.restoreAllMocks();
});

describe('utcToLocalTime / localToUtcTime', () => {
  it('converts UTC to local and back', () => {
    const utc = '01:00';
    const local = utcToLocalTime(utc, 'Asia/Shanghai');
    expect(local).toBe('09:00');
    expect(localToUtcTime(local, 'Asia/Shanghai')).toBe('01:00');
  });

  it('handles midnight cross', () => {
    expect(utcToLocalTime('23:00', 'Asia/Shanghai')).toBe('07:00');
    expect(localToUtcTime('07:00', 'Asia/Shanghai')).toBe('23:00');
  });

  it('handles negative offset', () => {
    const local = utcToLocalTime('01:00', 'America/New_York');
    expect(localToUtcTime(local, 'America/New_York')).toBe('01:00');
  });

  it('pads single-digit hours', () => {
    const result = utcToLocalTime('00:00', 'Asia/Shanghai');
    expect(result).toMatch(/^\d{2}:\d{2}$/);
  });
});

describe('utcDayToLocalDay / localDayToUtcDay', () => {
  it('shifts day forward when crossing midnight', () => {
    expect(utcDayToLocalDay('mon', '23:00', 'Asia/Shanghai')).toBe('tue');
  });

  it('shifts day backward for negative offset', () => {
    const result = utcDayToLocalDay('tue', '01:00', 'America/New_York');
    expect(result).toBe('mon');
  });

  it('keeps same day when no midnight cross', () => {
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
    const fixedDate = new Date('2026-01-05T10:30:00Z');
    vi.useFakeTimers();
    vi.setSystemTime(fixedDate);

    const slots = [{ days: ['mon'], start: '02:00', end: '03:00' }];
    expect(isAvailableNow(slots, 'UTC')).toBe(false);

    const slots2 = [{ days: ['mon'], start: '10:00', end: '11:00' }];
    expect(isAvailableNow(slots2, 'UTC')).toBe(true);

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
