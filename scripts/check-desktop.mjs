// Real Linux Tauri/WebKit smoke test. Run under Xvfb, with tauri-driver installed.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
const root = await mkdtemp(path.join(tmpdir(), 'computy-desktop-'));
const destination = 'docs/screenshots';
await mkdir(destination, { recursive: true });
await writeFile(
  path.join(root, 'preferences.state'),
  JSON.stringify({
    language: 'fr',
    theme: 'light',
    memory_mib: 16,
    passes: 1,
    disk_mib: 16,
    cpu_seconds: 1,
  }),
);
const driver = spawn('tauri-driver', ['--port', '4444', '--native-port', '4445'], {
  env: { ...process.env, COMPUTY_DATA_DIR: root },
  stdio: ['ignore', 'pipe', 'pipe'],
});
let logs = '';
driver.stdout.on('data', (data) => (logs += data));
driver.stderr.on('data', (data) => (logs += data));
let session;
async function request(method, endpoint, body) {
  const response = await fetch(`http://127.0.0.1:4444${endpoint}`, {
    method,
    headers: { 'content-type': 'application/json' },
    body: body ? JSON.stringify(body) : undefined,
    signal: AbortSignal.timeout(60000),
  });
  const result = await response.json();
  if (!response.ok || result.value?.error) throw new Error(JSON.stringify(result));
  return result.value;
}
const execute = (script, args = []) =>
  request('POST', `/session/${session}/execute/sync`, { script, args });
async function until(script) {
  const deadline = Date.now() + 30000;
  while (Date.now() < deadline) {
    if (await execute(script)) return;
    await new Promise((resolve) => setTimeout(resolve, 150));
  }
  throw new Error(`Timed out: ${script}\n${await execute('return document.body.innerText')}`);
}
async function click(text) {
  assert.equal(
    await execute(
      "const button=[...document.querySelectorAll('button')].find(item=>item.textContent.trim()===arguments[0]);if(button){button.click();return true;}return false;",
      [text],
    ),
    true,
    `Missing button: ${text}`,
  );
}
async function capture(name) {
  await execute('window.scrollTo(0,0)');
  const png = await request('GET', `/session/${session}/screenshot`);
  await writeFile(`${destination}/native-linux-${name}.png`, Buffer.from(png, 'base64'));
}
try {
  for (let n = 0; n < 100; n++) {
    try {
      await request('GET', '/status');
      break;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
  }
  const created = await request('POST', '/session', {
    capabilities: {
      alwaysMatch: {
        'tauri:options': {
          application: path.resolve(process.env.COMPUTY_DESKTOP_BINARY || 'release/computy'),
        },
      },
    },
  });
  session = created.sessionId;
  assert.ok(session);
  await until("return document.body.innerText.includes('Moteur Rust connecté')");
  assert.equal(await execute('return typeof window.__TAURI_INTERNALS__?.invoke'), 'function');
  await request('POST', `/session/${session}/window/rect`, { width: 1440, height: 1000 });
  await execute(
    "window.addEventListener('error',event=>(window.__computyErrors??=[]).push(event.message));window.addEventListener('unhandledrejection',event=>(window.__computyErrors??=[]).push(String(event.reason)));",
  );
  await execute(
    "const style=document.createElement('style');style.textContent='*,*::before,*::after{animation:none!important;transition:none!important}';document.head.append(style);",
  );
  await until("return document.fonts.status==='loaded'");
  await capture('overview-light');
  // Actual Rust telemetry and actual Tauri tray registrations, not browser fixtures.
  await execute(
    "window.__monitorReady=false;window.__TAURI_INTERNALS__.invoke('engine_call',{method:'monitor',params:{}}).then(value=>{window.__monitorReady=value.processes.length>0&&value.memory_percent>0;});",
  );
  await until('return window.__monitorReady===true');
  await execute(
    "window.__traysReady=false;Promise.all(['computy-memory','computy-cpu','computy-gpu'].map(id=>window.__TAURI_INTERNALS__.invoke('plugin:tray|get_by_id',{id}))).then(ids=>{window.__traysReady=ids.every(id=>id!==null);return Promise.all(ids.filter(id=>id!==null).map(rid=>window.__TAURI_INTERNALS__.invoke('plugin:resources|close',{rid})));});",
  );
  await until('return window.__traysReady===true');
  await click('Préférences');
  await until("return document.querySelectorAll('.monitor-metrics input:checked').length===3");
  await capture('monitor-light');
  await execute("document.querySelector('.monitor-settings [role=switch]').click()");
  await click('Enregistrer les préférences');
  await until(
    "return document.querySelector('.monitor-settings [role=switch]').getAttribute('aria-checked')==='false'",
  );
  await new Promise((resolve) => setTimeout(resolve, 2200));
  await execute(
    "window.__traysDisabled=false;window.__TAURI_INTERNALS__.invoke('plugin:tray|get_by_id',{id:'computy-memory'}).then(rid=>{window.__traysDisabled=rid===null;if(rid!==null)return window.__TAURI_INTERNALS__.invoke('plugin:resources|close',{rid});});",
  );
  await until('return window.__traysDisabled===true');
  await execute("document.querySelector('.monitor-settings [role=switch]').click()");
  await click('Enregistrer les préférences');
  await click('Diagnostics');
  await until("return document.querySelectorAll('.test-component-option input').length===3");
  assert.deepEqual(
    await execute(
      "return [...document.querySelectorAll('.test-component-option input')].map(input=>input.checked)",
    ),
    [true, true, true],
  );
  await capture('diagnostics-light');
  await click('Lancer le diagnostic');
  await until("return document.body.innerText.includes('Votre bilan est prêt.')");
  await capture('result-light');
  const files = (await readdir(path.join(root, 'reports'))).filter((file) =>
    file.endsWith('.json'),
  );
  assert.equal(files.length, 1);
  const envelope = JSON.parse(await readFile(path.join(root, 'reports', files[0]), 'utf8'));
  const report = JSON.parse(envelope.payload);
  assert.ok(
    report.checks.find((check) => check.id === 'memory-patterns').metrics.verified_bytes >
      16 * 1024 ** 2,
  );
  assert.equal(
    report.checks.find((check) => check.id === 'disk-roundtrip').metrics.verified_bytes,
    16 * 1024 ** 2,
  );
  assert.ok(report.checks.find((check) => check.id === 'cpu-known-answers').metrics.iterations > 0);
  await click('Préférences');
  await until("return !!document.querySelector('.settings-page')");
  await click('Sombre');
  await click('Enregistrer les préférences');
  await until("return document.documentElement.dataset.theme==='dark'");
  await click('Diagnostics');
  await until("return document.querySelectorAll('.test-component-option input').length===3");
  await click('Lancer le diagnostic');
  await until("return document.body.innerText.includes('Votre bilan est prêt.')");
  await capture('result-dark');
  await click('Nouveau diagnostic');
  await until("return document.querySelectorAll('.test-component-option input').length===3");
  await capture('diagnostics-dark');
  await click('Vue d’ensemble');
  await until(
    "return !document.querySelector('.test-component-option') && document.body.innerText.includes('Votre matériel, en clair.')",
  );
  await capture('overview-dark');
  assert.deepEqual(await execute('return window.__computyErrors||[]'), []);
  const result = {
    native: 'Tauri / WebKitGTK',
    actual_diagnostic: true,
    actual_tray_registrations: true,
    tray_disable_verified: true,
    actual_process_telemetry: true,
    themes: ['light', 'dark'],
    errors: [],
  };
  await writeFile(`${destination}/native-results.json`, JSON.stringify(result, null, 2) + '\n');
  console.log(
    'Native Tauri IPC, tray registrations and disabling, live processes, diagnostics and 7 real WebKit captures verified.',
  );
} catch (error) {
  console.error(logs.slice(-6000));
  throw error;
} finally {
  if (session) await request('DELETE', `/session/${session}`).catch(() => {});
  if (driver.exitCode === null) {
    const stopped = new Promise((resolve) => driver.once('exit', resolve));
    driver.kill('SIGINT');
    await stopped;
  }
  await rm(root, { recursive: true, force: true });
}
