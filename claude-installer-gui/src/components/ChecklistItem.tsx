import { motion } from 'motion/react';

interface ChecklistItemProps {
  label: string;
  status: 'pending' | 'checking' | 'pass' | 'fail' | 'warn' | 'skipped';
  detail: string;
}

export function ChecklistItem({ label, status, detail }: ChecklistItemProps) {
  return (
    <motion.div
      layout
      initial={{ opacity: 0, x: -10 }}
      animate={{ opacity: 1, x: 0 }}
      className="flex items-center gap-3 py-1.5"
    >
      {/* Status icon */}
      <div className="w-5 h-5 flex items-center justify-center shrink-0">
        {status === 'pending' && (
          <span className="text-dark-muted text-sm">☐</span>
        )}
        {status === 'checking' && (
          <svg className="w-4 h-4 text-primary spinner" viewBox="0 0 24 24" fill="none">
            <circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="3" opacity="0.2" />
            <path
              d="M12 2a10 10 0 019.75 7.75"
              stroke="currentColor"
              strokeWidth="3"
              strokeLinecap="round"
            />
          </svg>
        )}
        {status === 'pass' && (
          <motion.span
            initial={{ scale: 0 }}
            animate={{ scale: 1 }}
            className="text-success text-sm"
          >
            ✅
          </motion.span>
        )}
        {status === 'fail' && (
          <motion.span
            initial={{ scale: 0 }}
            animate={{ scale: 1 }}
            className="text-error text-sm"
          >
            ❌
          </motion.span>
        )}
        {status === 'warn' && (
          <motion.span
            initial={{ scale: 0 }}
            animate={{ scale: 1 }}
            className="text-warning text-sm"
          >
            ⚠️
          </motion.span>
        )}
        {status === 'skipped' && (
          <span className="text-dark-muted text-sm">⏭️</span>
        )}
      </div>

      {/* Label */}
      <span className="text-sm text-dark-text min-w-[140px]">{label}</span>

      {/* Detail / status text */}
      <span
        className={`text-xs ml-auto text-right ${
          status === 'pass'
            ? 'text-success'
            : status === 'fail'
            ? 'text-error'
            : status === 'warn'
            ? 'text-warning'
            : 'text-dark-muted'
        }`}
      >
        {detail}
      </span>
    </motion.div>
  );
}
