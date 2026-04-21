import { useEffect, useState } from 'react';
import { createPortal } from 'react-dom';

export interface DrawerProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  children: React.ReactNode;
  width?: number;
  /** Render inside a portal (default true). Set false to render inline. */
  portal?: boolean;
}

export function Drawer({ open, onClose, title, children, width = 480, portal = true }: DrawerProps) {
  const [phase, setPhase] = useState<'closed' | 'entering' | 'open' | 'exiting'>('closed');

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && phase === 'open') onClose();
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [phase, onClose]);

  useEffect(() => {
    document.body.style.overflow = phase === 'open' || phase === 'entering' ? 'hidden' : '';
    return () => { document.body.style.overflow = ''; };
  }, [phase]);

  useEffect(() => {
    if (open) {
      setPhase('entering');
      const timer = setTimeout(() => setPhase('open'), 280);
      return () => clearTimeout(timer);
    } else if (phase !== 'closed') {
      setPhase('exiting');
      const timer = setTimeout(() => setPhase('closed'), 280);
      return () => clearTimeout(timer);
    }
  }, [open]);

  if (phase === 'closed') return null;

  const panel = (
    <div
      onClick={onClose}
      className={`fixed inset-0 z-[110] ${
        phase === 'entering'
          ? 'drawer-backdrop-enter'
          : phase === 'exiting'
          ? 'drawer-backdrop-exit'
          : ''
      }`}
      style={{ background: 'rgba(0, 0, 0, 0.5)' }}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className={`absolute right-0 top-0 bottom-0 flex flex-col bg-base-100 border-base-300 border-l ${
          phase === 'entering'
            ? 'drawer-panel-enter'
            : phase === 'exiting'
            ? 'drawer-panel-exit'
            : ''
        }`}
        style={{
          width: `${width}px`,
          maxWidth: '100vw',
          boxShadow: '-8px 0 32px rgba(0, 0, 0, 0.4)',
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
    </div>
  );

  if (!portal) return panel;
  return createPortal(panel, document.body);
}
