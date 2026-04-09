import { useMemo, useState } from 'react';
import { motion } from 'motion/react';
import { useI18n } from '../hooks/useI18n';
import type { InstallerState } from '../hooks/useInstaller';

interface CompletionProps {
  state: InstallerState;
  onRetry: () => void;
  onExportLog: () => Promise<string | null>;
  onOpenTerminal: () => void;
  onRelaunchClaude: () => void;
  onClose: () => void;
}

/** Error details with troubleshooting info */
const errorHelp: Record<string, { cause: string; fix: string; link: string }> = {
  git: {
    cause: 'Network issue or permission error during Git installation',
    fix: 'Download and install Git manually from the link below',
    link: 'https://git-scm.com/download/win',
  },
  node: {
    cause: 'MSI installer failed or requires administrator privileges',
    fix: 'Download and install Node.js LTS manually from the link below',
    link: 'https://nodejs.org/en/download/',
  },
  claude: {
    cause: 'npm installation failed or network timeout',
    fix: 'Open a terminal and run: npm install -g @anthropic-ai/claude-code',
    link: 'https://claude.ai/download',
  },
};

/** Confetti effect pieces */
function Confetti() {
  const pieces = useMemo(
    () =>
      Array.from({ length: 40 }, (_, i) => ({
        id: i,
        left: `${Math.random() * 100}%`,
        delay: Math.random() * 2,
        color: ['#D97706', '#22C55E', '#3B82F6', '#EAB308', '#EC4899'][
          Math.floor(Math.random() * 5)
        ],
        size: 4 + Math.random() * 6,
        rotation: Math.random() * 360,
      })),
    []
  );

  return (
    <div className="fixed inset-0 pointer-events-none overflow-hidden z-40">
      {pieces.map((p) => (
        <div
          key={p.id}
          className="confetti-piece absolute"
          style={{
            left: p.left,
            top: '-10px',
            width: `${p.size}px`,
            height: `${p.size}px`,
            backgroundColor: p.color,
            borderRadius: p.size > 7 ? '50%' : '1px',
            animationDelay: `${p.delay}s`,
            transform: `rotate(${p.rotation}deg)`,
          }}
        />
      ))}
    </div>
  );
}

export function Completion({
  state,
  onRetry,
  onExportLog,
  onOpenTerminal,
  onRelaunchClaude,
  onClose,
}: CompletionProps) {
  const { t } = useI18n();
  const [logPath, setLogPath] = useState<string | null>(null);

  const allSuccess = state.results.every((r) => r.success);
  const failedSteps = state.results.filter((r) => !r.success);
  const successSteps = state.results.filter((r) => r.success);

  // Post-install sub-phase state (testing → launching → launched)
  const { postInstallPhase, testResult } = state;
  const isTesting = postInstallPhase === 'testing';
  const isLaunching = postInstallPhase === 'launching';
  const isLaunched = postInstallPhase === 'launched';
  const isTestFailed = postInstallPhase === 'testFailed';

  const handleExport = async () => {
    const path = await onExportLog();
    if (path) setLogPath(path);
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -20 }}
      transition={{ duration: 0.3 }}
      className="flex flex-col h-full px-8 py-6 overflow-y-auto"
    >
      {/* Confetti on full success */}
      {allSuccess && <Confetti />}

      {/* Success / Partial header */}
      <div className="flex flex-col items-center mb-4">
        {allSuccess ? (
          <>
            <motion.div
              initial={{ scale: 0 }}
              animate={{ scale: 1 }}
              transition={{ type: 'spring', stiffness: 200, damping: 10, delay: 0.2 }}
              className="w-16 h-16 rounded-full bg-success/20 flex items-center justify-center mb-3"
            >
              <span className="text-3xl">🎉</span>
            </motion.div>
            <h2 className="text-lg font-bold text-dark-text">{t('completion.success_title')}</h2>
            <p className="text-xs text-dark-muted mt-1 text-center">
              {t('completion.success_message')}
            </p>
          </>
        ) : (
          <>
            <motion.div
              initial={{ scale: 0 }}
              animate={{ scale: 1 }}
              className="w-16 h-16 rounded-full bg-warning/20 flex items-center justify-center mb-3"
            >
              <span className="text-3xl">⚠️</span>
            </motion.div>
            <h2 className="text-lg font-bold text-dark-text">{t('completion.partial_title')}</h2>
            <p className="text-xs text-dark-muted mt-1 text-center">
              {t('completion.partial_message')}
            </p>
          </>
        )}
      </div>

      {/* Results summary */}
      <div className="bg-dark-surface rounded-lg border border-dark-border p-3 space-y-2">
        {successSteps.map((r) => (
          <div key={r.step} className="flex items-center gap-2 text-xs">
            <span className="text-success">✅</span>
            <span className="text-dark-text">{r.step}</span>
            <span className="text-dark-muted ml-auto">
              {r.version || t('completion.installed')}
            </span>
          </div>
        ))}
        {failedSteps.map((r) => (
          <div key={r.step} className="text-xs">
            <div className="flex items-center gap-2">
              <span className="text-error">❌</span>
              <span className="text-dark-text">{r.step}</span>
              <span className="text-error ml-auto">{t('completion.failed')}</span>
            </div>
            {/* Error details */}
            {errorHelp[r.step] && (
              <div className="ml-7 mt-1 space-y-0.5 text-[11px]">
                <p className="text-dark-muted">
                  <span className="text-warning">{t('completion.probable_cause')}:</span>{' '}
                  {errorHelp[r.step].cause}
                </p>
                <p className="text-dark-muted">
                  <span className="text-primary">{t('completion.fix_instructions')}:</span>{' '}
                  {errorHelp[r.step].fix}
                </p>
                <p>
                  <a
                    href={errorHelp[r.step].link}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-primary hover:underline"
                  >
                    {t('completion.download_link')} →
                  </a>
                </p>
              </div>
            )}
            {r.error && (
              <p className="ml-7 mt-1 text-[10px] text-error/70 font-mono truncate">
                {r.error}
              </p>
            )}
          </div>
        ))}
      </div>

      {/* Post-install test / launch status */}
      {allSuccess && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 0.4 }}
          className="mt-4 bg-dark-surface rounded-lg border border-dark-border p-3"
        >
          <h3 className="text-xs font-semibold text-dark-text mb-2">
            {t('postInstall.section_title')}
          </h3>

          {/* Testing phase */}
          {isTesting && (
            <div className="flex items-center gap-2 text-xs text-dark-muted">
              <span className="inline-block w-3 h-3 rounded-full bg-primary animate-pulse" />
              <span>{t('postInstall.testing_message')}</span>
            </div>
          )}

          {/* Launching phase */}
          {isLaunching && (
            <div className="flex items-center gap-2 text-xs text-dark-muted">
              <span className="inline-block w-3 h-3 rounded-full bg-primary animate-pulse" />
              <span>{t('postInstall.launching_message')}</span>
            </div>
          )}

          {/* Launched (success) */}
          {isLaunched && (
            <div className="space-y-2">
              <div className="flex items-center gap-2 text-xs">
                <span className="text-success">✅</span>
                <span className="text-dark-text">
                  {t('postInstall.test_success', {
                    version: testResult?.version || '',
                  })}
                </span>
              </div>
              <div className="flex items-center gap-2 text-xs">
                <span className="text-success">✅</span>
                <span className="text-dark-text">{t('postInstall.launch_success')}</span>
              </div>
              {testResult && testResult.repairAttempts > 0 && (
                <div className="flex items-center gap-2 text-xs text-warning">
                  <span>🩹</span>
                  <span>
                    {t('postInstall.auto_repaired', {
                      count: String(testResult.repairAttempts),
                    })}
                  </span>
                </div>
              )}
            </div>
          )}

          {/* Test failed (after repair attempts) */}
          {isTestFailed && testResult && (
            <div className="space-y-2">
              <div className="flex items-center gap-2 text-xs">
                <span className="text-error">❌</span>
                <span className="text-dark-text">
                  {t('postInstall.test_failed', { error: testResult.errorKind })}
                </span>
              </div>
              <p className="text-[11px] text-dark-muted ml-5">
                {t(`postInstall.error_${testResult.errorKind}`)}
              </p>
              {testResult.rawOutput && (
                <p className="ml-5 text-[10px] text-error/70 font-mono truncate">
                  {testResult.rawOutput}
                </p>
              )}
            </div>
          )}
        </motion.div>
      )}

      {/* Next steps (shown only if not launched yet — as a fallback manual path) */}
      {allSuccess && !isLaunched && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 0.5 }}
          className="mt-4 bg-dark-surface rounded-lg border border-dark-border p-3"
        >
          <h3 className="text-xs font-semibold text-dark-text mb-2">
            {t('completion.next_steps')}
          </h3>
          <ol className="space-y-1.5 text-xs text-dark-muted list-decimal list-inside">
            <li>{t('completion.next_step_1')}</li>
            <li>{t('completion.next_step_2')}</li>
            <li>{t('completion.next_step_3')}</li>
          </ol>
        </motion.div>
      )}

      {/* Log path notification */}
      {logPath && (
        <div className="mt-2 text-[11px] text-success bg-success/10 rounded p-2 border border-success/20">
          {t('completion.log_saved', { path: logPath })}
        </div>
      )}

      {/* Spacer */}
      <div className="flex-1" />

      {/* Action buttons */}
      <div className="flex gap-2 mt-4 flex-wrap">
        {failedSteps.length > 0 && (
          <>
            <button
              onClick={onRetry}
              className="px-4 py-2 text-xs rounded-lg bg-primary hover:bg-primary-hover text-white font-medium transition-colors"
            >
              {t('completion.retry_failed')}
            </button>
            <button
              onClick={handleExport}
              className="px-4 py-2 text-xs rounded-lg bg-dark-border hover:bg-dark-border/80 text-dark-text transition-colors"
            >
              {t('completion.export_log')}
            </button>
          </>
        )}

        {/* Retry-launch button when test failed */}
        {isTestFailed && (
          <>
            <button
              onClick={onRelaunchClaude}
              className="px-4 py-2 text-xs rounded-lg bg-primary hover:bg-primary-hover text-white font-medium transition-colors"
            >
              {t('postInstall.retry_launch')}
            </button>
            <button
              onClick={handleExport}
              className="px-4 py-2 text-xs rounded-lg bg-dark-border hover:bg-dark-border/80 text-dark-text transition-colors"
            >
              {t('completion.export_log')}
            </button>
          </>
        )}

        <div className="flex-1" />

        {/* Primary action: re-open terminal if already launched, or manual fallback */}
        {allSuccess && isLaunched && (
          <motion.button
            whileHover={{ scale: 1.02 }}
            whileTap={{ scale: 0.98 }}
            onClick={onRelaunchClaude}
            className="px-5 py-2 text-xs rounded-lg bg-gradient-to-r from-primary to-amber-500 text-white font-semibold shadow-lg shadow-primary/25 transition-shadow"
          >
            {t('postInstall.open_another')}
          </motion.button>
        )}
        {allSuccess && !isLaunched && !isTesting && !isLaunching && !isTestFailed && (
          <motion.button
            whileHover={{ scale: 1.02 }}
            whileTap={{ scale: 0.98 }}
            onClick={onOpenTerminal}
            className="px-5 py-2 text-xs rounded-lg bg-gradient-to-r from-primary to-amber-500 text-white font-semibold shadow-lg shadow-primary/25 transition-shadow"
          >
            {t('completion.open_terminal')}
          </motion.button>
        )}

        <button
          onClick={onClose}
          className="px-4 py-2 text-xs rounded-lg bg-dark-border hover:bg-dark-border/80 text-dark-text transition-colors"
        >
          {t('completion.close')}
        </button>
      </div>
    </motion.div>
  );
}
