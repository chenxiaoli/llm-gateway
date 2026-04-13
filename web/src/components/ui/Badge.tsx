import { cn } from '../../lib/cn';

const variantMap = {
  green: 'badge-success',
  red: 'badge-error',
  amber: 'badge-warning',
  blue: 'badge-info',
  purple: 'badge-primary',
  neutral: 'badge-ghost',
} as const;

export interface BadgeProps {
  variant?: keyof typeof variantMap;
  children: React.ReactNode;
  className?: string;
}

export function Badge({ variant = 'neutral', children, className }: BadgeProps) {
  return (
    <span className={cn('badge badge-sm font-mono', variantMap[variant], className)}>
      {children}
    </span>
  );
}
