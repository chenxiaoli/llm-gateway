import type { TimeSlot } from '../types';

const DAYS = ['sun', 'mon', 'tue', 'wed', 'thu', 'fri', 'sat'] as const;
export const DAY_ORDER = ['mon', 'tue', 'wed', 'thu', 'fri', 'sat', 'sun'] as const;

function getUtcOffsetMinutes(tz: string, refDate: Date = new Date()): number {
  const utcStr = refDate.toLocaleString('en-US', { timeZone: 'UTC' });
  const tzStr = refDate.toLocaleString('en-US', { timeZone: tz });
  const utcDate = new Date(utcStr);
  const tzDate = new Date(tzStr);
  return (tzDate.getTime() - utcDate.getTime()) / 60000;
}

function hhmmToMinutes(hhmm: string): number {
  const [h, m] = hhmm.split(':').map(Number);
  return (h || 0) * 60 + (m || 0);
}

function minutesToHHMM(minutes: number): string {
  const m = ((minutes % 1440) + 1440) % 1440;
  const h = Math.floor(m / 60);
  const min = m % 60;
  return `${String(h).padStart(2, '0')}:${String(min).padStart(2, '0')}`;
}

export function utcToLocalTime(utcHHMM: string, tz: string): string {
  const offset = getUtcOffsetMinutes(tz);
  return minutesToHHMM(hhmmToMinutes(utcHHMM) + offset);
}

export function localToUtcTime(localHHMM: string, tz: string): string {
  const offset = getUtcOffsetMinutes(tz);
  return minutesToHHMM(hhmmToMinutes(localHHMM) - offset);
}

function getDayShift(utcHHMM: string, tz: string): number {
  const offset = getUtcOffsetMinutes(tz);
  const localMinutes = hhmmToMinutes(utcHHMM) + offset;
  if (localMinutes >= 1440) return 1;
  if (localMinutes < 0) return -1;
  return 0;
}

export function utcDayToLocalDay(utcDay: string, utcHHMM: string, tz: string): string {
  const shift = getDayShift(utcHHMM, tz);
  if (shift === 0) return utcDay;
  const idx = DAYS.indexOf(utcDay as typeof DAYS[number]);
  return DAYS[(idx + shift + 7) % 7];
}

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

export function isAvailableNow(slots: TimeSlot[] | null | undefined, _tz: string): boolean {
  if (!slots || slots.length === 0) return true;
  const now = new Date();
  const utcDay = now.toLocaleDateString('en-US', { weekday: 'short', timeZone: 'UTC' }).toLowerCase();
  const nowUtcMinutes = now.getUTCHours() * 60 + now.getUTCMinutes();

  return slots.some(slot => {
    if (!slot.days.includes(utcDay)) return false;
    const start = hhmmToMinutes(slot.start);
    const end = hhmmToMinutes(slot.end);
    if (start <= end) {
      return nowUtcMinutes >= start && nowUtcMinutes < end;
    }
    return nowUtcMinutes >= start || nowUtcMinutes < end;
  });
}

export function getTimezoneLabel(tz: string): string {
  const offset = getUtcOffsetMinutes(tz);
  const sign = offset >= 0 ? '+' : '-';
  const absOffset = Math.abs(offset);
  const hours = Math.floor(absOffset / 60);
  const minutes = absOffset % 60;
  const offsetStr = minutes > 0 ? `${sign}${hours}:${String(minutes).padStart(2, '0')}` : `${sign}${hours}`;
  return `UTC${offsetStr} (${tz})`;
}

export function getBrowserTimezone(): string {
  return Intl.DateTimeFormat().resolvedOptions().timeZone;
}
