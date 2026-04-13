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
  const isClosingRef = useRef(false);

  // Sync dialog open state with React prop
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;

    if (isClosingRef.current) return;

    if (open) {
      try {
        dialog.showModal();
      } catch {
        // Already open
      }
    } else if (dialog.open) {
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
      className="modal"
      onClose={(_e) => {
        isClosingRef.current = true;
        onClose();
        requestAnimationFrame(() => {
          setTimeout(() => {
            isClosingRef.current = false;
          }, 0);
        });
      }}
    >
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