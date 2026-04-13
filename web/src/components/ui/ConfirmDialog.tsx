import { useState } from 'react';
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

  const handleConfirm = () => {
    onConfirm();
    setOpen(false);
  };

  return (
    <>
      <div onClick={() => setOpen(true)} className="inline-block cursor-pointer">
        {children}
      </div>

      <dialog className={cn(open && 'modal modal-open')}>
        <div className="modal-box">
          <h3 className="text-lg font-semibold mb-4">{title}</h3>
          <div className="modal-action">
            <Button variant="ghost" size="sm" onClick={() => setOpen(false)}>
              {cancelText}
            </Button>
            <Button variant="danger" size="sm" onClick={handleConfirm}>
              {okText}
            </Button>
          </div>
        </div>
        <form method="dialog" className="modal-backdrop">
          <button onClick={() => setOpen(false)}>close</button>
        </form>
      </dialog>
    </>
  );
}

function cn(...classes: (string | boolean | undefined | false)[]) {
  return classes.filter(Boolean).join(' ');
}
