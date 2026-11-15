// Extracts the number, date and duration conventions the interface uses from ICU
// (Node's Intl) into a compact table for the native Rust formatter.
// Usage: node scripts/generate-intl.mjs
import { writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const locales = [
  'en',
  'fr',
  'es',
  'pt-BR',
  'de',
  'it',
  'nl',
  'pl',
  'ru',
  'uk',
  'tr',
  'ar',
  'hi',
  'bn',
  'ur',
  'id',
  'vi',
  'th',
  'ko',
  'ja',
  'zh-CN',
  'zh-TW',
  'fa',
  'he',
  'sw',
];
// Distinct components make each formatted part identifiable.
const sample = new Date(2031, 10, 23, 17, 45); // 23 Nov 2031, 17:45 local time
const long = { day: 'numeric', month: 'long', year: 'numeric', hour: '2-digit', minute: '2-digit' };
const short = { day: '2-digit', month: 'short', hour: '2-digit', minute: '2-digit' };

function template(locale, options) {
  return new Intl.DateTimeFormat(locale, options)
    .formatToParts(sample)
    .map((part) => (part.type === 'literal' ? { literal: part.value } : part.type));
}
// Month names indexed by the locale calendar's month number (Persian for fa).
function months(locale, options) {
  const format = new Intl.DateTimeFormat(locale, options);
  const number = new Intl.DateTimeFormat(`${locale}-u-nu-latn`, { month: 'numeric' });
  const names = Array(12).fill(null);
  for (let day = 0; day < 366 && names.includes(null); day += 5) {
    const date = new Date(2031, 0, 1 + day, 17, 45);
    const index = Number(number.formatToParts(date).find((p) => p.type === 'month').value) - 1;
    names[index] ??= format.formatToParts(date).find((p) => p.type === 'month').value;
  }
  return names;
}
function dayPeriods(locale, options) {
  const format = new Intl.DateTimeFormat(locale, options);
  const period = (hour) =>
    format.formatToParts(new Date(2031, 10, 23, hour, 45)).find((p) => p.type === 'dayPeriod')
      ?.value;
  return { am: period(9), pm: period(17) };
}
function units(locale, rules) {
  // Short units do not vary by plural category in the supported locales; keep
  // one pattern and fail generation if that ever changes.
  const result = {};
  for (const unit of ['millisecond', 'second', 'minute']) {
    const format = new Intl.NumberFormat(locale, {
      style: 'unit',
      unit,
      unitDisplay: 'short',
      maximumFractionDigits: 1,
    });
    const patterns = {};
    // Search small integers and one-decimal values for each plural category.
    for (const value of [
      0, 1, 2, 3, 4, 5, 6, 10, 11, 12, 21, 22, 25, 100, 101, 102, 111, 1.5, 2.5,
    ]) {
      const category = rules.select(value) + (Number.isInteger(value) ? '' : ':fraction');
      if (patterns[category]) continue;
      const number = new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }).format(value);
      patterns[category] = format.format(value).replace(number, '{0}');
    }
    const distinct = new Set(Object.values(patterns));
    if (distinct.size !== 1)
      throw new Error(`${locale} ${unit} varies by plural: ${[...distinct]}`);
    result[unit] = [...distinct][0];
  }
  return result;
}
const table = {};
for (const locale of locales) {
  const numbers = new Intl.NumberFormat(locale);
  const parts = numbers.formatToParts(1234567.5);
  const digits = new Intl.NumberFormat(locale, { useGrouping: false }).format(9876543210);
  const rules = new Intl.PluralRules(locale);
  table[locale] = {
    decimal: parts.find((p) => p.type === 'decimal')?.value ?? '.',
    group: parts.find((p) => p.type === 'group')?.value ?? '',
    // Indian-style grouping (12,34,567) and minimum grouping digits (es/pl: 1234).
    grouping: new Intl.NumberFormat(`${locale}-u-nu-latn`).format(1234567).startsWith('12,34')
      ? 'indian'
      : 'standard',
    min_grouping: /^\p{Nd}+$/u.test(numbers.format(1234)) ? 2 : 1,
    calendar: new Intl.DateTimeFormat(locale).resolvedOptions().calendar,
    digits: [...digits].reverse().join(''),
    date_long: template(locale, long),
    date_short: template(locale, short),
    months_long: months(locale, long),
    months_short: months(locale, short),
    periods: dayPeriods(locale, long),
    plural: rules.resolvedOptions().pluralCategories,
    units: units(locale, rules),
    samples: {
      long: new Intl.DateTimeFormat(locale, long).format(sample),
      short: new Intl.DateTimeFormat(locale, short).format(sample),
      number: numbers.format(1234567.5),
      small: numbers.format(1234),
    },
  };
}
await writeFile(path.join(root, 'desktop/src/intl.json'), JSON.stringify(table, null, 1) + '\n');
console.log(
  Object.entries(table)
    .map(
      ([l, v]) =>
        `${l}: ${v.samples.long} | ${v.samples.short} | ${v.samples.number} | ${v.samples.small}`,
    )
    .join('\n'),
);
