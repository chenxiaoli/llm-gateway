import { cn } from '../../lib/cn';

const sizeMap = {
  sm: 'select-sm',
  md: 'select-md',
} as const;

export interface SelectProps {
  value: string;
  onChange: (value: string) => void;
  options: { value: string; label: string }[];
  placeholder?: string;
  className?: string;
  size?: keyof typeof sizeMap;
}

export function Select({ value, onChange, options, placeholder, className, size = 'md' }: SelectProps) {
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className={cn('select select-bordered w-full', sizeMap[size], className)}
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
  );
}
