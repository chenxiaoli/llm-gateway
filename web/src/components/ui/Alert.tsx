import { Info, CheckCircle, AlertCircle } from 'lucide-react';
import { cn } from '../../lib/cn';

const variantConfig = {
  info: {
    container: 'border-info/30 bg-info/10 text-info',
    Icon: Info,
  },
  success: {
    container: 'border-accent/30 bg-accent/10 text-accent',
    Icon: CheckCircle,
  },
  warning: {
    container: 'border-warning/30 bg-warning/10 text-warning',
    Icon: AlertCircle,
  },
  error: {
    container: 'border-danger/30 bg-danger/10 text-danger',
    Icon: AlertCircle,
  },
} as const;

export interface AlertProps {
  variant?: keyof typeof variantConfig;
  children: React.ReactNode;
  onClose?: () => void;
  className?: string;
}

export function Alert({ variant = 'info', children, onClose, className }: AlertProps) {
  const { container, Icon } = variantConfig[variant];

  return (
    <div className={cn('flex items-start gap-3 rounded-lg border p-3 text-sm', container, className)}>
      <Icon className="mt-0.5 h-4 w-4 shrink-0" />
      <div className="flex-1">{children}</div>
      {onClose && (
        <button
          onClick={onClose}
          className="shrink-0 rounded p-0.5 opacity-70 hover:opacity-100 transition-opacity cursor-pointer"
        >
          <AlertCircle className="h-3.5 w-3.5" />
        </button>
      )}
    </div>
  );
}
