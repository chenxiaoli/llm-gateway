import { useEffect } from 'react';
import { createPortal } from 'react-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { cn } from '../../lib/cn';

export interface ModalProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  children: React.ReactNode;
  footer?: React.ReactNode;
  className?: string;
  width?: number;
}

export function Modal({ open, onClose, title, children, footer, className, width = 480 }: ModalProps) {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && open) onClose();
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [open, onClose]);

  useEffect(() => {
    if (open) {
      document.body.style.overflow = 'hidden';
    } else {
      document.body.style.overflow = '';
    }
    return () => {
      document.body.style.overflow = '';
    };
  }, [open]);

  return createPortal(
    <AnimatePresence mode="wait">
      {open ? (
        <motion.div
          key="modal-root"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.2 }}
          className="fixed inset-0 z-[120] flex items-center justify-center pointer-events-none p-4"
        >
          {/* Backdrop */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.2 }}
            className="absolute inset-0 bg-black/60"
            onClick={onClose}
          />

          {/* Panel */}
          <motion.div
            initial={{ scale: 0.96, opacity: 0, y: 8 }}
            animate={{ scale: 1, opacity: 1, y: 0 }}
            exit={{ scale: 0.96, opacity: 0, y: 8 }}
            transition={{ type: 'spring', damping: 28, stiffness: 300, mass: 0.8 }}
            className={cn(
              'relative bg-base-100 rounded-xl border border-base-300 flex flex-col overflow-hidden pointer-events-auto',
              className,
            )}
            style={{ width: `${width}px`, maxWidth: '100%', maxHeight: 'calc(100vh - 32px)' }}
          >
            {/* Header */}
            {title && (
              <>
                <div className="px-6 pt-5 pb-4 shrink-0">
                  <div className="flex items-start justify-between gap-4">
                    <h2 className="text-[15px] font-semibold text-base-content leading-tight pt-1">{title}</h2>
                    <motion.button
                      initial={{ scale: 0.8, opacity: 0 }}
                      animate={{ scale: 1, opacity: 1 }}
                      transition={{ delay: 0.1, duration: 0.2 }}
                      whileHover={{ scale: 1.05 }}
                      whileTap={{ scale: 0.95 }}
                      onClick={onClose}
                      className="w-8 h-8 rounded-lg flex items-center justify-center shrink-0 text-base-content/40 hover:text-base-content hover:bg-base-200/60 transition-all duration-150 cursor-pointer -mr-1 -mt-1"
                      aria-label="Close"
                    >
                      <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
                        <path d="M1 1l12 12M13 1L1 13" />
                      </svg>
                    </motion.button>
                  </div>
                </div>
                <div className="h-px mx-6" style={{ background: 'oklch(var(--b3) / 0.6)' }} />
              </>
            )}

            {/* Content */}
            <div className="flex-1 overflow-y-auto px-6 py-5">
              {children}
            </div>

            {/* Footer */}
            {footer && (
              <>
                <div className="h-px mx-6" style={{ background: 'oklch(var(--b3) / 0.6)' }} />
                <div className="px-6 py-4 shrink-0">
                  <div className="modal-action">{footer}</div>
                </div>
              </>
            )}
          </motion.div>
        </motion.div>
      ) : null}
    </AnimatePresence>,
    document.body,
  );
}
