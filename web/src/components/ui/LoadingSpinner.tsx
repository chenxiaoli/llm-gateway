import { cn } from '../../lib/cn';

const sizeStyles = {
  sm: 'h-4 w-4',
  md: 'h-8 w-8',
  lg: 'h-12 w-12',
} as const;

export interface LoadingSpinnerProps {
  size?: keyof typeof sizeStyles;
  className?: string;
}

export function LoadingSpinner({ size = 'md', className }: LoadingSpinnerProps) {
  return (
    <div
      className={cn(
        'animate-spin rounded-full border-2 border-[#262626] border-t-accent',
        sizeStyles[size],
        className,
      )}
    />
  );
}
