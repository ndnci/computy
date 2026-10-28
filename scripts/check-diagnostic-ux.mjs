import assert from 'node:assert/strict';
import { chromium } from '@playwright/test';
import { readFile, copyFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
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
const base = process.env.COMPUTY_TEST_BASE_URL || 'http://127.0.0.1:1420';
async function capture(name) {
  for (const theme of ['light', 'dark']) {
    await page.emulateMedia({ colorScheme: theme });
    await page.waitForFunction(
      (expected) => document.documentElement.dataset.theme === expected,
      theme,
    );
    await page.screenshot({ path: `docs/screenshots/${name}-${theme}.png`, fullPage: true });
  }
  await page.emulateMedia({ colorScheme: 'light' });
}
await page.goto(base + '/#diagnostics');
await page.getByText('Moteur Rust connecté', { exact: true }).waitFor();
await page
  .getByText('Tous ces profils s’exécutent dans votre session. Aucun ne redémarre le PC.', {
    exact: true,
  })
  .waitFor();
for (const selector of ['.scope-card .inline-icon-heading', '.local-card .inline-icon-heading']) {
  const icon = await page.locator(selector).locator('svg').boundingBox();
  const title = await page.locator(selector).locator('h3,b').boundingBox();
  assert.ok(icon && title);
  assert.ok(
    Math.abs(icon.y + icon.height / 2 - (title.y + title.height / 2)) < 5,
    selector + ' icon should align with title',
  );
  assert.ok(title.x > icon.x + icon.width, selector + ' title follows icon');
}
await page.getByRole('button', { name: 'Avant démarrage', exact: false }).click();
await page
  .getByRole('heading', { name: 'Tester la mémoire avant le lancement du système' })
  .waitFor();
const reopen = page.getByLabel('Rouvrir Computy à la connexion pour retrouver le résultat', {
  exact: true,
});
assert.equal(await reopen.isChecked(), true);
const agreement = page.getByLabel(
  'J’autorise Computy à préparer un seul démarrage de test. Cela ne redémarre pas le PC maintenant.',
  { exact: true },
);
assert.equal(await agreement.isChecked(), false);
await capture('boot-guide-internal');
await page.getByRole('button', { name: 'Avec clé USB', exact: true }).click();
await page.getByText('Rechercher les clés compatibles', { exact: true }).waitFor();
await capture('boot-guide-usb');
// Capability fixture only: no simulated health results and no system command is invoked.
await page.route('**/api/computy', async (route) => {
  const data = route.request().postDataJSON();
  if (data.method === 'offline_status')
    return route.fulfill({
      json: { result: { supported: true, pending: false, platform: 'windows' } },
    });
  return route.continue();
});
await page.reload();
await page.getByRole('button', { name: 'Avant démarrage', exact: false }).click();
await page.getByRole('button', { name: 'Ouvrir le diagnostic Windows', exact: true }).waitFor();
assert.equal(await reopen.isChecked(), true);
await page.unroute('**/api/computy');
if (process.env.COMPUTY_REFERENCE_REPORT) {
  const envelope = JSON.parse(await readFile(process.env.COMPUTY_REFERENCE_REPORT, 'utf8'));
  const report = JSON.parse(envelope.payload);
  assert.match(report.id, /^[0-9-]+$/);
  await copyFile(
    process.env.COMPUTY_REFERENCE_REPORT,
    path.join(process.env.COMPUTY_DATA_DIR, 'reports', report.id + '.json'),
  );
  await page.goto(base + '/#history');
  await page.reload();
  await page.getByText(report.label, { exact: true }).click();
  await page.locator('.report-intro').waitFor();
  assert.equal(report.summary.score, 10);
  await page.locator('.report-intro .lucide-smile').waitFor();
  await page
    .getByText('Informations indisponibles, pas erreurs détectées', { exact: true })
    .waitFor();
  assert.equal(
    await page.locator('.report-actions [data-result-status]').getAttribute('data-result-status'),
    'passed',
  );
  await capture('ram-report-explained');
}
assert.deepEqual(errors, []);
await writeFile(
  'docs/screenshots/diagnostic-ux-results.json',
  JSON.stringify(
    {
      separate_modes: true,
      explicit_consent: true,
      startup_default_in_boot_flow: true,
      inline_icon_titles: true,
      reference_report_checked: !!process.env.COMPUTY_REFERENCE_REPORT,
      errors,
    },
    null,
    2,
  ) + '\n',
);
await browser.close();
console.log(
  'Boot guidance, explicit consent, startup default, aligned headings and report explanation verified.',
);
