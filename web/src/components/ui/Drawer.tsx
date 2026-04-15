import { useEffect } from 'react';
import { createPortal } from 'react-dom';
import { cn } from '../../lib/cn';

export interface DrawerProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  children: React.ReactNode;
  width?: number;
}

export function Drawer({ open, onClose, title, children, width = 480 }: DrawerProps) {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && open) onClose();
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [open, onClose]);

  useEffect(() => {
    document.body.style.overflow = open ? 'hidden' : '';
    return () => { document.body.style.overflow = ''; };
  }, [open]);

  if (!open) return null;

  return createPortal(
    <div
      onClick={onClose}
      className="fixed inset-0 z-[110]"
      style={{ background: 'rgba(0, 0, 0, 0.5)' }}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className={cn(
          'absolute right-0 top-0 bottom-0 flex flex-col',
          'bg-base-100 border-base-300 border-l',
        )}
        style={{
          width: `${width}px`,
          maxWidth: '100vw',
          boxShadow: '-8px 0 32px rgba(0, 0, 0, 0.4)',
          animation: 'slideInRight 0.25s cubic-bezier(0.32, 0.72, 0, 1) forwards',
        }}
      >
        <div className="flex items-center justify-between px-6 py-5 shrink-0 border-b border-base-300">
          {title && (
            <h3 className="text-[15px] font-semibold text-base-content">{title}</h3>
          )}
          <button
            onClick={onClose}
            aria-label="Close"
            className="w-7 h-7 flex items-center justify-center rounded-md border-none bg-transparent cursor-pointer text-base-content/50 hover:text-base-content hover:bg-base-200 transition-all"
          >
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
              <path d="M1 1l12 12M13 1L1 13" />
            </svg>
          </button>
        </div>
        <div className="flex-1 overflow-y-auto px-6 py-5">
          {children}
        </div>
      </div>
      <style>{`
        @keyframes slideInRight {
          from { transform: translateX(100%); }
          to { transform: translateX(0); }
        }
      `}</style>
    </div>,
    document.body,
  );
}
