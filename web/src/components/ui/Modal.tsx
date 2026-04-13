import { useEffect, useRef } from 'react';
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
  const dialogRef = useRef<HTMLDialogElement>(null);
  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;

  // Sync dialog open/close state with React prop
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (open) {
      try {
        dialog.showModal();
      } catch {
        // Already open — ignore
      }
    } else if (dialog.open) {
      dialog.close();
    }
  }, [open]);

  // Listen for native close event (backdrop click, Escape key, form submit)
  // Uses a ref for onClose so the listener stays stable across renders
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    const handler = () => onCloseRef.current();
    dialog.addEventListener('close', handler);
    return () => dialog.removeEventListener('close', handler);
  }, []);

  return (
    <dialog ref={dialogRef} className={cn('modal', className)}>
      <div className="modal-box">
        {title && (
          <div className="flex items-center justify-between mb-5">
            <h3 className="text-lg font-semibold">{title}</h3>
            <button className="btn btn-sm btn-circle btn-ghost" onClick={() => dialogRef.current?.close()}>✕</button>
          </div>
        )}

        <div>{children}</div>

        {footer && <div className="modal-action">{footer}</div>}
      </div>
      <form method="dialog" className="modal-backdrop">
        <button>close</button>
      </form>
    </dialog>
  );
}
