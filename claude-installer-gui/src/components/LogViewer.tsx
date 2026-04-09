import { useEffect, useRef } from 'react';
import type { LogEntry } from '../hooks/useInstaller';

interface LogViewerProps {
  logs: LogEntry[];
  maxHeight?: number;
}

const levelColors: Record<string, string> = {
  info: 'text-gray-300',
  warn: 'text-warning',
  error: 'text-error',
};

export function LogViewer({ logs, maxHeight = 160 }: LogViewerProps) {
  const scrollRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when new logs arrive
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs.length]);

  return (
    <div className="relative rounded-lg border border-dark-border bg-black/40 overflow-hidden">
      <div
        ref={scrollRef}
        className="log-scrollbar overflow-y-auto font-mono text-xs leading-5 p-3"
        style={{ maxHeight: `${maxHeight}px` }}
      >
        {logs.length === 0 ? (
          <span className="text-dark-muted">Waiting for output...</span>
        ) : (
          logs.map((log, i) => (
            <div key={i} className="flex gap-2">
              <span className="text-dark-muted shrink-0">[{log.timestamp}]</span>
              <span className={levelColors[log.level] || 'text-gray-300'}>
                {log.message}
              </span>
            </div>
          ))
        )}
      </div>

      {/* Copy button */}
      <button
        onClick={() => {
          const text = logs
            .map((l) => `[${l.timestamp}] ${l.level.toUpperCase()} ${l.message}`)
            .join('\n');
          navigator.clipboard.writeText(text);
        }}
        className="absolute top-2 right-2 text-[10px] px-2 py-0.5 rounded bg-dark-border/50 hover:bg-dark-border text-dark-muted hover:text-dark-text transition-colors"
        title="Copy log"
      >
        Copy
      </button>
    </div>
  );
}
