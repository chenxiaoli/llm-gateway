import React, { forwardRef } from 'react';
import { cn } from '../../lib/cn';

const variantMap = {
  primary: 'btn-primary',
  secondary: 'btn-secondary',
  ghost: 'btn-ghost',
  danger: 'btn-error',
  outline: 'btn-outline',
} as const;

const sizeMap = {
  sm: 'btn-sm',
  md: 'btn-md',
  lg: 'btn-lg',
} as const;

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: keyof typeof variantMap;
  size?: keyof typeof sizeMap;
  loading?: boolean;
  icon?: React.ReactNode;
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant = 'primary', size = 'md', loading, icon, children, disabled, ...props }, ref) => {
    return (
      <button
        ref={ref}
        className={cn(
          'btn',
          variantMap[variant],
          sizeMap[size],
          loading && 'btn-disabled',
          !loading && 'cursor-pointer',
          className,
        )}
        disabled={loading || disabled}
        {...props}
      >
        {loading ? <span className="loading loading-spinner loading-sm" /> : icon ? icon : null}
        {children}
      </button>
    );
  },
);

Button.displayName = 'Button';
