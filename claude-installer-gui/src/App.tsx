import { useState, useEffect } from 'react';
import { AnimatePresence } from 'motion/react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { TitleBar } from './components/TitleBar';
import { Welcome } from './screens/Welcome';
import { SystemCheck } from './screens/SystemCheck';
import { Installation } from './screens/Installation';
import { Completion } from './screens/Completion';
import { Uninstall } from './screens/Uninstall';
import { useInstaller } from './hooks/useInstaller';

type Screen = 'welcome' | 'systemcheck' | 'installation' | 'completion' | 'uninstall';

export default function App() {
  const [screen, setScreen] = useState<Screen>('welcome');
  const [theme, setTheme] = useState<'dark' | 'light'>('dark');
  const installer = useInstaller();

  // Detect system theme and --uninstall flag on mount
  useEffect(() => {
    // Theme detection
    async function detectTheme() {
      try {
        const appWindow = getCurrentWindow();
        const systemTheme = await appWindow.theme();
        if (systemTheme) {
          setTheme(systemTheme === 'dark' ? 'dark' : 'light');
        }
      } catch {
        // Fallback: use media query
        if (window.matchMedia('(prefers-color-scheme: light)').matches) {
          setTheme('light');
        }
      }
    }
    detectTheme();

    // Check for --uninstall CLI flag
    async function checkArgs() {
      try {
        const isUninstall = await invoke<boolean>('check_cli_args');
        if (isUninstall) {
          setScreen('uninstall');
        }
      } catch {
        // Ignore — not critical
      }
    }
    checkArgs();
  }, []);

  // Apply theme class to root element
  useEffect(() => {
    const root = document.getElementById('root');
    if (root) {
      root.className = theme === 'light' ? 'light' : '';
    }
  }, [theme]);

  // Navigate to installation when install starts
  useEffect(() => {
    if (installer.state.phase === 'installing' && screen !== 'installation') {
      setScreen('installation');
    }
    if (
      (installer.state.phase === 'done' || installer.state.phase === 'cancelled') &&
      screen === 'installation'
    ) {
      setScreen('completion');
    }
  }, [installer.state.phase, screen]);

  return (
    <div className="flex flex-col h-screen w-screen bg-dark-bg">
      {/* Custom title bar — always visible */}
      <TitleBar />

      {/* Screen content */}
      <div className="flex-1 overflow-hidden">
        <AnimatePresence mode="wait">
          {screen === 'welcome' && (
            <Welcome
              key="welcome"
              onStart={() => {
                installer.reset();
                setScreen('systemcheck');
              }}
              onUninstall={() => setScreen('uninstall')}
            />
          )}

          {screen === 'systemcheck' && (
            <SystemCheck
              key="systemcheck"
              state={installer.state}
              onRunCheck={installer.runSystemCheck}
              onInstall={(skipNode) => {
                installer.runInstallation(skipNode);
              }}
              onSkipNodeChange={installer.setSkipNode}
              onBack={() => setScreen('welcome')}
            />
          )}

          {screen === 'installation' && (
            <Installation
              key="installation"
              state={installer.state}
              onCancel={installer.cancelInstallation}
            />
          )}

          {screen === 'completion' && (
            <Completion
              key="completion"
              state={installer.state}
              onRetry={() => {
                installer.reset();
                setScreen('systemcheck');
              }}
              onExportLog={installer.exportLogs}
              onOpenTerminal={installer.openTerminal}
              onRelaunchClaude={installer.relaunchClaude}
              onClose={async () => {
                const appWindow = getCurrentWindow();
                await appWindow.close();
              }}
            />
          )}

          {screen === 'uninstall' && (
            <Uninstall
              key="uninstall"
              onBack={() => setScreen('welcome')}
            />
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}
