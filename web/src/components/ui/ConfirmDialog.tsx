import { useState, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { Button } from './Button';
import { useTranslation } from 'react-i18next';

export interface ConfirmDialogProps {
  title: string;
  onConfirm: () => void;
  children?: React.ReactNode;
  okText?: string;
  cancelText?: string;
  variant?: 'danger' | 'default';
  /** Controlled open state. When provided, the dialog won't render its trigger wrapper. */
  open?: boolean;
  /** Called when the user cancels or dismisses (controlled mode only). */
  onCancel?: () => void;
}

export function ConfirmDialog({
  title,
  onConfirm,
  children,
  okText,
  cancelText,
  variant = 'default',
  open: controlledOpen,
  onCancel,
}: ConfirmDialogProps) {
  const { t } = useTranslation();
  const resolvedOkText = okText ?? t('common.confirm');
  const resolvedCancelText = cancelText ?? t('common.cancel');
  const [internalOpen, setInternalOpen] = useState(false);
  const isControlled = controlledOpen !== undefined;
  const open = isControlled ? controlledOpen : internalOpen;
  const close = () => (isControlled ? onCancel?.() : setInternalOpen(false));

  useEffect(() => {
    if (!open) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') close();
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  const handleConfirm = () => {
    onConfirm();
    if (!isControlled) setInternalOpen(false);
  };

  return (
    <>
      {!isControlled && (
        <div onClick={() => setInternalOpen(true)} className="inline-block cursor-pointer">
          {children}
        </div>
      )}

      {createPortal(
        <AnimatePresence>
          {open && (
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              transition={{ duration: 0.15 }}
              className="fixed inset-0 z-[200] flex items-center justify-center p-4"
            >
              {/* Backdrop */}
              <motion.div
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                transition={{ duration: 0.15 }}
                className="absolute inset-0 bg-black/60"
                onClick={close}
              />

              {/* Panel */}
              <motion.div
                initial={{ scale: 0.96, opacity: 0, y: 8 }}
                animate={{ scale: 1, opacity: 1, y: 0 }}
                exit={{ scale: 0.96, opacity: 0, y: 8 }}
                transition={{ type: 'spring', damping: 28, stiffness: 300, mass: 0.8 }}
                className="relative bg-base-100 rounded-xl border border-base-300 p-6 w-full pointer-events-auto"
                style={{ maxWidth: 400 }}
                onClick={(e) => e.stopPropagation()}
              >
                <h3 className="text-[15px] font-semibold text-base-content">{title}</h3>
                {isControlled && children ? (
                  <p className="text-sm text-base-content/60 mt-2 mb-6">{children}</p>
                ) : (
                  <div className="mb-6" />
                )}
                <div className="flex justify-end gap-2">
                  <Button variant="ghost" size="sm" onClick={close}>
                    {resolvedCancelText}
                  </Button>
                  <Button variant={variant === 'danger' ? 'danger' : undefined} size="sm" onClick={handleConfirm}>
                    {resolvedOkText}
                  </Button>
                </div>
              </motion.div>
            </motion.div>
          )}
        </AnimatePresence>,
        document.body,
      )}
    </>
  );
}
