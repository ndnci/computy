import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
const source = JSON.parse(
  await readFile(new URL('../locales/source.json', import.meta.url), 'utf8'),
);
const catalogs = JSON.parse(
  await readFile(new URL('../locales/catalogs.json', import.meta.url), 'utf8'),
);
const markers = (value) =>
  [...value.matchAll(/\{\{[^{}]+\}\}|\{[A-Za-z_]\w*\}/g)].map((match) => match[0]).sort();
test('25 locales are shipped as embedded catalogs', async () => {
  assert.equal(Object.keys(catalogs).length, 24);
  for (const language of Object.keys(catalogs))
    JSON.parse(
      await readFile(new URL(`../locales/messages/${language}.json`, import.meta.url), 'utf8'),
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
test('Computy remains a product name in every translated paragraph', () => {
  for (const [language, catalog] of Object.entries(catalogs)) {
    for (const key of Object.keys(source).filter((key) => key.includes('Computy'))) {
      assert.ok(
        catalog[key]?.includes('Computy'),
        `${language} translated the Computy brand in: ${key}`,
      );
    }
  }
});
