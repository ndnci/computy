import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
const catalogs = JSON.parse(readFileSync('locales/catalogs.json', 'utf8'));
const overrides = JSON.parse(readFileSync('locales/overrides.json', 'utf8'));
for (const [language, values] of Object.entries(overrides))
  Object.assign(catalogs[language], values);
writeFileSync('locales/catalogs.json', JSON.stringify(catalogs, null, 2) + '\n');
mkdirSync('locales/messages', { recursive: true });
for (const [language, translation] of Object.entries(catalogs)) {
  writeFileSync(`locales/messages/${language}.json`, JSON.stringify(translation, null, 2) + '\n');
}
console.log(`${Object.keys(catalogs).length} locale catalogs embedded by the desktop application.`);
