import { cn } from '../../lib/cn';

const variantStyles = {
  green: 'border-accent/30 bg-accent/10 text-accent',
  red: 'border-danger/30 bg-danger/10 text-danger',
  amber: 'border-warning/30 bg-warning/10 text-warning',
  blue: 'border-info/30 bg-info/10 text-info',
  purple: 'border-purple/30 bg-purple/10 text-purple',
  neutral: 'border-[#262626] bg-white/[0.04] text-[#888888]',
} as const;

export interface BadgeProps {
  variant?: keyof typeof variantStyles;
  children: React.ReactNode;
  className?: string;
}

export function Badge({ variant = 'neutral', children, className }: BadgeProps) {
  return (
    <span
      className={cn(
        'inline-flex items-center rounded-full border px-2 py-0.5 text-xs font-medium',
        variantStyles[variant],
        className,
      )}
    >
      {children}
    </span>
  );
}
