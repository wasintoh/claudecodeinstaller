import { useReducer, useCallback, useRef } from 'react';
import { invoke, Channel } from '@tauri-apps/api/core';

// ── Types ──

export interface CheckItem {
  key: string;
  label: string;
  status: 'pass' | 'fail' | 'warn' | 'checking' | 'skipped';
  detail: string;
  version: string | null;
}

export interface SystemCheckResult {
  items: CheckItem[];
  installCount: number;
  approxDownloadMb: number;
}

export type InstallEvent =
  | { event: 'stepStarted'; data: { step: string; totalSteps: number; currentStep: number } }
  | { event: 'downloadProgress'; data: { step: string; downloaded: number; total: number; speedBps: number; etaSecs: number } }
  | { event: 'stepLog'; data: { step: string; level: string; message: string } }
  | { event: 'retryAttempt'; data: { step: string; attempt: number; maxAttempts: number; error: string } }
  | { event: 'stepCompleted'; data: { step: string; success: boolean; version: string | null; error: string | null } }
  | { event: 'overallProgress'; data: { percent: number; message: string } };

export interface StepResult {
  step: string;
  success: boolean;
  version: string | null;
  error: string | null;
}

export interface LogEntry {
  timestamp: string;
  level: 'info' | 'warn' | 'error';
  message: string;
}

// ── State ──

export interface InstallerState {
  phase: 'idle' | 'checking' | 'ready' | 'installing' | 'done' | 'error' | 'cancelled';
  systemCheck: SystemCheckResult | null;
  skipNode: boolean;
  /** Steps to install (keys like "git", "node", "claude") */
  installQueue: string[];
  currentStep: string | null;
  currentStepIndex: number;
  totalSteps: number;
  overallPercent: number;
  downloadProgress: {
    downloaded: number;
    total: number;
    speedBps: number;
    etaSecs: number;
  } | null;
  logs: LogEntry[];
  results: StepResult[];
}

const initialState: InstallerState = {
  phase: 'idle',
  systemCheck: null,
  skipNode: false,
  installQueue: [],
  currentStep: null,
  currentStepIndex: 0,
  totalSteps: 0,
  overallPercent: 0,
  downloadProgress: null,
  logs: [],
  results: [],
};

// ── Actions ──

type Action =
  | { type: 'START_CHECK' }
  | { type: 'CHECK_COMPLETE'; payload: SystemCheckResult }
  | { type: 'CHECK_ERROR'; payload: string }
  | { type: 'SET_SKIP_NODE'; payload: boolean }
  | { type: 'START_INSTALL'; payload: { queue: string[] } }
  | { type: 'STEP_STARTED'; payload: { step: string; totalSteps: number; currentStep: number } }
  | { type: 'DOWNLOAD_PROGRESS'; payload: { downloaded: number; total: number; speedBps: number; etaSecs: number } }
  | { type: 'STEP_LOG'; payload: LogEntry }
  | { type: 'STEP_COMPLETED'; payload: StepResult }
  | { type: 'OVERALL_PROGRESS'; payload: { percent: number; message: string } }
  | { type: 'CANCEL' }
  | { type: 'RESET' };

function reducer(state: InstallerState, action: Action): InstallerState {
  switch (action.type) {
    case 'START_CHECK':
      return { ...state, phase: 'checking', systemCheck: null };

    case 'CHECK_COMPLETE':
      return {
        ...state,
        phase: 'ready',
        systemCheck: action.payload,
      };

    case 'CHECK_ERROR':
      return {
        ...state,
        phase: 'error',
        logs: [...state.logs, { timestamp: now(), level: 'error', message: action.payload }],
      };

    case 'SET_SKIP_NODE':
      return { ...state, skipNode: action.payload };

    case 'START_INSTALL':
      return {
        ...state,
        phase: 'installing',
        installQueue: action.payload.queue,
        totalSteps: action.payload.queue.length,
        currentStepIndex: 0,
        overallPercent: 0,
        results: [],
        logs: [],
      };

    case 'STEP_STARTED':
      return {
        ...state,
        currentStep: action.payload.step,
        currentStepIndex: action.payload.currentStep,
        downloadProgress: null,
      };

    case 'DOWNLOAD_PROGRESS':
      return { ...state, downloadProgress: action.payload };

    case 'STEP_LOG':
      return {
        ...state,
        logs: [...state.logs, action.payload],
      };

    case 'STEP_COMPLETED': {
      const results = [...state.results, action.payload];
      const allDone = results.length >= state.totalSteps;
      return {
        ...state,
        results,
        downloadProgress: null,
        phase: allDone ? 'done' : state.phase,
      };
    }

    case 'OVERALL_PROGRESS':
      return { ...state, overallPercent: action.payload.percent };

    case 'CANCEL':
      return { ...state, phase: 'cancelled' };

    case 'RESET':
      return initialState;

    default:
      return state;
  }
}

function now(): string {
  return new Date().toLocaleTimeString();
}

// ── Hook ──

export function useInstaller() {
  const [state, dispatch] = useReducer(reducer, initialState);
  const cancelRef = useRef(false);

  /** Run the system check */
  const runSystemCheck = useCallback(async () => {
    dispatch({ type: 'START_CHECK' });
    try {
      const result = await invoke<SystemCheckResult>('system_check');
      dispatch({ type: 'CHECK_COMPLETE', payload: result });
      return result;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      dispatch({ type: 'CHECK_ERROR', payload: msg });
      return null;
    }
  }, []);

  /** Start the installation of required components */
  const runInstallation = useCallback(async (skipNode: boolean) => {
    if (!state.systemCheck) return;

    // Determine which components to install based on check results
    const queue: string[] = [];
    for (const item of state.systemCheck.items) {
      if (item.key === 'git' && (item.status === 'fail')) {
        queue.push('git');
      }
      if (item.key === 'node' && (item.status === 'fail' || item.status === 'warn') && !skipNode) {
        queue.push('node');
      }
      if (item.key === 'claude' && item.status === 'fail') {
        queue.push('claude');
      }
    }

    if (queue.length === 0) {
      // Nothing to install, go to completion
      dispatch({ type: 'START_INSTALL', payload: { queue: [] } });
      dispatch({ type: 'STEP_COMPLETED', payload: { step: 'none', success: true, version: null, error: null } });
      return;
    }

    dispatch({ type: 'START_INSTALL', payload: { queue } });
    cancelRef.current = false;

    // Install each component sequentially
    for (let i = 0; i < queue.length; i++) {
      if (cancelRef.current) {
        dispatch({ type: 'CANCEL' });
        return;
      }

      const component = queue[i];
      const commandName = `install_${component}`;

      dispatch({
        type: 'STEP_STARTED',
        payload: { step: component, totalSteps: queue.length, currentStep: i + 1 },
      });

      // Calculate overall progress
      const basePercent = (i / queue.length) * 100;
      dispatch({
        type: 'OVERALL_PROGRESS',
        payload: { percent: basePercent, message: `Installing ${component}...` },
      });

      try {
        // Create a Tauri channel for real-time events from this step
        const onEvent = new Channel<InstallEvent>();
        onEvent.onmessage = (event: InstallEvent) => {
          switch (event.event) {
            case 'downloadProgress':
              dispatch({ type: 'DOWNLOAD_PROGRESS', payload: event.data });
              // Update overall progress based on download within this step
              if (event.data.total > 0) {
                const stepProgress = event.data.downloaded / event.data.total;
                const overallPercent = basePercent + (stepProgress / queue.length) * 100;
                dispatch({ type: 'OVERALL_PROGRESS', payload: { percent: overallPercent, message: `Downloading ${component}...` } });
              }
              break;
            case 'stepLog':
              dispatch({
                type: 'STEP_LOG',
                payload: { timestamp: now(), level: event.data.level as 'info' | 'warn' | 'error', message: event.data.message },
              });
              break;
            case 'retryAttempt':
              dispatch({
                type: 'STEP_LOG',
                payload: { timestamp: now(), level: 'warn', message: `Retry ${event.data.attempt}/${event.data.maxAttempts}: ${event.data.error}` },
              });
              break;
            case 'stepCompleted':
              dispatch({ type: 'STEP_COMPLETED', payload: event.data });
              break;
          }
        };

        await invoke(commandName, { onEvent });

        // If stepCompleted wasn't sent via channel, send it manually
        dispatch({
          type: 'STEP_COMPLETED',
          payload: { step: component, success: true, version: null, error: null },
        });

      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        dispatch({
          type: 'STEP_LOG',
          payload: { timestamp: now(), level: 'error', message: `Failed to install ${component}: ${msg}` },
        });
        dispatch({
          type: 'STEP_COMPLETED',
          payload: { step: component, success: false, version: null, error: msg },
        });
      }
    }

    // Fix PATH after all installations
    try {
      await invoke('fix_path');
    } catch (e) {
      // Non-fatal: PATH fix failure
      dispatch({
        type: 'STEP_LOG',
        payload: { timestamp: now(), level: 'warn', message: `PATH fix warning: ${e}` },
      });
    }

    dispatch({ type: 'OVERALL_PROGRESS', payload: { percent: 100, message: 'Complete' } });
  }, [state.systemCheck]);

  /** Cancel the current installation */
  const cancelInstallation = useCallback(() => {
    cancelRef.current = true;
    dispatch({ type: 'CANCEL' });
  }, []);

  /** Toggle skip Node.js option */
  const setSkipNode = useCallback((skip: boolean) => {
    dispatch({ type: 'SET_SKIP_NODE', payload: skip });
  }, []);

  /** Reset the installer to initial state */
  const reset = useCallback(() => {
    cancelRef.current = false;
    dispatch({ type: 'RESET' });
  }, []);

  /** Export logs to file */
  const exportLogs = useCallback(async (): Promise<string | null> => {
    try {
      return await invoke<string>('export_logs');
    } catch {
      return null;
    }
  }, []);

  /** Open terminal */
  const openTerminal = useCallback(async () => {
    try {
      await invoke('open_terminal');
    } catch {
      // Silently fail
    }
  }, []);

  return {
    state,
    runSystemCheck,
    runInstallation,
    cancelInstallation,
    setSkipNode,
    reset,
    exportLogs,
    openTerminal,
  };
}
