import { useI18n } from '../hooks/useI18n';

export function LanguageToggle() {
  const { locale, setLocale } = useI18n();

  const toggleLocale = () => {
    setLocale(locale === 'en' ? 'th' : 'en');
  };

  return (
    <button
      onClick={toggleLocale}
      className="flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-dark-surface border border-dark-border hover:border-primary/50 transition-colors text-xs"
      title={locale === 'en' ? 'เปลี่ยนเป็นภาษาไทย' : 'Switch to English'}
    >
      <span className="text-sm">{locale === 'en' ? '🇺🇸' : '🇹🇭'}</span>
      <span className="text-dark-muted">{locale === 'en' ? 'EN' : 'TH'}</span>
    </button>
  );
}
