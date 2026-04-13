import { cn } from '../../lib/cn';

const sizeMap = {
  sm: 'loading-xs',
  md: 'loading-sm',
  lg: 'loading-md',
} as const;

export interface LoadingSpinnerProps {
  size?: keyof typeof sizeMap;
  className?: string;
}

export function LoadingSpinner({ size = 'md', className }: LoadingSpinnerProps) {
  return (
    <span className={cn('loading loading-spinner', sizeMap[size], className)} />
  );
}
