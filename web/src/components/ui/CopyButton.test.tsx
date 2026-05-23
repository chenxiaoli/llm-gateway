import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { CopyButton } from './CopyButton';

describe('CopyButton', () => {
  beforeEach(() => {
    Object.assign(navigator, {
      clipboard: {
        writeText: vi.fn().mockResolvedValue(undefined),
      },
    });
  });

  it('renders a copy icon button', () => {
    render(<CopyButton value="test-value" />);
    const button = screen.getByRole('button');
    expect(button).toBeInTheDocument();
  });

  it('copies value to clipboard on click', () => {
    render(<CopyButton value="test-value" />);
    fireEvent.click(screen.getByRole('button'));
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith('test-value');
  });

  it('swaps to check icon after click', () => {
    render(<CopyButton value="test-value" />);
    fireEvent.click(screen.getByRole('button'));
    const svg = screen.getByRole('button').querySelector('svg');
    expect(svg?.className.baseVal).toContain('text-success');
  });
});
