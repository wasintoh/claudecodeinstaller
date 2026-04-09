import { createContext, useContext, useState, useCallback, type ReactNode } from 'react';
import en from '../i18n/en.json';
import th from '../i18n/th.json';

type Locale = 'en' | 'th';

// Flatten nested JSON into dot-notation keys for easy lookup
// e.g., { welcome: { title: "Hello" } } => { "welcome.title": "Hello" }
type FlatMap = Record<string, string>;

function flattenObject(obj: Record<string, unknown>, prefix = ''): FlatMap {
  const result: FlatMap = {};
  for (const [key, value] of Object.entries(obj)) {
    const fullKey = prefix ? `${prefix}.${key}` : key;
    if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
      Object.assign(result, flattenObject(value as Record<string, unknown>, fullKey));
    } else {
      result[fullKey] = String(value);
    }
  }
  return result;
}

const translations: Record<Locale, FlatMap> = {
  en: flattenObject(en),
  th: flattenObject(th),
};

interface I18nContextValue {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  /** Translate a key. Supports template variables: t('key', { name: 'value' }) */
  t: (key: string, vars?: Record<string, string | number>) => string;
}

const I18nContext = createContext<I18nContextValue | null>(null);

/** Get the initial locale from localStorage or default to English */
function getInitialLocale(): Locale {
  try {
    const stored = localStorage.getItem('claude-installer-locale');
    if (stored === 'th' || stored === 'en') return stored;
  } catch {
    // localStorage may not be available
  }
  return 'en';
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(getInitialLocale);

  const setLocale = useCallback((newLocale: Locale) => {
    setLocaleState(newLocale);
    try {
      localStorage.setItem('claude-installer-locale', newLocale);
    } catch {
      // Silently fail if localStorage is unavailable
    }
  }, []);

  const t = useCallback(
    (key: string, vars?: Record<string, string | number>): string => {
      let value = translations[locale][key];
      if (!value) {
        // Fallback to English if Thai translation is missing
        value = translations['en'][key];
      }
      if (!value) {
        // Return the key itself as last resort
        return key;
      }
      // Replace template variables: {name} => value
      if (vars) {
        for (const [varName, varValue] of Object.entries(vars)) {
          value = value.replace(new RegExp(`\\{${varName}\\}`, 'g'), String(varValue));
        }
      }
      return value;
    },
    [locale]
  );

  return (
    <I18nContext.Provider value={{ locale, setLocale, t }}>
      {children}
    </I18nContext.Provider>
  );
}

export function useI18n(): I18nContextValue {
  const context = useContext(I18nContext);
  if (!context) {
    throw new Error('useI18n must be used within an I18nProvider');
  }
  return context;
}
