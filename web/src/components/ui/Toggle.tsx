import { cn } from '../../lib/cn';

export interface ToggleProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}

export function Toggle({ checked, onChange, disabled }: ToggleProps) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      onClick={() => onChange(!checked)}
      disabled={disabled}
      className={cn(
        'relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full transition-colors duration-200',
        checked ? 'bg-accent' : 'bg-[#262626]',
        disabled && 'opacity-50 pointer-events-none',
      )}
    >
      <span
        className={cn(
          'pointer-events-none inline-block h-4 w-4 rounded-full bg-white shadow-sm transition-transform duration-200',
          checked ? 'translate-x-4' : 'translate-x-0.5',
        )}
      />
    </button>
  );
}
