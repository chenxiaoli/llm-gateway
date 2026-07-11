/**
 * Phase 7 budget-observability color helpers.
 *
 * Shared by the OrgSettings "Budget status" card and the Keys-table MTD
 * column. Thresholds match the design spec:
 *   - null pct (no budget set) → muted gray, no bar fill
 *   - < 60% → green
 *   - 60-79% → amber
 *   - 80-100% → orange
 *   - > 100% → red
 *
 * Returned strings are Tailwind class names; consumers apply them directly
 * to the bar's `className`. The classes used here must exist in the project's
 * Tailwind setup (verified: `bg-emerald-500`, `bg-amber-500`, `bg-orange-500`,
 * `bg-red-500`, and `bg-muted` are all standard DaisyUI / Tailwind tokens).
 */

export function budgetUsedPct(accruedUnits: number, budgetUnits: number | null): number | null {
  if (budgetUnits === null || budgetUnits === 0) return null;
  return (accruedUnits / budgetUnits) * 100;
}

export function budgetBarColor(usedPct: number | null): string {
  if (usedPct === null) return 'bg-muted';
  if (usedPct > 100) return 'bg-red-500';
  if (usedPct >= 80) return 'bg-orange-500';
  if (usedPct >= 60) return 'bg-amber-500';
  return 'bg-emerald-500';
}
