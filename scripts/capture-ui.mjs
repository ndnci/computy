import { chromium } from '@playwright/test';
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
const destination = 'docs/screenshots';
await mkdir(destination, { recursive: true });
const browser = await chromium.launch({
  headless: true,
  executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE || '/usr/bin/google-chrome',
  args: ['--no-sandbox'],
});
const context = await browser.newContext({
  viewport: { width: 1440, height: 1000 },
  locale: 'fr-FR',
  timezoneId: 'Europe/Paris',
  colorScheme: 'light',
  reducedMotion: 'reduce',
});
const page = await context.newPage();
const errors = [];
page.on('pageerror', (error) => errors.push(error.message));
await page.goto(process.env.COMPUTY_TEST_BASE_URL || 'http://127.0.0.1:1420');
await page.waitForFunction(
  () =>
    !!document.querySelector('.main-footer b') &&
    !document.querySelector('.connection') &&
    !document.querySelector('.engine-error'),
  { timeout: 30000 },
);
// Explicitly use the UI's shared preferences API, not test fixtures or mocked hardware.
await page.evaluate(async () => {
  const response = await fetch('/api/computy', {
    method: 'POST',
    headers: { 'content-type': 'application/json', 'x-computy-client': 'local-ui' },
    body: JSON.stringify({
      method: 'save_preferences',
      params: {
        memory_mib: 256,
        passes: 2,
        disk_mib: 64,
        cpu_seconds: 5,
        theme: 'system',
        language: 'fr',
      },
    }),
  });
  if (!response.ok) throw new Error(await response.text());
});
await page.reload();
await page.waitForFunction(
  () =>
    !!document.querySelector('.main-footer b') &&
    !document.querySelector('.connection') &&
    !document.querySelector('.engine-error'),
);
const captures = [];
async function capture(name) {
  for (const theme of ['light', 'dark']) {
    await page.emulateMedia({ colorScheme: theme });
    await page.waitForFunction(
      (expected) => document.documentElement.dataset.theme === expected,
      theme,
    );
    await page.evaluate(() => window.scrollTo(0, 0));
    await page.screenshot({ path: `${destination}/${name}-${theme}.png`, fullPage: true });
    captures.push(`${name}-${theme}.png`);
    assert.equal(
      await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
      true,
      `${name}/${theme} horizontal overflow`,
    );
  }
  await page.emulateMedia({ colorScheme: 'light' });
}
await capture('overview');
await page.getByRole('button', { name: 'Diagnostics', exact: true }).click();
await page.getByRole('heading', { name: 'Un diagnostic qui a du sens.' }).waitFor();
const checkboxes = page.locator('.test-component-option input');
assert.equal(await checkboxes.count(), 3);
await checkboxes.nth(1).uncheck();
await checkboxes.nth(2).uncheck();
assert.equal(await checkboxes.nth(0).isChecked(), true);
await capture('diagnostics-memory-only');
await checkboxes.nth(1).check();
await capture('diagnostics-memory-storage');
await checkboxes.nth(2).check();
await capture('diagnostics');
await page.getByRole('button', { name: /Premier regard/ }).click();
await page.getByLabel(/Nom du diagnostic/).fill('Vérification locale · complète');
await page.getByRole('button', { name: 'Lancer le diagnostic', exact: true }).click();
await page.getByRole('heading', { name: 'On regarde tout ça de plus près.' }).waitFor();
await capture('diagnostic-running');
await page.getByRole('heading', { name: 'Votre bilan est prêt.' }).waitFor({ timeout: 60000 });
await capture('diagnostic-result');
await page.getByRole('button', { name: 'Nouveau diagnostic', exact: true }).click();
await page.locator('.test-component-option input').nth(2).uncheck();
await page.getByRole('button', { name: /Premier regard/ }).click();
await page.getByLabel(/Nom du diagnostic/).fill('Vérification locale · RAM et disque');
await page.getByRole('button', { name: 'Lancer le diagnostic', exact: true }).click();
await page.getByRole('heading', { name: 'Votre bilan est prêt.' }).waitFor({ timeout: 60000 });
await page.getByRole('button', { name: /^Historique/ }).click();
await page.getByRole('heading', { name: 'L’histoire de votre matériel.' }).waitFor();
await capture('history');
const selections = page.locator('.table-checkbox input');
assert.ok((await selections.count()) >= 2);
await selections.nth(0).check();
await selections.nth(1).check();
await page.getByRole('button', { name: 'Comparer les bilans' }).click();
await page.getByRole('heading', { name: 'Les changements, côte à côte.' }).waitFor();
await capture('comparison');
for (const [name, label] of [
  ['memory', 'Mémoire vive'],
  ['storage', 'Stockage'],
  ['cpu', 'Processeur'],
  ['settings', 'Préférences'],
  ['about', 'À propos de Computy'],
]) {
  await page.getByRole('button', { name: label, exact: true }).click();
  await capture(name);
}
await page.getByRole('button', { name: 'Vue d’ensemble', exact: true }).click();
await capture('overview-with-history');
assert.deepEqual(errors, []);
await writeFile(
  `${destination}/capture-results.json`,
  JSON.stringify(
    {
      captures,
      checks: {
        checkboxes: true,
        real_diagnostics: 2,
        comparison: true,
        system_theme: true,
        horizontal_overflow: false,
        browser_errors: errors,
      },
    },
    null,
    2,
  ),
);
await browser.close();
console.log(JSON.stringify({ captures: captures.length, errors }));
