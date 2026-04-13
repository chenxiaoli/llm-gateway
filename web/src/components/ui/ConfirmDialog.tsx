import { useState, useEffect, useCallback, useRef } from 'react';
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
    const handler = () => setOpen(false);
    dialog.addEventListener('close', handler);
    return () => dialog.removeEventListener('close', handler);
  }, []);

  const handleConfirm = () => {
    onConfirm();
    close();
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
            <Button variant="ghost" size="sm" onClick={close}>
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
