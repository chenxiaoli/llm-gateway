import React, { forwardRef } from 'react';
import { Loader2 } from 'lucide-react';
import { cn } from '../../lib/cn';

const variantStyles = {
  primary: 'bg-accent text-black hover:bg-accent-hover font-medium shadow-[0_0_20px_rgba(6,214,160,0.2)]',
  secondary: 'border border-[#3a3a3a] text-[#e0e0e0] hover:bg-white/[0.06] hover:border-[#4a4a4a]',
  ghost: 'bg-transparent text-[#999999] hover:bg-white/[0.06] hover:text-[#cccccc]',
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
