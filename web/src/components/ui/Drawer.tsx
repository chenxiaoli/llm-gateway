import { useEffect, useRef } from 'react';

export interface DrawerProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  children: React.ReactNode;
  width?: number;
}

export function Drawer({ open, onClose, title, children, width = 640 }: DrawerProps) {
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
        // Already open
      }
    } else if (dialog.open) {
      dialog.close();
    }
  }, [open]);

  // Listen for native close event — stable listener via ref
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    const handler = () => onCloseRef.current();
    dialog.addEventListener('close', handler);
    return () => dialog.removeEventListener('close', handler);
  }, []);

  return (
    <dialog ref={dialogRef} className="modal">
      <div
        className="modal-box max-w-none p-0 flex flex-col"
        style={{ width: `${width}px`, maxWidth: '100vw' }}
      >
        <div className="flex items-center justify-between border-b border-base-300 px-6 py-4">
          {title && <h3 className="text-lg font-semibold">{title}</h3>}
          <button className="btn btn-sm btn-circle btn-ghost" onClick={() => dialogRef.current?.close()}>✕</button>
        </div>
        <div className="flex-1 overflow-y-auto px-6 py-4">{children}</div>
      </div>
      <form method="dialog" className="modal-backdrop">
        <button>close</button>
      </form>
    </dialog>
  );
}
