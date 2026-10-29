import assert from 'node:assert/strict';
import { chromium } from '@playwright/test';
import { writeFile } from 'node:fs/promises';

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
const page = await context.newPage();
context.setDefaultTimeout(20000);
const errors = [];
page.on('pageerror', (error) => errors.push(error.message));
const base = process.env.COMPUTY_TEST_BASE_URL || 'http://127.0.0.1:1420';
async function call(method, params = {}) {
  const response = await context.request.post(base + '/api/computy', {
    headers: { 'x-computy-client': 'local-ui' },
    data: { method, params },
  });
  const data = await response.json();
  if (!response.ok() || data.error) throw new Error(data.error || 'RPC failed');
  return data.result;
}
async function finished(id) {
  for (let n = 0; n < 150; n++) {
    const job = await call('job', { id });
    if (!job.running) {
      assert.equal(job.error, null);
      return job.report;
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error('Diagnostic timeout');
}
async function capture(name, locator = page) {
  for (const theme of ['light', 'dark']) {
    await page.emulateMedia({ colorScheme: theme });
    await page.waitForFunction((value) => document.documentElement.dataset.theme === value, theme);
    await locator.screenshot({
      path: `docs/screenshots/${name}-${theme}.png`,
      ...(locator === page ? { fullPage: name !== 'processes-live' } : {}),
    });
  }
  await page.emulateMedia({ colorScheme: 'light' });
}
try {
  const preferences = await call('preferences');
  await call('save_preferences', {
    ...preferences,
    theme: 'system',
    language: 'fr',
    tray_enabled: true,
    tray_components: ['memory', 'cpu', 'gpu'],
    tray_disks: [],
  });
  const options = {
    components: ['memory', 'cpu'],
    memory_mib: 16,
    passes: 1,
    disk_mib: 32,
    cpu_seconds: 1,
    label: `Relance · configuration conservée ${Date.now()}`,
  };
  const original = await finished((await call('start', options)).id);
  await page.goto(base + '/#history');
  await page.waitForFunction(
    () =>
      !!document.querySelector('.main-footer b') &&
      !document.querySelector('.connection') &&
      !document.querySelector('.engine-error'),
  );
  const row = page.locator('.history-table tbody tr').filter({ hasText: options.label }).first();
  await row.locator('td').nth(2).click();
  await page.getByRole('dialog').waitFor();
  await capture('history-rerun-report');
  const launches = [];
  page.on('request', (request) => {
    if (request.url().endsWith('/api/computy') && request.postDataJSON()?.method === 'start')
      launches.push(request.postDataJSON().params);
  });
  const firstRerun = page.waitForResponse(
    (r) => r.url().endsWith('/api/computy') && r.request().postDataJSON()?.method === 'start',
  );
  await page.getByRole('dialog').getByRole('button', { name: 'Relancer', exact: true }).click();
  await finished((await (await firstRerun).json()).result.id);
  await page.getByText('Diagnostic terminé et enregistré.', { exact: true }).waitFor();
  assert.deepEqual(launches[0], original.options);
  let history = (await call('history')).reports;
  const second = history.find(
    (report) => report.id !== original.id && report.label === original.label,
  );
  assert.ok(second);
  assert.deepEqual(second.options, original.options);
  assert.ok(history.some((report) => report.id === original.id));
  await page.goto(base + '/#history');
  const nextRow = page
    .locator('.history-table tbody tr')
    .filter({ hasText: options.label })
    .first();
  await nextRow.locator('input[type=checkbox]').check();
  assert.equal(await page.getByRole('dialog').count(), 0);
  await capture('history-rerun-actions');
  const secondRerun = page.waitForResponse(
    (r) => r.url().endsWith('/api/computy') && r.request().postDataJSON()?.method === 'start',
  );
  await nextRow
    .getByRole('button', { name: `Relancer le diagnostic : ${original.label}`, exact: true })
    .click();
  await finished((await (await secondRerun).json()).result.id);
  await page.getByText('Diagnostic terminé et enregistré.', { exact: true }).waitFor();
  assert.deepEqual(launches[1], original.options);
  assert.equal((await call('history')).reports.filter((r) => r.label === original.label).length, 3);
  console.log('Verified two real reruns and preserved history.');

  await page.goto(base + '/#settings');
  const monitor = page.locator('.monitor-settings');
  await monitor.getByRole('switch').waitFor();
  assert.equal(await monitor.getByRole('switch').getAttribute('aria-checked'), 'true');
  assert.equal(await monitor.locator('.monitor-metrics input:checked').count(), 3);
  await monitor.locator('.metric-visual').first().waitFor();
  await page.waitForTimeout(2200);
  await capture('monitor-settings', monitor);
  await monitor.getByLabel('Indicateur : Processeur graphique', { exact: true }).uncheck();
  await page.getByRole('button', { name: 'Enregistrer les préférences', exact: true }).click();
  assert.deepEqual((await call('preferences')).tray_components, ['memory', 'cpu']);
  await monitor.getByRole('switch').click();
  await page.getByRole('button', { name: 'Enregistrer les préférences', exact: true }).click();
  assert.equal((await call('preferences')).tray_enabled, false);
  await page.reload();
  await page.waitForFunction(
    () =>
      document.querySelector('.monitor-settings [role=switch]')?.getAttribute('aria-checked') ===
      'false',
  );
  assert.equal(await monitor.getByRole('switch').getAttribute('aria-checked'), 'false');
  await page.goto(base + '/#processes?metric=cpu');
  await page.locator('.process-table tbody tr').first().waitFor();
  assert.equal(await page.getByLabel('Trier les processus par').inputValue(), 'cpu');
  const snapshot = await call('monitor');
  assert.ok(snapshot.processes.length > 10);
  assert.ok((await page.locator('.process-table tbody tr').count()) > 10);
  await capture('processes-live');
  console.log('Verified monitor settings and the complete process list.');
  await page.getByLabel('Rechercher un processus').fill('COMPUTY-NO-SUCH-PROCESS-XYZ');
  await page.getByText('Aucun processus ne correspond à la recherche.', { exact: true }).waitFor();
  await page.getByLabel('Rechercher un processus').fill('');
  await page.getByRole('button', { name: 'Actualiser les informations', exact: true }).click();
  await page.getByText('Informations matérielles actualisées.', { exact: true }).waitFor();
  await page.route('**/api/computy', async (route) =>
    route.request().postDataJSON().method === 'inventory'
      ? route.fulfill({ status: 500, json: { error: 'Inventory capability fixture' } })
      : route.continue(),
  );
  await page.getByRole('button', { name: 'Actualiser les informations', exact: true }).click();
  await page
    .getByText('Impossible d’actualiser les informations matérielles.', { exact: true })
    .waitFor();
  await page.unroute('**/api/computy');
  console.log('Verified hardware refresh feedback.');

  // Capability fixtures exercise USB loading and Secure Boot guidance only.
  // No USB write, firmware change, boot preparation or reboot is invoked.
  const desktop = await browser.newContext({
    viewport: { width: 1440, height: 1000 },
    locale: 'en-US',
    reducedMotion: 'reduce',
  });
  desktop.setDefaultTimeout(20000);
  await desktop.addInitScript(() => {
    window.__usbMode = 0;
    window.__TAURI_INTERNALS__ = {
      invoke: async (command, args) => {
        if (command === 'engine_call') {
          if (args.method === 'offline_status')
            return {
              platform: 'linux',
              pending: false,
              linux: { code: 'secure_boot', facts: { secure_boot: true, payload_installed: true } },
            };
          const response = await fetch('/api/computy', {
            method: 'POST',
            headers: { 'content-type': 'application/json', 'x-computy-client': 'local-ui' },
            body: JSON.stringify(args),
          });
          const data = await response.json();
          if (data.error) throw new Error(data.error);
          return data.result;
        }
        if (command === 'usb_targets') {
          await new Promise((resolve) => setTimeout(resolve, 650));
          if (window.__usbMode === -1) throw new Error('boot:usb_not_found');
          return window.__usbMode
            ? [
                {
                  id: 'fixture-only',
                  label: 'USB capability fixture',
                  mount: '/media/fixture',
                  size_bytes: 1024 ** 3,
                },
              ]
            : [];
        }
        if (command === 'startup_status') return false;
        throw new Error('Unexpected native mutation: ' + command);
      },
    };
  });
  const usb = await desktop.newPage();
  usb.on('pageerror', (e) => errors.push(e.message));
  await usb.goto(base + '/#diagnostics');
  await usb.getByRole('button', { name: 'Avant démarrage', exact: false }).click();
  await usb.getByText('Test avant démarrage bloqué par Secure Boot', { exact: true }).waitFor();
  await usb.getByText('Comment débloquer le test avant démarrage ?', { exact: true }).click();
  await usb.getByText('Continuer avec un test dans ma session', { exact: true }).waitFor();
  assert.ok((await usb.locator('.secure-boot-help').innerText()).includes('BitLocker'));
  await usb.getByRole('button', { name: 'Avec clé USB', exact: true }).click();
  await usb.getByRole('button', { name: 'Rechercher les clés compatibles', exact: true }).click();
  const searching = usb.getByRole('button', { name: 'Recherche des clés USB…', exact: true });
  assert.equal(await searching.getAttribute('aria-busy'), 'true');
  assert.equal(await searching.locator('.spinner,.spin').count(), 1);
  await usb
    .getByText(
      'Aucune clé USB compatible trouvée. Branchez et montez une clé FAT32, puis réessayez.',
      { exact: true },
    )
    .waitFor();
  await usb.evaluate(() => (window.__usbMode = 1));
  await usb.getByRole('button', { name: 'Rechercher les clés compatibles', exact: true }).click();
  await usb.getByText('Clés USB compatibles trouvées : 1', { exact: true }).waitFor();
  await usb.evaluate(() => (window.__usbMode = -1));
  await usb.getByRole('button', { name: 'Rechercher les clés compatibles', exact: true }).click();
  await usb.locator('.toast.error').waitFor();
  await usb
    .getByRole('button', { name: 'Continuer avec un test dans ma session', exact: true })
    .click();
  await usb.getByText('01. Que souhaitez-vous vérifier ?', { exact: true }).waitFor();
  assert.deepEqual(errors, []);
  await writeFile(
    'docs/screenshots/history-monitor-results.json',
    JSON.stringify(
      {
        real_reruns: 2,
        original_preserved: true,
        exact_options_preserved: true,
        whole_row_opens_report: true,
        checkboxes_do_not_open_report: true,
        monitor_preferences_persisted: true,
        all_accessible_processes: true,
        usb_spinner_and_result_toasts: true,
        secure_boot_guidance: true,
        errors,
      },
      null,
      2,
    ) + '\n',
  );
  await desktop.close();
} finally {
  await browser.close();
}
console.log(
  'History reruns, live monitoring, processes, action feedback and boot guidance verified.',
);
