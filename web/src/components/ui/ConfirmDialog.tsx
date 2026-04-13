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
      {/* Trigger */}
      <div onClick={() => setOpen(true)} className="inline-block cursor-pointer">
        {children}
      </div>

      {/* Dialog */}
      {open && (
        <div className="fixed inset-0 z-[200] flex items-center justify-center">
          {/* Backdrop */}
          <div
            className="absolute inset-0 bg-black/60 backdrop-blur-sm"
            onClick={() => setOpen(false)}
          />

          {/* Panel */}
          <div className="relative z-10 w-full max-w-xs rounded-xl border border-[#1e1e1e] bg-[#111111] p-5 animate-fade-in-up">
            <h3 className="mb-4 text-sm font-semibold text-[#ededed]">{title}</h3>

            <div className="flex items-center justify-end gap-2">
              <Button variant="ghost" size="sm" onClick={() => setOpen(false)}>
                {cancelText}
              </Button>
              <Button variant="danger" size="sm" onClick={handleConfirm}>
                {okText}
              </Button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
