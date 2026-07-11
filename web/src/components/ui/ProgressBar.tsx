/**
 * Phase 7: reusable color-coded progress bar.
 *
 * Purely presentational — caller decides pct and color. Used by the
 * OrgSettings Budget status card (large) and the Keys-table MTD column
 * (mini). The track is always muted; the fill carries the semantic color.
 */

export function ProgressBar({
  pct,
  colorClass,
  size = 'md',
}: {
  pct: number;
  colorClass: string;
  size?: 'sm' | 'md';
}) {
  const clamped = Math.max(0, Math.min(100, pct));
  const trackHeight = size === 'sm' ? 'h-1.5' : 'h-2.5';
  return (
    <div
      data-testid="progress-track"
      className={`w-full ${trackHeight} rounded-full bg-muted overflow-hidden`}
    >
      <div
        data-testid="progress-fill"
        className={`${colorClass} h-full rounded-full transition-all`}
        style={{ width: `${clamped}%` }}
      />
    </div>
  );
}
