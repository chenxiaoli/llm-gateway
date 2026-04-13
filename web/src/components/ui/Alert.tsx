import { Info, CheckCircle, AlertCircle } from 'lucide-react';
import { cn } from '../../lib/cn';

const variantMap = {
  info: 'alert-info',
  success: 'alert-success',
  warning: 'alert-warning',
  error: 'alert-error',
} as const;

export interface AlertProps {
  variant?: keyof typeof variantMap;
  children: React.ReactNode;
  onClose?: () => void;
  className?: string;
}

export function Alert({ variant = 'info', children, onClose, className }: AlertProps) {
  return (
    <div className={cn('alert', variantMap[variant], className)}>
      <Info className={cn(variant !== 'info' && 'hidden')} />
      <CheckCircle className={cn(variant !== 'success' && 'hidden')} />
      <AlertCircle className={cn(variant !== 'warning' && variant !== 'error' && 'hidden')} />
      <span>{children}</span>
      {onClose && (
        <button className="btn btn-sm btn-ghost" onClick={onClose}>✕</button>
      )}
    </div>
  );
}
