import { useState, useEffect, useCallback } from 'react';
import { motion } from 'motion/react';
import { invoke, Channel } from '@tauri-apps/api/core';
import { useI18n } from '../hooks/useI18n';
import { ProgressBar } from '../components/ProgressBar';
import { LogViewer } from '../components/LogViewer';
import { ConfirmDialog } from '../components/ConfirmDialog';
import type { LogEntry, InstallEvent } from '../hooks/useInstaller';

interface InstalledComponent {
  key: string;
  label: string;
  version: string | null;
  installed: boolean;
  warning: string | null;
}

interface StepResult {
  step: string;
  success: boolean;
}

type UninstallPhase = 'select' | 'progress' | 'done';

interface UninstallProps {
  onBack: () => void;
}

export function Uninstall({ onBack }: UninstallProps) {
  const { t } = useI18n();
  const [phase, setPhase] = useState<UninstallPhase>('select');
  const [components, setComponents] = useState<InstalledComponent[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set(['claude']));
  const [includeSettings, setIncludeSettings] = useState(false);
  const [includeNpmCache, setIncludeNpmCache] = useState(false);
  const [showConfirm, setShowConfirm] = useState(false);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [progress, setProgress] = useState(0);
  const [results, setResults] = useState<StepResult[]>([]);
  const [loading, setLoading] = useState(true);

  // Detect installed components on mount
  useEffect(() => {
    async function detect() {
      try {
        const result = await invoke<InstalledComponent[]>('detect_installed');
        setComponents(result);
      } catch (e) {
        console.error('Failed to detect installed components:', e);
      } finally {
        setLoading(false);
      }
    }
    detect();
  }, []);

  const toggleComponent = (key: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const selectedLabels = components
    .filter((c) => selected.has(c.key))
    .map((c) => c.label)
    .join(', ');

  const runUninstall = useCallback(async () => {
    setShowConfirm(false);
    setPhase('progress');
    setLogs([]);
    setResults([]);
    setProgress(0);

    const items = Array.from(selected);

    const onEvent = new Channel<InstallEvent>();
    onEvent.onmessage = (event: InstallEvent) => {
      const now = new Date().toLocaleTimeString();
      switch (event.event) {
        case 'stepStarted':
          setProgress(((event.data.currentStep - 1) / event.data.totalSteps) * 100);
          break;
        case 'stepLog':
          setLogs((prev) => [...prev, { timestamp: now, level: event.data.level as 'info' | 'warn' | 'error', message: event.data.message }]);
          break;
        case 'stepCompleted':
          setResults((prev) => [...prev, { step: event.data.step, success: event.data.success }]);
          break;
      }
    };

    try {
      await invoke('uninstall_components', {
        components: items,
        includeSettings,
        includeNpmCache,
        onEvent,
      });
    } catch (e) {
      setLogs((prev) => [
        ...prev,
        { timestamp: new Date().toLocaleTimeString(), level: 'error', message: `Uninstall error: ${e}` },
      ]);
    }

    setProgress(100);
    setPhase('done');
  }, [selected, includeSettings, includeNpmCache]);

  // ── SELECT PHASE ──
  if (phase === 'select') {
    return (
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        exit={{ opacity: 0, y: -20 }}
        className="flex flex-col h-full px-8 py-6"
      >
        <h2 className="text-lg font-bold text-dark-text mb-1">{t('uninstall.title')}</h2>
        <p className="text-xs text-dark-muted mb-4">{t('uninstall.description')}</p>

        {loading ? (
          <div className="flex-1 flex items-center justify-center">
            <div className="w-6 h-6 border-2 border-primary border-t-transparent rounded-full spinner" />
          </div>
        ) : (
          <div className="flex-1 space-y-2">
            {components.map((comp) => (
              <div key={comp.key} className="bg-dark-surface rounded-lg border border-dark-border p-3">
                <label className={`flex items-center gap-3 ${!comp.installed ? 'opacity-50' : 'cursor-pointer'}`}>
                  <input
                    type="checkbox"
                    checked={selected.has(comp.key)}
                    onChange={() => toggleComponent(comp.key)}
                    disabled={!comp.installed}
                    className="rounded border-dark-border accent-primary"
                  />
                  <div className="flex-1">
                    <div className="flex items-center gap-2">
                      <span className="text-sm text-dark-text">{comp.label}</span>
                      {comp.version && (
                        <span className="text-[10px] text-dark-muted bg-dark-border rounded px-1.5 py-0.5">
                          {comp.version}
                        </span>
                      )}
                      {!comp.installed && (
                        <span className="text-[10px] text-dark-muted">{t('uninstall.not_found')}</span>
                      )}
                    </div>
                    {comp.warning && comp.installed && (
                      <p className="text-[11px] text-warning mt-0.5">⚠️ {comp.warning}</p>
                    )}
                  </div>
                </label>

                {/* Sub-options */}
                {comp.key === 'claude' && selected.has('claude') && (
                  <label className="flex items-center gap-2 ml-7 mt-2 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={includeSettings}
                      onChange={(e) => setIncludeSettings(e.target.checked)}
                      className="rounded border-dark-border accent-primary"
                    />
                    <span className="text-[11px] text-dark-muted">{t('uninstall.include_settings')}</span>
                  </label>
                )}
                {comp.key === 'node' && selected.has('node') && (
                  <label className="flex items-center gap-2 ml-7 mt-2 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={includeNpmCache}
                      onChange={(e) => setIncludeNpmCache(e.target.checked)}
                      className="rounded border-dark-border accent-primary"
                    />
                    <span className="text-[11px] text-dark-muted">{t('uninstall.include_npm')}</span>
                  </label>
                )}
              </div>
            ))}
          </div>
        )}

        <div className="flex gap-2 mt-4">
          <button
            onClick={onBack}
            className="px-4 py-2 text-xs rounded-lg bg-dark-border hover:bg-dark-border/80 text-dark-text transition-colors"
          >
            {t('uninstall.back')}
          </button>
          <div className="flex-1" />
          <button
            onClick={() => setShowConfirm(true)}
            disabled={selected.size === 0}
            className="px-5 py-2 text-xs rounded-lg bg-error hover:bg-error/80 text-white font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
          >
            {t('uninstall.uninstall_btn')}
          </button>
        </div>

        <ConfirmDialog
          open={showConfirm}
          title={t('uninstall.confirm_title')}
          message={t('uninstall.confirm_message', { items: selectedLabels })}
          confirmLabel={t('uninstall.confirm_yes')}
          cancelLabel={t('uninstall.confirm_no')}
          onConfirm={runUninstall}
          onCancel={() => setShowConfirm(false)}
          variant="danger"
        />
      </motion.div>
    );
  }

  // ── PROGRESS PHASE ──
  if (phase === 'progress') {
    return (
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        className="flex flex-col h-full px-8 py-6"
      >
        <h2 className="text-lg font-bold text-dark-text mb-4">{t('uninstall.progress_title')}</h2>
        <ProgressBar percent={progress} variant="primary" showLabel />
        <div className="mt-4 flex-1 min-h-0">
          <LogViewer logs={logs} maxHeight={240} />
        </div>
      </motion.div>
    );
  }

  // ── DONE PHASE ──
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className="flex flex-col h-full px-8 py-6"
    >
      <h2 className="text-lg font-bold text-dark-text mb-4">{t('uninstall.done_title')}</h2>

      <div className="space-y-2 mb-4">
        {components.map((comp) => {
          const result = results.find((r) => r.step === comp.key);
          const wasSelected = selected.has(comp.key);
          return (
            <div key={comp.key} className="flex items-center gap-2 text-sm">
              {result?.success ? (
                <span className="text-success">✅</span>
              ) : wasSelected && result ? (
                <span className="text-error">❌</span>
              ) : (
                <span className="text-dark-muted">⏭️</span>
              )}
              <span className="text-dark-text">{comp.label}</span>
              <span className="text-xs text-dark-muted ml-auto">
                {result?.success
                  ? t('uninstall.removed')
                  : !wasSelected
                  ? t('uninstall.skipped')
                  : 'Failed'}
              </span>
            </div>
          );
        })}
      </div>

      <div className="text-xs text-dark-muted bg-dark-surface rounded-lg p-3 border border-dark-border">
        💡 {t('uninstall.done_message')}
      </div>

      <div className="flex-1" />

      <div className="flex justify-end mt-4">
        <button
          onClick={onBack}
          className="px-5 py-2 text-xs rounded-lg bg-primary hover:bg-primary-hover text-white font-medium transition-colors"
        >
          {t('common.close')}
        </button>
      </div>
    </motion.div>
  );
}
