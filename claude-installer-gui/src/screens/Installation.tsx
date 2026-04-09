import { useState } from 'react';
import { motion } from 'motion/react';
import { useI18n } from '../hooks/useI18n';
import { ProgressBar } from '../components/ProgressBar';
import { LogViewer } from '../components/LogViewer';
import { ConfirmDialog } from '../components/ConfirmDialog';
import type { InstallerState } from '../hooks/useInstaller';

interface InstallationProps {
  state: InstallerState;
  onCancel: () => void;
}

// Human-readable step names
const stepNames: Record<string, string> = {
  git: 'Git for Windows',
  node: 'Node.js',
  claude: 'Claude Code',
};

/** Format bytes to human-readable string */
function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${bytes} B`;
}

/** Format seconds to human-readable ETA */
function formatEta(secs: number): string {
  if (secs < 60) return `${secs}s`;
  const mins = Math.floor(secs / 60);
  const remainSecs = secs % 60;
  return `${mins}m ${remainSecs}s`;
}

export function Installation({ state, onCancel }: InstallationProps) {
  const { t } = useI18n();
  const [showLog, setShowLog] = useState(false);
  const [showCancelDialog, setShowCancelDialog] = useState(false);

  const currentName = state.currentStep ? (stepNames[state.currentStep] || state.currentStep) : '';
  const dl = state.downloadProgress;

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -20 }}
      transition={{ duration: 0.3 }}
      className="flex flex-col h-full px-8 py-6"
    >
      {/* Title */}
      <h2 className="text-lg font-bold text-dark-text mb-1">{t('installation.title')}</h2>
      <p className="text-xs text-dark-muted mb-4">
        {t('installation.overall_progress', {
          percent: String(Math.round(state.overallPercent)),
          message: state.currentStep ? `Installing ${currentName}...` : 'Preparing...',
        })}
      </p>

      {/* Overall progress bar */}
      <ProgressBar percent={state.overallPercent} showLabel />

      {/* Current step detail */}
      <div className="mt-5 bg-dark-surface rounded-lg border border-dark-border p-4">
        {/* Step indicator */}
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-primary animate-pulse" />
            <span className="text-sm font-medium text-dark-text">
              {currentName || 'Preparing...'}
            </span>
          </div>
          <span className="text-xs text-dark-muted">
            Step {state.currentStepIndex}/{state.totalSteps}
          </span>
        </div>

        {/* Download progress (if downloading) */}
        {dl && dl.total > 0 && (
          <div className="space-y-2">
            <ProgressBar
              percent={(dl.downloaded / dl.total) * 100}
              variant="primary"
              height={6}
            />
            <div className="flex justify-between text-[11px] text-dark-muted">
              <span>
                {formatBytes(dl.downloaded)} / {formatBytes(dl.total)}
              </span>
              <span className="flex gap-3">
                <span>{formatBytes(dl.speedBps)}/s</span>
                {dl.etaSecs > 0 && (
                  <span>{t('installation.eta', { time: formatEta(dl.etaSecs) })}</span>
                )}
              </span>
            </div>
          </div>
        )}

        {/* Latest log message */}
        {state.logs.length > 0 && (
          <p className="text-[11px] text-dark-muted mt-2 truncate">
            {state.logs[state.logs.length - 1].message}
          </p>
        )}
      </div>

      {/* Step progress dots */}
      <div className="flex items-center justify-center gap-2 mt-4">
        {state.installQueue.map((step, i) => {
          const result = state.results.find((r) => r.step === step);
          const isCurrent = state.currentStep === step;
          return (
            <div key={step} className="flex items-center gap-1">
              <div
                className={`w-6 h-6 rounded-full flex items-center justify-center text-[10px] font-medium ${
                  result?.success
                    ? 'bg-success text-white'
                    : result && !result.success
                    ? 'bg-error text-white'
                    : isCurrent
                    ? 'bg-primary text-white'
                    : 'bg-dark-border text-dark-muted'
                }`}
              >
                {result?.success ? '✓' : result && !result.success ? '✗' : i + 1}
              </div>
              {i < state.installQueue.length - 1 && (
                <div className="w-8 h-0.5 bg-dark-border" />
              )}
            </div>
          );
        })}
      </div>

      {/* Collapsible log viewer */}
      <div className="mt-4 flex-1 min-h-0">
        <button
          onClick={() => setShowLog(!showLog)}
          className="text-[11px] text-dark-muted hover:text-primary transition-colors mb-1"
        >
          {showLog ? t('installation.hide_log') : t('installation.show_log')} ▾
        </button>
        {showLog && <LogViewer logs={state.logs} maxHeight={120} />}
      </div>

      {/* Cancel button */}
      <div className="flex justify-end mt-3">
        <button
          onClick={() => setShowCancelDialog(true)}
          className="px-4 py-2 text-xs rounded-lg bg-dark-border hover:bg-error/20 hover:text-error text-dark-muted transition-colors"
        >
          {t('installation.cancel')}
        </button>
      </div>

      {/* Cancel confirmation dialog */}
      <ConfirmDialog
        open={showCancelDialog}
        title={t('installation.cancel_confirm_title')}
        message={t('installation.cancel_confirm_message')}
        confirmLabel={t('installation.cancel_confirm_yes')}
        cancelLabel={t('installation.cancel_confirm_no')}
        onConfirm={() => {
          setShowCancelDialog(false);
          onCancel();
        }}
        onCancel={() => setShowCancelDialog(false)}
        variant="danger"
      />
    </motion.div>
  );
}
