import { describe, it, expect } from 'vitest';
import { budgetBarColor, budgetUsedPct } from './budgetColor';

describe('budgetUsedPct', () => {
  it('returns null when budget is null', () => {
    expect(budgetUsedPct(100, null)).toBeNull();
  });

  it('returns null when budget is 0', () => {
    expect(budgetUsedPct(0, 0)).toBeNull();
    expect(budgetUsedPct(100, 0)).toBeNull();
  });

  it('returns 0 when accrued is 0', () => {
    expect(budgetUsedPct(0, 100)).toBe(0);
  });

  it('returns the percentage at boundary points', () => {
    expect(budgetUsedPct(50, 100)).toBe(50);
    expect(budgetUsedPct(80, 100)).toBe(80);
    expect(budgetUsedPct(100, 100)).toBe(100);
    expect(budgetUsedPct(105, 100)).toBe(105);
  });
});

describe('budgetBarColor', () => {
  it('returns muted class when pct is null', () => {
    expect(budgetBarColor(null)).toBe('bg-muted');
  });

  it('returns green below 60%', () => {
    expect(budgetBarColor(0)).toBe('bg-emerald-500');
    expect(budgetBarColor(30)).toBe('bg-emerald-500');
    expect(budgetBarColor(59)).toBe('bg-emerald-500');
  });

  it('returns amber at 60% inclusive to 80% exclusive', () => {
    expect(budgetBarColor(60)).toBe('bg-amber-500');
    expect(budgetBarColor(79)).toBe('bg-amber-500');
  });

  it('returns orange at 80% inclusive to 100% inclusive', () => {
    expect(budgetBarColor(80)).toBe('bg-orange-500');
    expect(budgetBarColor(99)).toBe('bg-orange-500');
    expect(budgetBarColor(100)).toBe('bg-orange-500');
  });

  it('returns red over 100%', () => {
    expect(budgetBarColor(101)).toBe('bg-red-500');
    expect(budgetBarColor(105)).toBe('bg-red-500');
  });
});
