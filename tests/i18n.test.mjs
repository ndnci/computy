import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, readdir } from 'node:fs/promises';

const directory = new URL('../locales/', import.meta.url);
const files = (await readdir(directory)).filter((name) => name.endsWith('.json'));
const catalogs = Object.fromEntries(
  await Promise.all(
    files.map(async (name) => [
      name.slice(0, -5),
      JSON.parse(await readFile(new URL(name, directory), 'utf8')),
    ]),
  ),
);
const source = catalogs.fr;
const markers = (value) =>
  [...value.matchAll(/\{\{[^{}]+\}\}|\{[A-Za-z_]\w*\}/g)].map((match) => match[0]).sort();

test('25 locales are shipped as embedded catalogs', () => {
  assert.equal(Object.keys(catalogs).length, 25);
  assert.ok(catalogs.en && catalogs.fr);
});

test('keys are semantic English identifiers', () => {
  for (const key of Object.keys(source))
    assert.match(key, /^[a-z][a-z0-9_]*(\.[a-z0-9_]+)+$/, `invalid key ${key}`);
});

test('every locale contains every message with intact interpolation variables', () => {
  for (const [language, catalog] of Object.entries(catalogs)) {
    assert.deepEqual(Object.keys(catalog).sort(), Object.keys(source).sort(), `${language} keys`);
    for (const [key, original] of Object.entries(source)) {
      assert.ok(
        typeof catalog[key] === 'string' && catalog[key].trim(),
        `${language} missing ${key}`,
      );
      assert.deepEqual(markers(catalog[key]), markers(original), `${language} variables in ${key}`);
      assert.ok(!catalog[key].includes('▁'), `${language} undecoded text in ${key}`);
    }
  }
});

test('percent signs survive translation without printf artifacts', () => {
  for (const [language, catalog] of Object.entries(catalogs)) {
    for (const [key, value] of Object.entries(catalog))
      assert.doesNotMatch(value, /%%|%[sd]\b/, `${language} printf artifact in ${key}`);
    for (const key of ['common.percent_value', 'common.usage', 'settings.space_used'])
      assert.match(
        catalog[key],
        /^[^%٪]*(%\{\{value\}\}|\{\{value\}\}\s?[%٪])[^%٪]*$/,
        `${language} ${key} must show one percent sign next to the value`,
      );
  }
});

test('French messages are unique so engine text maps back to one key', () => {
  const values = Object.values(source);
  assert.equal(new Set(values).size, values.length);
});

test('critical result labels remain distinct in all languages', () => {
  for (const [language, catalog] of Object.entries(catalogs)) {
    const labels = [
      'common.test_passed',
      'common.issue_detected',
      'common.not_verified',
      'common.needs_review',
      'common.interrupted',
    ].map((key) => catalog[key]);
    assert.equal(new Set(labels).size, labels.length, `${language} conflates results`);
  }
});

test('Computy remains a product name in every translated paragraph', () => {
  for (const [language, catalog] of Object.entries(catalogs))
    for (const [key, text] of Object.entries(source))
      if (text.includes('Computy'))
        assert.ok(catalog[key]?.includes('Computy'), `${language} translated the brand in ${key}`);
});
