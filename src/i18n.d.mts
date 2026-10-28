export const languages: { code: string; name: string; dir: string }[];
export function resolveLanguage(requested?: string, systemLanguages?: readonly string[]): string;
export function currentLocale(): string;
export function setLanguage(language: string): Promise<unknown>;
export function t(key: string, params?: Record<string, unknown>): string;
export function tx<T>(value: T): T;
declare const i18next: import('i18next').i18n;
export default i18next;
