import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ProgressBar } from './ProgressBar';

describe('ProgressBar', () => {
  it('renders a track with a fill', () => {
    const { container } = render(<ProgressBar pct={50} colorClass="bg-emerald-500" />);
    const fill = container.querySelector('[data-testid="progress-fill"]') as HTMLElement | null;
    expect(fill).not.toBeNull();
    expect(fill!.style.width).toBe('50%');
    expect(fill!.className).toContain('bg-emerald-500');
  });

  it('clamps pct above 100 to width 100%', () => {
    const { container } = render(<ProgressBar pct={150} colorClass="bg-red-500" />);
    const fill = container.querySelector('[data-testid="progress-fill"]') as HTMLElement;
    expect(fill.style.width).toBe('100%');
  });

  it('clamps negative pct to 0%', () => {
    const { container } = render(<ProgressBar pct={-5} colorClass="bg-emerald-500" />);
    const fill = container.querySelector('[data-testid="progress-fill"]') as HTMLElement;
    expect(fill.style.width).toBe('0%');
  });

  it('applies the color class to the fill, not the track', () => {
    const { container } = render(<ProgressBar pct={75} colorClass="bg-amber-500" />);
    const fill = container.querySelector('[data-testid="progress-fill"]') as HTMLElement;
    const track = container.querySelector('[data-testid="progress-track"]') as HTMLElement;
    expect(fill.className).toContain('bg-amber-500');
    expect(track.className).not.toContain('bg-amber-500');
  });
});
