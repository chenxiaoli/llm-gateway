import { ChevronDown } from 'lucide-react';
import { cn } from '../../lib/cn';

const sizeStyles = {
  sm: 'h-8 px-3 text-xs',
  md: 'h-9 px-4 text-sm',
} as const;

export interface SelectProps {
  value: string;
  onChange: (value: string) => void;
  options: { value: string; label: string }[];
  placeholder?: string;
  className?: string;
  size?: keyof typeof sizeStyles;
}

export function Select({ value, onChange, options, placeholder, className, size = 'md' }: SelectProps) {
  return (
    <div className="relative">
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className={cn(
          'w-full appearance-none rounded-lg border border-[#262626] bg-[#111111] text-[#ededed] pr-8 focus:outline-none focus:border-accent/50 transition-colors cursor-pointer',
          sizeStyles[size],
          className,
        )}
      >
        {placeholder && (
          <option value="" disabled>
            {placeholder}
          </option>
        )}
        {options.map((opt) => (
          <option key={opt.value} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>
      <ChevronDown className="pointer-events-none absolute right-2 top-1/2 h-4 w-4 -translate-y-1/2 text-[#888888]" />
    </div>
  );
}
