import { useEffect, useCallback } from 'react';
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
    <dialog className={cn('modal modal-open')}>
      <div
        className="modal-box max-w-none p-0 flex flex-col"
        style={{ width: `${width}px`, maxWidth: '100vw' }}
      >
        <div className="flex items-center justify-between border-b border-base-300 px-6 py-4">
          {title && <h3 className="text-lg font-semibold">{title}</h3>}
          <button className="btn btn-sm btn-circle btn-ghost" onClick={onClose}>✕</button>
        </div>
        <div className="flex-1 overflow-y-auto px-6 py-4">{children}</div>
      </div>
      <form method="dialog" className="modal-backdrop">
        <button onClick={onClose}>close</button>
      </form>
    </dialog>
  );
}
