import { useEffect, useCallback } from 'react';
import { X } from 'lucide-react';
import { cn } from '../../lib/cn';

export interface DrawerProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  children: React.ReactNode;
  width?: number;
}

export function Drawer({ open, onClose, title, children, width = 640 }: DrawerProps) {
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
    <div className="fixed inset-0 z-[200] flex justify-end">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/60 backdrop-blur-sm"
        onClick={onClose}
      />

      {/* Panel */}
      <div
        className="relative z-10 flex h-full flex-col border-l border-[#1e1e1e] bg-[#111111] animate-fade-in-up"
        style={{ width: `${width}px`, maxWidth: '100vw' }}
      >
        {/* Sticky Header */}
        <div className="sticky top-0 z-10 flex items-center justify-between border-b border-[#1e1e1e] bg-[#111111] px-6 py-4">
          {title && <h2 className="text-lg font-semibold text-[#ededed]">{title}</h2>}
          <button
            onClick={onClose}
            className={cn(
              'rounded-lg p-1 text-[#888888] hover:bg-white/[0.04] hover:text-[#ededed] transition-colors cursor-pointer',
              !title && 'ml-auto',
            )}
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Scrollable Content */}
        <div className="flex-1 overflow-y-auto px-6 py-4">{children}</div>
      </div>
    </div>
  );
}
