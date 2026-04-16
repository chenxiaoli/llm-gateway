import { useState, useEffect, useRef } from 'react';
import { Button } from './Button';

export interface ConfirmDialogProps {
  title: string;
  onConfirm: () => void;
  children: React.ReactNode;
  okText?: string;
  cancelText?: string;
}

export function ConfirmDialog({ title, onConfirm, children, okText = 'Confirm', cancelText = 'Cancel' }: ConfirmDialogProps) {
  const [open, setOpen] = useState(false);
  const dialogRef = useRef<HTMLDialogElement>(null);

  // Sync dialog open/close state with internal state
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

  // Listen for native close event — stable listener
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    const handler = () => setOpen(false);
    dialog.addEventListener('close', handler);
    return () => dialog.removeEventListener('close', handler);
  }, []);

  const handleConfirm = () => {
    onConfirm();
    dialogRef.current?.close();
  };

  return (
    <>
      <div onClick={() => setOpen(true)} className="inline-block cursor-pointer">
        {children}
      </div>

      <dialog ref={dialogRef} className="modal">
        <div className="modal-box">
          <h3 className="text-lg font-semibold mb-4">{title}</h3>
          <div className="modal-action">
            <Button variant="ghost" size="sm" onClick={() => dialogRef.current?.close()}>
              {cancelText}
            </Button>
            <Button variant="danger" size="sm" onClick={handleConfirm}>
              {okText}
            </Button>
          </div>
        </div>
        <form method="dialog" className="modal-backdrop">
          <button>close</button>
        </form>
      </dialog>
    </>
  );
}
