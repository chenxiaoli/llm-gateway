import React, { forwardRef } from 'react';
import { Loader2 } from 'lucide-react';
import { cn } from '../../lib/cn';

const variantStyles = {
  primary: 'bg-accent text-black hover:bg-accent-hover font-medium',
  secondary: 'border border-[#262626] text-[#ededed] hover:bg-white/[0.04]',
  ghost: 'bg-transparent text-[#888888] hover:bg-white/[0.04]',
  danger: 'border border-danger/50 text-danger hover:bg-danger/10',
} as const;

const sizeStyles = {
  sm: 'h-8 px-3 text-xs',
  md: 'h-9 px-4 text-sm',
  lg: 'h-10 px-5 text-sm',
} as const;

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: keyof typeof variantStyles;
  size?: keyof typeof sizeStyles;
  loading?: boolean;
  icon?: React.ReactNode;
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant = 'primary', size = 'md', loading, icon, children, disabled, ...props }, ref) => {
    return (
      <button
        ref={ref}
        className={cn(
          'inline-flex items-center justify-center gap-2 rounded-lg transition-colors duration-150 cursor-pointer whitespace-nowrap',
          variantStyles[variant],
          sizeStyles[size],
          (loading || disabled) && 'pointer-events-none opacity-50',
          className,
        )}
        disabled={loading || disabled}
        {...props}
      >
        {loading ? <Loader2 className="h-4 w-4 animate-spin" /> : icon ? icon : null}
        {children}
      </button>
    );
  },
);

Button.displayName = 'Button';
