import assert from 'node:assert/strict';
import { chromium } from '@playwright/test';
import { writeFile } from 'node:fs/promises';
const base = process.env.COMPUTY_TEST_BASE_URL || 'http://127.0.0.1:1420';
const browser = await chromium.launch({
  headless: true,
  executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE || '/usr/bin/google-chrome',
  args: ['--no-sandbox'],
});
const context = await browser.newContext({
  viewport: { width: 1440, height: 1000 },
  locale: 'fr-FR',
  colorScheme: 'light',
  reducedMotion: 'reduce',
});
context.setDefaultTimeout(20000);
const page = await context.newPage();
const errors = [];
page.on('pageerror', (e) => errors.push(e.message));
async function call(method, params = {}) {
  const r = await context.request.post(base + '/api/computy', {
    headers: { 'x-computy-client': 'local-ui' },
    data: { method, params },
  });
  const value = await r.json();
  if (value.error) throw Error(value.error);
  return value.result;
}
try {
  const prefs = await call('preferences');
  await call('save_preferences', {
    ...prefs,
    language: 'fr',
    theme: 'system',
    tray_enabled: true,
    tray_components: ['memory', 'cpu', 'gpu'],
    tray_disks: [],
    tray_style: 'icon_text',
  });
  await page.goto(base + '/#settings');
  await page.getByText('Moteur Rust connecté', { exact: true }).waitFor();
  for (const style of ['ring', 'fill', 'icon_text']) {
    await page.locator(`.tray-style-option input[value="${style}"]`).check();
    await page.getByRole('button', { name: 'Enregistrer les préférences', exact: true }).click();
    assert.equal((await call('preferences')).tray_style, style);
    await page.reload();
    await page.locator(`.tray-style-option input[value="${style}"]:checked`).waitFor();
  }
  assert.equal(await page.locator('.tray-style-option').count(), 3);
  await page.waitForTimeout(2200);
  for (const theme of ['light', 'dark']) {
    await page.emulateMedia({ colorScheme: theme });
    await page.waitForFunction(
      (expected) => document.documentElement.dataset.theme === expected,
      theme,
    );
    await page
      .locator('.monitor-settings')
      .screenshot({ path: `docs/screenshots/tray-styles-${theme}.png` });
    const privacy = page.locator('.privacy-settings');
    await privacy.scrollIntoViewIfNeeded();
    const css = await privacy.evaluate((element) => {
      const s = getComputedStyle(element);
      return {
        padding: parseFloat(s.paddingTop),
        radius: parseFloat(s.borderRadius),
        panel: element.classList.contains('panel'),
      };
    });
    assert.ok(css.panel && css.padding >= 20 && css.radius >= 12);
    await privacy.screenshot({ path: `docs/screenshots/privacy-card-${theme}.png` });
  }
  const preferencesButton = page.locator('.sidebar-bottom > .nav-item').nth(0);
  const about = page.locator('.sidebar-bottom > .nav-item').nth(1);
  await about.hover();
  const a = await preferencesButton.boundingBox();
  const b = await about.boundingBox();
  assert.ok(b.y - (a.y + a.height) >= 4, 'hover backgrounds must retain a gap');
  const popup = await context.newPage();
  await popup.setViewportSize({ width: 420, height: 720 });
  popup.on('pageerror', (e) => errors.push(e.message));
  for (const metric of ['memory', 'cpu']) {
    await popup.goto(base + `/?popover=1&metric=${metric}`);
    await popup.locator('.popover-process').first().waitFor();
    assert.ok((await popup.locator('.popover-process').count()) <= 10);
    assert.equal(await popup.locator('.popover-actions .popover-action').count(), 3);
    assert.equal(await popup.locator('.popover-actions .popover-action svg').count(), 5);
    for (const theme of ['light', 'dark']) {
      await popup.emulateMedia({ colorScheme: theme });
      await popup.waitForFunction(
        (expected) => document.documentElement.dataset.theme === expected,
        theme,
      );
      await popup.screenshot({ path: `docs/screenshots/tray-panel-${metric}-${theme}.png` });
      assert.ok(
        await popup.evaluate(
          () => document.querySelector('.monitor-popover').scrollWidth <= innerWidth,
        ),
      );
    }
  }
  await popup.getByRole('button', { name: 'Voir tout', exact: true }).click();
  await popup.locator('.process-table').waitFor();
  assert.ok(popup.url().includes('metric=cpu'));
  assert.deepEqual(errors, []);
  await writeFile(
    'docs/screenshots/tray-style-results.json',
    JSON.stringify(
      {
        styles: ['icon_text', 'ring', 'fill'],
        default_style: 'icon_text',
        persisted: true,
        theme_pairs: true,
        popover_actions_with_icons: true,
        privacy_shared_panel: true,
        sidebar_gap: true,
        errors,
      },
      null,
      2,
    ) + '\n',
  );
} finally {
  await browser.close();
}
console.log('Tray styles, popovers, shared privacy panel and sidebar spacing verified.');
