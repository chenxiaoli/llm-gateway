import { useEffect, useCallback } from 'react';
import { X } from 'lucide-react';
import { cn } from '../../lib/cn';

export interface ModalProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  children: React.ReactNode;
  footer?: React.ReactNode;
  className?: string;
}

export function Modal({ open, onClose, title, children, footer, className }: ModalProps) {
  const handleEscape = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    },
    [onClose],
  );

  useEffect(() => {
    if (open) {
      document.addEventListener('keydown', handleEscape);
      document.body.style.overflow = 'hidden';
    }
    return () => {
      document.removeEventListener('keydown', handleEscape);
      document.body.style.overflow = '';
    };
  }, [open, handleEscape]);

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-[200] flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/60 backdrop-blur-sm"
        onClick={onClose}
      />

      {/* Panel */}
      <div
        className={cn(
          'relative z-10 w-full max-w-lg rounded-xl border border-[#1e1e1e] bg-[#111111] p-6 animate-fade-in-up',
          className,
        )}
      >
        {/* Header */}
        {title && (
          <div className="mb-4 flex items-center justify-between">
            <h2 className="text-lg font-semibold text-[#ededed]">{title}</h2>
            <button
              onClick={onClose}
              className="rounded-lg p-1 text-[#888888] hover:bg-white/[0.04] hover:text-[#ededed] transition-colors cursor-pointer"
            >
              <X className="h-4 w-4" />
            </button>
          </div>
        )}

        {/* Body */}
        <div className="text-sm text-[#ededed]">{children}</div>

        {/* Footer */}
        {footer && <div className="mt-6 flex items-center justify-end gap-3">{footer}</div>}
      </div>
    </div>
  );
}
