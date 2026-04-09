import { useEffect } from 'react';
import { motion } from 'motion/react';
import { useI18n } from '../hooks/useI18n';
import { ChecklistItem } from '../components/ChecklistItem';
import type { InstallerState } from '../hooks/useInstaller';

interface SystemCheckProps {
  state: InstallerState;
  onRunCheck: () => void;
  onInstall: (skipNode: boolean) => void;
  onSkipNodeChange: (skip: boolean) => void;
  onBack: () => void;
}

export function SystemCheck({
  state,
  onRunCheck,
  onInstall,
  onSkipNodeChange,
  onBack,
}: SystemCheckProps) {
  const { t } = useI18n();

  // Run system check on mount
  useEffect(() => {
    if (state.phase === 'idle' || state.phase === 'checking') {
      onRunCheck();
    }
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const isChecking = state.phase === 'checking';
  const isReady = state.phase === 'ready';
  const checkResult = state.systemCheck;
  const needsInstall = checkResult && checkResult.installCount > 0;

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -20 }}
      transition={{ duration: 0.3 }}
      className="flex flex-col h-full px-8 py-6"
    >
      {/* Header */}
      <h2 className="text-lg font-bold text-dark-text mb-1">{t('systemCheck.title')}</h2>
      <p className="text-xs text-dark-muted mb-4">{t('systemCheck.description')}</p>

      {/* Checklist */}
      <div className="flex-1 space-y-0.5 overflow-y-auto">
        {checkResult ? (
          checkResult.items.map((item) => (
            <ChecklistItem
              key={item.key}
              label={item.label}
              status={item.status as 'pass' | 'fail' | 'warn' | 'checking'}
              detail={item.detail}
            />
          ))
        ) : (
          // Show placeholder items while checking
          ['windows', 'internet', 'disk', 'ram', 'git', 'node', 'claude'].map((key) => (
            <ChecklistItem
              key={key}
              label={t(`systemCheck.${key}`)}
              status={isChecking ? 'checking' : 'pending'}
              detail={isChecking ? t('systemCheck.checking') : ''}
            />
          ))
        )}
      </div>

      {/* Summary */}
      {isReady && checkResult && (
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          className="mt-3"
        >
          {needsInstall ? (
            <div className="text-xs text-dark-muted bg-dark-surface rounded-lg p-3 border border-dark-border">
              <p>
                {t('systemCheck.summary', {
                  count: String(checkResult.installCount),
                  size: String(checkResult.approxDownloadMb),
                })}
              </p>

              {/* Skip Node.js option */}
              <label className="flex items-center gap-2 mt-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={state.skipNode}
                  onChange={(e) => onSkipNodeChange(e.target.checked)}
                  className="rounded border-dark-border accent-primary"
                />
                <span className="text-xs text-dark-muted">
                  {t('systemCheck.skip_node')}
                </span>
              </label>
            </div>
          ) : (
            <div className="text-xs text-success bg-success/10 rounded-lg p-3 border border-success/20">
              ✅ {t('systemCheck.all_good')}
            </div>
          )}
        </motion.div>
      )}

      {/* Error state */}
      {state.phase === 'error' && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          className="mt-3 text-xs text-error bg-error/10 rounded-lg p-3 border border-error/20"
        >
          {state.logs[state.logs.length - 1]?.message || 'System check failed'}
        </motion.div>
      )}

      {/* Buttons */}
      <div className="flex gap-2 mt-4">
        <button
          onClick={onBack}
          className="px-4 py-2 text-xs rounded-lg bg-dark-border hover:bg-dark-border/80 text-dark-text transition-colors"
        >
          {t('common.back')}
        </button>
        <div className="flex-1" />
        {isReady && (
          <motion.button
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            whileHover={{ scale: 1.02 }}
            whileTap={{ scale: 0.98 }}
            onClick={() => onInstall(state.skipNode)}
            className="px-6 py-2 text-xs rounded-lg bg-gradient-to-r from-primary to-amber-500 text-white font-semibold shadow-lg shadow-primary/25 hover:shadow-primary/40 transition-shadow"
          >
            {needsInstall ? t('systemCheck.install_btn') : t('systemCheck.continue_btn')}
          </motion.button>
        )}
        {state.phase === 'error' && (
          <button
            onClick={onRunCheck}
            className="px-4 py-2 text-xs rounded-lg bg-primary hover:bg-primary-hover text-white transition-colors"
          >
            Retry
          </button>
        )}
      </div>
    </motion.div>
  );
}
