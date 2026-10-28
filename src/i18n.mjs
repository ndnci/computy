import i18next from 'i18next';
import { initReactI18next } from 'react-i18next';
import source from './locales/source.json' with { type: 'json' };
import english from './locales/messages/en.mjs';
import catalogLoaders from './locales/loaders.mjs';
export const languages = [
  ['en', 'English'],
  ['fr', 'Français'],
  ['es', 'Español'],
  ['pt-BR', 'Português (Brasil)'],
  ['de', 'Deutsch'],
  ['it', 'Italiano'],
  ['nl', 'Nederlands'],
  ['pl', 'Polski'],
  ['ru', 'Русский'],
  ['uk', 'Українська'],
  ['tr', 'Türkçe'],
  ['ar', 'العربية'],
  ['hi', 'हिन्दी'],
  ['bn', 'বাংলা'],
  ['ur', 'اردو'],
  ['id', 'Bahasa Indonesia'],
  ['vi', 'Tiếng Việt'],
  ['th', 'ไทย'],
  ['ko', '한국어'],
  ['ja', '日本語'],
  ['zh-CN', '简体中文'],
  ['zh-TW', '繁體中文'],
  ['fa', 'فارسی'],
  ['he', 'עברית'],
  ['sw', 'Kiswahili'],
].map(([code, name]) => ({
  code,
  name,
  dir: ['ar', 'ur', 'fa', 'he'].includes(code) ? 'rtl' : 'ltr',
}));
export function resolveLanguage(
  requested = 'system',
  systemLanguages = typeof navigator === 'undefined' ? ['fr'] : navigator.languages,
) {
  const candidates = requested === 'system' ? systemLanguages : [requested];
  for (const candidate of candidates) {
    if (languages.some((l) => l.code === candidate)) return candidate;
    const lower = candidate.toLowerCase();
    if (lower.startsWith('zh')) return /tw|hk|hant/.test(lower) ? 'zh-TW' : 'zh-CN';
    if (lower.startsWith('pt')) return 'pt-BR';
    const base = candidate.split('-')[0];
    if (languages.some((l) => l.code === base)) return base;
  }
  return 'en';
}
const resources = {
  fr: { translation: source },
  en: { translation: english },
};
void i18next.use(initReactI18next).init({
  resources,
  lng: resolveLanguage(),
  fallbackLng: 'en',
  initAsync: false,
  keySeparator: false,
  nsSeparator: false,
  interpolation: { escapeValue: false },
  returnEmptyString: false,
});
export function currentLocale() {
  return i18next.resolvedLanguage || i18next.language || 'en';
}
let languageRequest = 0;
export async function setLanguage(language) {
  const requested = resolveLanguage(language);
  const request = ++languageRequest;
  if (!i18next.hasResourceBundle(requested, 'translation')) {
    const catalog = await catalogLoaders[requested]();
    i18next.addResourceBundle(requested, 'translation', catalog.default, true, true);
  }
  if (request === languageRequest) return i18next.changeLanguage(requested);
}
export function t(key, params = {}) {
  const values = Object.fromEntries(
    Object.entries(params).map(([name, value]) => [
      name,
      typeof value === 'number' && name !== 'count' ? value.toLocaleString(currentLocale()) : value,
    ]),
  );
  return i18next.t(key, { ...values, defaultValue: key });
}
const dynamicMessages = Object.keys(source)
  .filter((key) => !key.includes('{{') && /\{[A-Za-z_]\w*\}/.test(key))
  .map((key) => {
    const variables = [];
    let pattern = '';
    let cursor = 0;
    const escape = (value) => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    for (const match of key.matchAll(/\{([A-Za-z_]\w*)\}/g)) {
      pattern += escape(key.slice(cursor, match.index)) + '(.+?)';
      variables.push(match[1]);
      cursor = match.index + match[0].length;
    }
    pattern += escape(key.slice(cursor));
    return { key, variables, pattern: new RegExp('^' + pattern + '$') };
  });
export function tx(value) {
  if (typeof value === 'number') return value.toLocaleString(currentLocale());
  if (typeof value !== 'string' || !value.trim()) return value;
  const key = value.replace(/\s+/g, ' ').trim();
  const prefix = /^\s/.test(value) ? ' ' : '';
  const suffix = /\s$/.test(value) ? ' ' : '';
  if (i18next.exists(key)) return prefix + t(key) + suffix;
  for (const template of dynamicMessages) {
    const match = key.match(template.pattern);
    if (match) {
      const values = Object.fromEntries(
        template.variables.map((name, index) => [name, match[index + 1]]),
      );
      return (
        prefix +
        t(template.key).replace(
          /\{([A-Za-z_]\w*)\}/g,
          (original, name) => values[name] ?? original,
        ) +
        suffix
      );
    }
  }
  return value;
}
export default i18next;
