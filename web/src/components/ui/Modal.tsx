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
  const isClosingRef = useRef(false);

  // Sync dialog open state with React prop
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;

    // Prevent effect from running during our own close operation
    if (isClosingRef.current) return;

    if (open) {
      try {
        dialog.showModal();
      } catch {
        // Already open
      }
    } else if (dialog.open) {
      // Use requestAnimationFrame to ensure smooth close
      requestAnimationFrame(() => {
        if (dialog.open) {
          dialog.close();
        }
      });
    }
  }, [open]);

  return (
    <dialog
      ref={dialogRef}
      className={cn('modal', className)}
      onClose={(_e) => {
        // Prevent double-close: mark as closing before calling onClose
        isClosingRef.current = true;
        onClose();
        // Reset after a frame to allow the effect to settle
        requestAnimationFrame(() => {
          setTimeout(() => {
            isClosingRef.current = false;
          }, 0);
        });
      }}
    >
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
