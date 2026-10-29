import { chromium } from '@playwright/test';
import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
const base = process.env.COMPUTY_TEST_BASE_URL || 'http://127.0.0.1:1420';
const catalogs = JSON.parse(await readFile('src/locales/catalogs.json', 'utf8'));
const french = JSON.parse(await readFile('src/locales/source.json', 'utf8'));
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
await page.goto(base + '/#settings');
await page.waitForFunction(
  () =>
    !!document.querySelector('.main-footer b') &&
    !document.querySelector('.connection') &&
    !document.querySelector('.engine-error'),
  { timeout: 30000 },
);
// Verify explicit override, then restoration of the system-controlled theme.
await page.locator('.theme-option').nth(2).click();
await page.locator('.settings-actions .primary').click();
await page.waitForFunction(() => document.documentElement.dataset.theme === 'dark');
await page.emulateMedia({ colorScheme: 'light' });
assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark');
await page.locator('.theme-option').nth(0).click();
await page.locator('.settings-actions .primary').click();
await page.waitForFunction(() => document.documentElement.dataset.theme === 'light');
const languages = await page
  .locator('#interface-language option')
  .evaluateAll((options) =>
    options.map((option) => option.value).filter((code) => code !== 'system'),
  );
assert.equal(languages.length, 25);
const captureLocales = new Set(['en', 'ar', 'ja', 'zh-CN', 'hi']);
for (const code of languages) {
  console.log(`Checking locale: ${code}`);
  await page.locator('#interface-language').selectOption(code);
  await page.locator('.settings-actions .primary').click();
  await page
    .waitForFunction((language) => document.documentElement.lang === language, code)
    .catch(async (error) => {
      console.error(
        JSON.stringify({
          code,
          actual: await page.locator('html').getAttribute('lang'),
          errors,
          notice: await page.locator('.toast').allTextContents(),
        }),
      );
      throw error;
    });
  const expectedDir = ['ar', 'ur', 'fa', 'he'].includes(code) ? 'rtl' : 'ltr';
  assert.equal(await page.locator('html').getAttribute('dir'), expectedDir);
  const catalog = code === 'fr' ? french : catalogs[code];
  assert.ok(
    (await page.locator('.nav-item').nth(0).textContent()).includes(catalog['Vue d’ensemble']),
  );
  await page.locator('.nav-item').nth(1).click();
  assert.equal(await page.locator('.test-component-option input').count(), 3);
  assert.ok(
    (await page.locator('.test-component-option').nth(0).textContent()).includes(
      catalog['Mémoire vive'],
    ),
  );
  assert.equal(
    await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth),
    true,
    `${code} layout overflow`,
  );
  if (captureLocales.has(code)) {
    await page.locator('.nav-item').nth(0).click();
    for (const theme of ['light', 'dark']) {
      await page.emulateMedia({ colorScheme: theme });
      await page.waitForFunction(
        (value) => document.documentElement.dataset.theme === value,
        theme,
      );
      await page.screenshot({
        path: `docs/screenshots/overview-${code}-${theme}.png`,
        fullPage: true,
      });
    }
    await page.emulateMedia({ colorScheme: 'light' });
  }
  await page.locator('.sidebar-bottom .nav-item').nth(0).click();
}
await page.locator('#interface-language').selectOption('fr');
await page.locator('.settings-actions .primary').click();
await page.waitForFunction(() => document.documentElement.lang === 'fr');
for (const width of [1024, 768, 430]) {
  await page.setViewportSize({ width, height: 950 });
  await page.locator('.nav-item').nth(0).click();
  assert.equal(
    await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth),
    true,
    `${width}px overflow`,
  );
  for (const theme of ['light', 'dark']) {
    await page.emulateMedia({ colorScheme: theme });
    await page.waitForFunction((value) => document.documentElement.dataset.theme === value, theme);
    await page.screenshot({
      path: `docs/screenshots/responsive-${width}-${theme}.png`,
      fullPage: true,
    });
  }
  await page.emulateMedia({ colorScheme: 'light' });
  await page.locator('.sidebar-bottom .nav-item').nth(0).click();
}
assert.deepEqual(errors, []);
await writeFile(
  'docs/screenshots/locales-results.json',
  JSON.stringify(
    {
      locales: languages,
      rtl: true,
      system_theme: true,
      explicit_theme: true,
      responsive_widths: [1024, 768, 430],
      errors,
    },
    null,
    2,
  ),
);
await browser.close();
console.log(`${languages.length} locales, RTL, theme overrides and responsive layouts verified.`);
