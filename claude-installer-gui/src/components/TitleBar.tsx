import { getCurrentWindow } from '@tauri-apps/api/window';
import { useI18n } from '../hooks/useI18n';

export function TitleBar() {
  const { t } = useI18n();
  const appWindow = getCurrentWindow();

  return (
    <div
      data-tauri-drag-region
      className="flex items-center justify-between h-8 px-3 bg-dark-surface/80 backdrop-blur border-b border-dark-border select-none shrink-0"
    >
      {/* App icon + title */}
      <div data-tauri-drag-region className="flex items-center gap-2 text-xs text-dark-muted">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" className="text-primary">
          <path
            d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
        <span data-tauri-drag-region>{t('app.title')}</span>
      </div>

      {/* Window controls */}
      <div className="flex items-center gap-1">
        {/* Minimize button */}
        <button
          onClick={() => appWindow.minimize()}
          className="flex items-center justify-center w-6 h-6 rounded hover:bg-white/10 transition-colors text-dark-muted hover:text-dark-text"
          aria-label="Minimize"
        >
          <svg width="10" height="1" viewBox="0 0 10 1">
            <rect width="10" height="1" fill="currentColor" />
          </svg>
        </button>

        {/* Close button */}
        <button
          onClick={() => appWindow.close()}
          className="flex items-center justify-center w-6 h-6 rounded hover:bg-error/80 transition-colors text-dark-muted hover:text-white"
          aria-label="Close"
        >
          <svg width="10" height="10" viewBox="0 0 10 10">
            <path d="M1 1l8 8M9 1l-8 8" stroke="currentColor" strokeWidth="1.5" />
          </svg>
        </button>
      </div>
    </div>
  );
}
