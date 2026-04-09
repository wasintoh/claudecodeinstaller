import { motion } from 'motion/react';

interface ProgressBarProps {
  /** Progress percentage (0-100) */
  percent: number;
  /** Visual variant */
  variant?: 'primary' | 'success' | 'warning';
  /** Show percentage text */
  showLabel?: boolean;
  /** Height in pixels */
  height?: number;
}

const variantColors = {
  primary: 'from-primary to-amber-400',
  success: 'from-success to-emerald-400',
  warning: 'from-warning to-yellow-400',
};

export function ProgressBar({
  percent,
  variant = 'primary',
  showLabel = false,
  height = 8,
}: ProgressBarProps) {
  const clamped = Math.min(100, Math.max(0, percent));

  return (
    <div className="w-full">
      <div
        className="w-full bg-dark-border rounded-full overflow-hidden"
        style={{ height: `${height}px` }}
      >
        <motion.div
          className={`h-full rounded-full bg-gradient-to-r ${variantColors[variant]}`}
          initial={{ width: 0 }}
          animate={{ width: `${clamped}%` }}
          transition={{ type: 'spring', stiffness: 100, damping: 20 }}
        />
      </div>
      {showLabel && (
        <div className="text-xs text-dark-muted mt-1 text-right">
          {Math.round(clamped)}%
        </div>
      )}
    </div>
  );
}
