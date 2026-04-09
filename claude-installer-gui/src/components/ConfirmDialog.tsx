import { motion, AnimatePresence } from 'motion/react';

interface ConfirmDialogProps {
  open: boolean;
  title: string;
  message: string;
  confirmLabel: string;
  cancelLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
  variant?: 'danger' | 'default';
}

export function ConfirmDialog({
  open,
  title,
  message,
  confirmLabel,
  cancelLabel,
  onConfirm,
  onCancel,
  variant = 'default',
}: ConfirmDialogProps) {
  return (
    <AnimatePresence>
      {open && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
          onClick={onCancel}
        >
          <motion.div
            initial={{ scale: 0.9, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }}
            exit={{ scale: 0.9, opacity: 0 }}
            transition={{ type: 'spring', stiffness: 300, damping: 25 }}
            onClick={(e) => e.stopPropagation()}
            className="bg-dark-surface border border-dark-border rounded-xl p-5 w-[340px] shadow-2xl"
          >
            <h3 className="text-sm font-semibold text-dark-text mb-2">{title}</h3>
            <p className="text-xs text-dark-muted mb-5 leading-relaxed">{message}</p>

            <div className="flex gap-2 justify-end">
              <button
                onClick={onCancel}
                className="px-4 py-1.5 text-xs rounded-lg bg-dark-border hover:bg-dark-border/80 text-dark-text transition-colors"
              >
                {cancelLabel}
              </button>
              <button
                onClick={onConfirm}
                className={`px-4 py-1.5 text-xs rounded-lg font-medium transition-colors ${
                  variant === 'danger'
                    ? 'bg-error hover:bg-error/80 text-white'
                    : 'bg-primary hover:bg-primary-hover text-white'
                }`}
              >
                {confirmLabel}
              </button>
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
