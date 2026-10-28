import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { languages, resolveLanguage, setLanguage, tx, t, currentLocale } from '../src/i18n.mjs';
const source = JSON.parse(
  await readFile(new URL('../src/locales/source.json', import.meta.url), 'utf8'),
);
const catalogs = JSON.parse(
  await readFile(new URL('../src/locales/catalogs.json', import.meta.url), 'utf8'),
);
const markers = (value) =>
  [...value.matchAll(/\{\{[^{}]+\}\}|\{[A-Za-z_]\w*\}/g)].map((match) => match[0]).sort();
test('25 genuine locales include RTL and resolve system variants correctly', () => {
  assert.equal(languages.length, 25);
  assert.equal(new Set(languages.map((l) => l.code)).size, 25);
  assert.equal(resolveLanguage('system', ['zh-Hant-HK', 'en-US']), 'zh-TW');
  assert.equal(resolveLanguage('system', ['pt-PT']), 'pt-BR');
  assert.equal(resolveLanguage('system', ['fr-CA']), 'fr');
  assert.equal(resolveLanguage('system', ['xx-XX']), 'en');
  assert.deepEqual(
    languages.filter((l) => l.dir === 'rtl').map((l) => l.code),
    ['ar', 'ur', 'fa', 'he'],
  );
});
test('every locale contains every extracted message with intact interpolation variables', () => {
  for (const [language, catalog] of Object.entries(catalogs))
    for (const [key, original] of Object.entries(source)) {
      assert.ok(
        typeof catalog[key] === 'string' && catalog[key].trim(),
        `${language} missing ${key}`,
      );
      assert.deepEqual(markers(catalog[key]), markers(original), `${language} variables in ${key}`);
      assert.ok(!catalog[key].includes('▁'), `${language} undecoded text in ${key}`);
    }
});
test('critical result labels remain distinct in all languages', () => {
  for (const [language, catalog] of Object.entries(catalogs)) {
    const labels = [
      'Conforme au test',
      'Anomalie détectée',
      'Non vérifié',
      'À examiner',
      'Interrompu',
    ].map((key) => catalog[key]);
    assert.equal(new Set(labels).size, labels.length, `${language} conflates results`);
  }
});
test('catalogs load on demand, preserve data and format numbers per locale', async () => {
  await setLanguage('ja');
  assert.equal(currentLocale(), 'ja');
  assert.equal(t('Vue d’ensemble'), '概要');
  assert.equal(t('Anomalie détectée'), '問題を検出');
  assert.equal(tx('FIRMWARE-PART-123'), 'FIRMWARE-PART-123');
  await setLanguage('ar');
  assert.equal(t('Non vérifié'), 'لم يتم التحقق');
  assert.equal(tx(123), new Intl.NumberFormat('ar').format(123));
  await setLanguage('fr');
});
