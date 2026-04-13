import { useEffect, useCallback, useRef } from 'react';

export interface DrawerProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  children: React.ReactNode;
  width?: number;
}

export function Drawer({ open, onClose, title, children, width = 640 }: DrawerProps) {
  const dialogRef = useRef<HTMLDialogElement>(null);

  const close = useCallback(() => {
    dialogRef.current?.close();
  }, []);

  useEffect(() => {
    if (open) {
      try {
        dialogRef.current?.showModal();
      } catch {
        dialogRef.current?.setAttribute('open', '');
      }
    }
  }, [open]);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    const handler = () => onClose();
    dialog.addEventListener('close', handler);
    return () => dialog.removeEventListener('close', handler);
  }, [onClose]);

  useEffect(() => {
    if (open) {
      document.body.style.overflow = 'hidden';
    }
    return () => {
      document.body.style.overflow = '';
    };
  }, [open]);

  return (
    <dialog ref={dialogRef} className="modal">
      <div
        className="modal-box max-w-none p-0 flex flex-col"
        style={{ width: `${width}px`, maxWidth: '100vw' }}
      >
        <div className="flex items-center justify-between border-b border-base-300 px-6 py-4">
          {title && <h3 className="text-lg font-semibold">{title}</h3>}
          <button className="btn btn-sm btn-circle btn-ghost" onClick={close}>✕</button>
        </div>
        <div className="flex-1 overflow-y-auto px-6 py-4">{children}</div>
      </div>
      <form method="dialog" className="modal-backdrop">
        <button>close</button>
      </form>
    </dialog>
  );
}
