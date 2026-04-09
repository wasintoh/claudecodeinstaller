import { motion } from 'motion/react';
import { useI18n } from '../hooks/useI18n';
import { LanguageToggle } from '../components/LanguageToggle';

interface WelcomeProps {
  onStart: () => void;
  onUninstall: () => void;
}

export function Welcome({ onStart, onUninstall }: WelcomeProps) {
  const { t } = useI18n();

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -20 }}
      transition={{ duration: 0.3 }}
      className="flex flex-col h-full px-8 py-6"
    >
      {/* Language toggle in top right */}
      <div className="flex justify-end mb-2">
        <LanguageToggle />
      </div>

      {/* Logo / Banner */}
      <div className="flex flex-col items-center mb-6">
        <motion.div
          initial={{ scale: 0.8 }}
          animate={{ scale: 1 }}
          transition={{ type: 'spring', stiffness: 200, damping: 15, delay: 0.1 }}
          className="w-16 h-16 rounded-2xl bg-gradient-to-br from-primary to-amber-400 flex items-center justify-center mb-4 shadow-lg shadow-primary/20"
        >
          <svg width="32" height="32" viewBox="0 0 24 24" fill="none" className="text-white">
            <path
              d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </motion.div>
        <h1 className="text-xl font-bold text-dark-text">{t('welcome.title')}</h1>
      </div>

      {/* Description */}
      <p className="text-sm text-dark-muted mb-4 text-center">
        {t('welcome.description')}
      </p>

      {/* Checklist of what will be installed */}
      <div className="space-y-2.5 mb-6 mx-auto">
        {[
          { key: 'item_git', icon: '✦' },
          { key: 'item_node', icon: '✦' },
          { key: 'item_claude', icon: '✦' },
        ].map((item, i) => (
          <motion.div
            key={item.key}
            initial={{ opacity: 0, x: -20 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: 0.2 + i * 0.1 }}
            className="flex items-start gap-2.5"
          >
            <span className="text-primary text-sm mt-0.5">{item.icon}</span>
            <span className="text-sm text-dark-text">
              {t(`welcome.${item.key}`)}
            </span>
          </motion.div>
        ))}
      </div>

      {/* Spacer */}
      <div className="flex-1" />

      {/* Start button */}
      <motion.button
        whileHover={{ scale: 1.02 }}
        whileTap={{ scale: 0.98 }}
        onClick={onStart}
        className="w-full py-3 rounded-xl bg-gradient-to-r from-primary to-amber-500 text-white font-semibold text-sm shadow-lg shadow-primary/25 hover:shadow-primary/40 transition-shadow"
      >
        {t('welcome.start')}
      </motion.button>

      {/* Uninstall link */}
      <div className="text-center mt-3">
        <button
          onClick={onUninstall}
          className="text-xs text-dark-muted hover:text-primary transition-colors underline underline-offset-2"
        >
          {t('welcome.uninstall_link')}
        </button>
      </div>
    </motion.div>
  );
}
