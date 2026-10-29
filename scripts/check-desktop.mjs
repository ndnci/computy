// Real Linux Tauri/WebKit smoke test. Run under Xvfb, with tauri-driver installed.
import assert from 'node:assert/strict';
import { spawn, execFile } from 'node:child_process';
import { promisify } from 'node:util';
const exec = promisify(execFile);
import { access, mkdtemp, mkdir, readdir, readFile, rm, writeFile, utimes } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
// This suite changes a login entry and sends native window events: container only.
await access('/.dockerenv');
const startupFile = path.join(process.env.HOME, '.config/autostart/Computy.desktop');
await assert.rejects(access(startupFile), 'Never overwrite an existing startup entry in a test');
const root = await mkdtemp(path.join(tmpdir(), 'computy-desktop-'));
const cleanupFixture = await mkdtemp(path.join(tmpdir(), 'computy-clean-fixture-'));
const destination = process.env.COMPUTY_SCREENSHOT_DIR || 'docs/screenshots';
const expectedVersion = JSON.parse(await readFile('package.json', 'utf8')).version;
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
let watcher = spawn('python3', ['scripts/mock-status-watcher.py'], {
  stdio: ['ignore', 'pipe', 'pipe'],
});
let watcherReady = false;
watcher.stdout.on('data', (data) => {
  if (data.toString().includes('WATCHER_READY')) watcherReady = true;
});
const driver = spawn('tauri-driver', ['--port', '4444', '--native-port', '4445'], {
  env: { ...process.env, COMPUTY_DATA_DIR: root, XDG_DATA_HOME: root },
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
const native = async (command, args = {}) => {
  const envelope = await request('POST', `/session/${session}/execute/async`, {
    script:
      'const done=arguments[arguments.length-1];window.__TAURI_INTERNALS__.invoke(arguments[0],arguments[1]).then(value=>done({value})).catch(error=>done({error:String(error)}));',
    args: [command, args],
  });
  if (envelope.error) throw Error(envelope.error);
  return envelope.value;
};
async function until(script) {
  const deadline = Date.now() + 30000;
  while (Date.now() < deadline) {
    if (await execute(script)) return;
    await new Promise((resolve) => setTimeout(resolve, 150));
  }
  throw new Error(`Timed out: ${script}\n${await execute('return document.body.innerText')}`);
}
async function click(text) {
  await until(
    "return [...document.querySelectorAll('button')].some(item=>item.textContent.trim()===" +
      JSON.stringify(text) +
      ' && !item.disabled)',
  );
  assert.equal(
    await execute(
      "const button=[...document.querySelectorAll('button')].find(item=>item.textContent.trim()===arguments[0]);if(button){button.click();return true;}return false;",
      [text],
    ),
    true,
    `Missing button: ${text}`,
  );
}
async function capture(name, scroll = true) {
  if (scroll) await execute('window.scrollTo(0,0)');
  // WebKit may expose updated DOM before its native screenshot surface is painted.
  await request('POST', `/session/${session}/execute/async`, {
    script:
      'const done=arguments[arguments.length-1];requestAnimationFrame(()=>requestAnimationFrame(()=>setTimeout(done,150)));',
    args: [],
  });
  const png = await request('GET', `/session/${session}/screenshot`);
  await writeFile(`${destination}/native-linux-${name}.png`, Buffer.from(png, 'base64'));
}
try {
  for (let n = 0; n < 100 && !watcherReady; n++)
    await new Promise((resolve) => setTimeout(resolve, 50));
  assert.ok(watcherReady, 'private StatusNotifier watcher must be running');
  await mkdir(path.join(root, 'applications'), { recursive: true });
  await writeFile(
    path.join(root, 'applications/computy.desktop'),
    `[Desktop Entry]\nType=Application\nName=Computy\nExec=${path.resolve(process.env.COMPUTY_DESKTOP_BINARY || 'release/computy')}\nIcon=${path.resolve('public/computy-logo.png')}\n`,
  );
  // Valid D-Bus-only launchers may have no Exec key (seen on Ubuntu).
  await writeFile(
    path.join(root, 'applications/org.ndnci.ComputyNoExecutable.desktop'),
    '[Desktop Entry]\nType=Application\nName=Computy no-executable fixture\nDBusActivatable=true\nNoDisplay=true\nIcon=application-x-executable\n',
  );
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
          args: process.env.COMPUTY_TEST_BACKGROUND === '1' ? ['--background'] : [],
        },
      },
    },
  });
  session = created.sessionId;
  assert.ok(session);
  await until("return typeof window.__TAURI_INTERNALS__?.invoke==='function'");
  assert.equal(
    await native('plugin:window|is_visible', { label: 'main' }),
    process.env.COMPUTY_TEST_BACKGROUND !== '1',
  );
  if (process.env.COMPUTY_TEST_BACKGROUND === '1') {
    await trayCount(3);
    await native('monitor_panel_action', { destination: 'overview' });
    assert.equal(await native('plugin:window|is_visible', { label: 'main' }), true);
  }

  await request('POST', `/session/${session}/window/rect`, { width: 1440, height: 1000 });
  await until(
    "return !!document.querySelector('.main-footer b') && !document.querySelector('.connection') && !document.querySelector('.engine-error')",
  );
  assert.equal(
    await execute("return document.querySelector('.main-footer b')?.textContent"),
    expectedVersion,
    'The packaged interface must display the current application version',
  );
  assert.equal(await execute('return typeof window.__TAURI_INTERNALS__?.invoke'), 'function');
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
  async function trayCount(expected) {
    for (let n = 0; n < 40; n++) {
      const state = await native('monitor_tray_status');
      if (state.active === expected) return state;
      await new Promise((resolve) => setTimeout(resolve, 150));
    }
    throw Error('Wrong native tray count');
  }
  async function dbus(item, method, args = []) {
    return (
      await exec('gdbus', [
        'call',
        '--session',
        '--dest',
        item.bus,
        '--object-path',
        item.path,
        '--method',
        method,
        ...args,
      ])
    ).stdout;
  }
  await trayCount(3);
  const watcherCall = async (method) =>
    (
      await exec('gdbus', [
        'call',
        '--session',
        '--dest',
        'org.kde.StatusNotifierWatcher',
        '--object-path',
        '/StatusNotifierWatcher',
        '--method',
        `com.ndnci.TestWatcher.${method}`,
      ])
    ).stdout;
  assert.equal(
    (await watcherCall('SimulateDesktopRediscovery')).trim(),
    '(3,)',
    'GNOME rediscovery must retain three distinct indicators',
  );
  await watcherCall('ForgetRegistrations');
  let recovered = false;
  for (let n = 0; n < 40; n++) {
    if ((await watcherCall('Count')).trim() === '(3,)') {
      recovered = true;
      break;
    }
    await new Promise((resolve) => setTimeout(resolve, 150));
  }
  assert.ok(recovered, 'the same watcher owner must recover forgotten registrations');
  await click('Préférences');
  await until("return document.querySelectorAll('.monitor-metrics input:checked').length===3");
  const startupSwitch = "button[aria-label='Afficher les indicateurs à la connexion']";
  // Fresh installations opt into login monitoring without opening the main window.
  await until(
    `return document.querySelector(${JSON.stringify(startupSwitch)}).getAttribute('aria-checked')==='true'`,
  );
  assert.equal(await native('startup_status'), true);
  assert.ok((await readFile(startupFile, 'utf8')).includes('--background'));
  await execute(
    'document.querySelector(arguments[0]).closest(".panel").scrollIntoView({block:"center"})',
    [startupSwitch],
  );
  await capture('startup-light', false);
  await capture('monitor-light');
  for (let n = 0; n < 3; n++) {
    await execute("document.querySelector('.monitor-metrics input').click()");
    await click('Enregistrer les préférences');
    let state = await trayCount(2);
    const memory = state.items.find((item) => item.id === 'computy-memory');
    assert.ok(
      (
        await dbus(memory, 'org.freedesktop.DBus.Properties.Get', [
          'org.kde.StatusNotifierItem',
          'Status',
        ])
      ).includes('Passive'),
    );
    await execute("document.querySelector('.monitor-metrics input').click()");
    await click('Enregistrer les préférences');
    state = await trayCount(3);
    const enabled = state.items.find((item) => item.id === 'computy-memory');
    assert.equal(memory.path, enabled.path);
    assert.ok(
      (
        await dbus(enabled, 'org.freedesktop.DBus.Properties.Get', [
          'org.kde.StatusNotifierItem',
          'Status',
        ])
      ).includes('Active'),
    );
  }
  for (let n = 0; n < 2; n++) {
    await execute("document.querySelector('.monitor-settings [role=switch]').click()");
    await click('Enregistrer les préférences');
    await trayCount(0);
    await execute("document.querySelector('.monitor-settings [role=switch]').click()");
    await click('Enregistrer les préférences');
    await trayCount(3);
  }
  const snapshot = await native('engine_call', { method: 'monitor', params: {} });
  const appProcess = snapshot.processes.find((p) => p.name === 'computy');
  assert.ok(appProcess);
  const appIcons = await native('process_icons', { pids: [appProcess.pid] });
  assert.ok(appIcons[appProcess.pid]?.startsWith('data:image/png;base64,'));
  const state = await trayCount(3);
  const memory = state.items.find((item) => item.id === 'computy-memory');
  const menuPath = memory.path + '/Menu';
  await exec('python3', ['scripts/close-test-window.py', String(appProcess.pid)], {
    env: { ...process.env, COMPUTY_TEST_X11: '1' },
  });
  for (let n = 0; n < 30; n++) {
    if (!(await native('plugin:window|is_visible', { label: 'main' }))) break;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  assert.equal(await native('plugin:window|is_visible', { label: 'main' }), false);
  process.kill(appProcess.pid, 0);
  await trayCount(3);
  const watcherExited = new Promise((resolve) => watcher.once('exit', resolve));
  watcher.kill('SIGTERM');
  await watcherExited;
  await trayCount(0);
  assert.equal(
    await native('plugin:window|is_visible', { label: 'main' }),
    false,
    'a temporary tray host loss must not open the main window',
  );
  watcherReady = false;
  watcher = spawn('python3', ['scripts/mock-status-watcher.py'], {
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  watcher.stdout.on('data', (data) => {
    if (data.toString().includes('WATCHER_READY')) watcherReady = true;
  });
  for (let n = 0; n < 100 && !watcherReady; n++)
    await new Promise((resolve) => setTimeout(resolve, 50));
  assert.ok(watcherReady);
  await trayCount(3);
  assert.ok(
    (
      await dbus(memory, 'org.freedesktop.DBus.Properties.Get', [
        'org.kde.StatusNotifierItem',
        'Status',
      ])
    ).includes('Active'),
  );
  const binary = path.resolve(process.env.COMPUTY_DESKTOP_BINARY || 'release/computy');
  const launchEnv = { ...process.env, COMPUTY_DATA_DIR: root, XDG_DATA_HOME: root };
  await exec(binary, ['--background'], { env: launchEnv, timeout: 10000 });
  assert.equal(await native('plugin:window|is_visible', { label: 'main' }), false);
  await exec(binary, [], { env: launchEnv, timeout: 10000 });
  assert.equal(await native('plugin:window|is_visible', { label: 'main' }), true);
  const layout = (
    await exec('gdbus', [
      'call',
      '--session',
      '--dest',
      memory.bus,
      '--object-path',
      menuPath,
      '--method',
      'com.canonical.dbusmenu.GetLayout',
      '0',
      '1',
      '[]',
    ])
  ).stdout;
  assert.ok(layout.includes('icon-data') && layout.includes('icon-name'));
  const mainHandle = await request('GET', `/session/${session}/window`);
  // The protocol unit test covers Activate. Exercise the GNOME menu path here.
  await exec('gdbus', [
    'call',
    '--session',
    '--dest',
    memory.bus,
    '--object-path',
    menuPath,
    '--method',
    'com.canonical.dbusmenu.Event',
    '1',
    'clicked',
    '<0>',
    '0',
  ]);
  let popupHandle;
  for (let n = 0; n < 40 && !popupHandle; n++) {
    const handles = await request('GET', `/session/${session}/window/handles`);
    for (const handle of handles.filter((handle) => handle !== mainHandle)) {
      await request('POST', `/session/${session}/window`, { handle });
      if (await execute("return location.search.includes('popover')")) {
        popupHandle = handle;
        break;
      }
    }
    if (!popupHandle) await new Promise((resolve) => setTimeout(resolve, 100));
  }
  assert.ok(popupHandle);
  await until("return document.querySelectorAll('.popover-process').length>0");
  assert.equal(
    await execute("return !!document.querySelector('.metric-visual-ring')"),
    true,
    'Popover must keep its ring independently of the system-bar style',
  );
  await capture('tray-panel');
  await click('Voir tout');
  await request('POST', `/session/${session}/window`, { handle: mainHandle });
  await until("return !!document.querySelector('.process-table')");
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
  // Actual filesystem fixture in this container's private temporary directory.
  const cache = cleanupFixture;
  await writeFile(path.join(cache, 'old.log'), 'temporary fixture');
  await utimes(path.join(cache, 'old.log'), new Date(0), new Date(0));
  await click('Nettoyage');
  await until("return !!document.querySelector('.clean-page textarea')");
  await execute(
    "const el=document.querySelector('.clean-page textarea');Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype,'value').set.call(el,arguments[0]);el.dispatchEvent(new Event('input',{bubbles:true}));",
    [cache],
  );
  await click('Analyser sans supprimer');
  await until("return !!document.querySelector('.clean-summary')");
  assert.ok((await readFile(path.join(cache, 'old.log'), 'utf8')).includes('temporary'));
  assert.equal(
    await execute("return document.querySelectorAll('.utility-table tbody tr').length"),
    1,
    'The dry run must contain the actual fixture',
  );
  await capture('clean-rules-light');
  await execute(
    "document.querySelector('.clean-summary').closest('section').scrollIntoView({block:'start'})",
  );
  await capture('clean-light', false);
  const sidebarBefore = await execute(
    "const r=document.querySelector('.sidebar').getBoundingClientRect();return {x:r.x,y:r.y,width:r.width}",
  );
  await execute('window.scrollBy(0,100)');
  assert.deepEqual(
    await execute(
      "const r=document.querySelector('.sidebar').getBoundingClientRect();return {x:r.x,y:r.y,width:r.width}",
    ),
    sidebarBefore,
    'Sidebar must stay fixed while the main page scrolls',
  );
  await click('Automatiser cette règle');
  await click('Autoriser cette automatisation');
  await until(
    "return !!document.querySelector('.schedule-row') && !document.querySelector('.modal-backdrop')",
  );
  await click('Vérifier et nettoyer');
  await click('Confirmer la suppression');
  await until("return !document.querySelector('.clean-summary')");
  await assert.rejects(access(path.join(cache, 'old.log')));
  await click('Applications');
  await until(
    "return !!document.querySelector('.applications-page') && !document.querySelector('.applications-page .utility-empty .spin')",
  );
  if (!(await execute("return !!document.querySelector('.application-card')"))) {
    await execute("document.querySelector('.applications-page input[type=checkbox]').click()");
    await until("return !!document.querySelector('.application-card')");
  }
  await capture('applications-light');
  await click('Ventilateurs');
  await until("return !!document.querySelector('.utility-page')");
  await capture('fans-light');
  await click('Préférences');
  await until("return !!document.querySelector('.settings-page')");
  await click('Sombre');
  await click('Enregistrer les préférences');
  await until("return document.documentElement.dataset.theme==='dark'");
  await until(
    `return document.querySelector(${JSON.stringify(startupSwitch)}).getAttribute('aria-checked')==='true'`,
  );
  await execute(
    'document.querySelector(arguments[0]).closest(".panel").scrollIntoView({block:"center"})',
    [startupSwitch],
  );
  await capture('startup-dark', false);
  await click('Nettoyage');
  await until("return !!document.querySelector('.clean-page')");
  await writeFile(path.join(cache, 'old.log'), 'temporary fixture');
  await utimes(path.join(cache, 'old.log'), new Date(0), new Date(0));
  await execute(
    "const el=document.querySelector('.clean-page textarea');Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype,'value').set.call(el,arguments[0]);el.dispatchEvent(new Event('input',{bubbles:true}));",
    [cache],
  );
  await click('Analyser sans supprimer');
  await until("return !!document.querySelector('.clean-summary')");
  await capture('clean-rules-dark');
  await execute(
    "document.querySelector('.clean-summary').closest('section').scrollIntoView({block:'start'})",
  );
  await capture('clean-dark', false);
  await click('Applications');
  await until(
    "return !!document.querySelector('.application-card') || !!document.querySelector('.applications-page .utility-empty p')",
  );
  await execute(
    "const el=document.querySelector('.applications-page input[type=checkbox]');if(el&&!el.checked)el.click()",
  );
  await capture('applications-dark');
  await click('Ventilateurs');
  await until("return !!document.querySelector('.utility-page')");
  await capture('fans-dark');
  await click('Préférences');
  await until("return !!document.querySelector('.settings-page')");

  await execute('document.querySelector(arguments[0]).click()', [startupSwitch]);
  await until(
    `return document.querySelector(${JSON.stringify(startupSwitch)}).getAttribute('aria-checked')==='false'`,
  );
  assert.equal(await native('startup_status'), false);
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
  await click('Préférences');
  await until("return !!document.querySelector('.maintenance-settings')");
  await execute("document.querySelector('.maintenance-settings').scrollIntoView({block:'center'})");
  await capture('maintenance-dark', false);
  await click('Clair');
  await click('Enregistrer les préférences');
  await until("return document.documentElement.dataset.theme==='light'");
  await execute("document.querySelector('.maintenance-settings').scrollIntoView({block:'center'})");
  await capture('maintenance-light', false);
  assert.ok(
    await execute(
      "const save=[...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Enregistrer les préférences').getBoundingClientRect();const panel=document.querySelector('.maintenance-settings').getBoundingClientRect();return panel.top-save.bottom>=23;",
    ),
    'Save actions must be separated from the maintenance panel',
  );
  await click('Supprimer toutes les données');
  await until("return !!document.querySelector('[role=dialog]')");
  assert.equal(
    await execute("return document.querySelector('[role=dialog] .danger-button').disabled"),
    true,
  );
  await click('Annuler');
  await assert.rejects(native('maintenance_action', { action: 'erase', confirmed: false }));
  await assert.rejects(native('maintenance_action', { action: 'uninstall', confirmed: false }));
  assert.equal(
    (await readdir(path.join(root, 'reports'))).filter((f) => f.endsWith('.json')).length,
    2,
  );
  const quitMethod = process.env.COMPUTY_TEST_QUIT_METHOD || 'dock';
  if (quitMethod === 'dock') {
    await writeFile(path.join(root, 'export-sentinel.json'), 'user export');
    await click('Supprimer toutes les données');
    await execute("document.querySelector('[role=dialog] input[type=checkbox]').click()");
    await execute("document.querySelector('[role=dialog] .danger-button').click()");
    for (let n = 0; n < 100; n++) {
      if (!(await readdir(path.join(root, 'reports'))).some((f) => f.endsWith('.json'))) break;
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    assert.equal(
      (await readdir(path.join(root, 'reports'))).filter((f) => f.endsWith('.json')).length,
      0,
    );
    assert.equal(await readFile(path.join(root, 'export-sentinel.json'), 'utf8'), 'user export');
    await until(
      "return !!document.querySelector('.main-footer b') && !document.querySelector('[role=dialog]')",
    );
    assert.equal(
      (await native('engine_call', { method: 'preferences', params: {} })).language,
      'system',
    );
    await trayCount(3);
    const actions = [
      'call',
      '--session',
      '--dest',
      'com.ndnci.computy',
      '--object-path',
      '/com/ndnci/computy',
      '--method',
    ];
    assert.ok((await exec('gdbus', [...actions, 'org.gtk.Actions.List'])).stdout.includes('quit'));
    await exec('gdbus', [...actions, 'org.gtk.Actions.Activate', 'quit', '[]', '{}']);
  } else if (quitMethod === 'tray') {
    await trayCount(3);
    await exec('gdbus', [
      'call',
      '--session',
      '--dest',
      memory.bus,
      '--object-path',
      menuPath,
      '--method',
      'com.canonical.dbusmenu.Event',
      '33',
      'clicked',
      '<0>',
      '0',
    ]);
  } else if (quitMethod === 'button') {
    await execute("document.querySelector('.quit-application').click()");
  } else if (quitMethod === 'disabled') {
    await execute("document.querySelector('.monitor-settings [role=switch]').click()");
    await click('Enregistrer les préférences');
    await trayCount(0);
    await native('monitor_panel_action', { destination: 'quit' }).catch(() => {});
  } else throw Error('Unknown quit test method');
  let exited = false;
  for (let n = 0; n < 60; n++) {
    try {
      process.kill(appProcess.pid, 0);
      const stat = await readFile(`/proc/${appProcess.pid}/stat`, 'utf8').catch(() => '');
      if (!stat || ['Z', 'X'].includes(stat.slice(stat.lastIndexOf(')') + 2).split(' ')[0])) {
        exited = true;
        break;
      }
    } catch (error) {
      if (error.code === 'ESRCH') {
        exited = true;
        break;
      }
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  assert.ok(exited, `quitting via ${quitMethod} must exit the app`);
  const result = {
    version: expectedVersion,
    native: 'Tauri / WebKitGTK',
    actual_diagnostic: true,
    actual_tray_registrations: true,
    tray_disable_verified: true,
    repeated_ram_toggles: 3,
    repeated_global_toggles: 2,
    real_dbus_status_and_activation: true,
    real_dbus_menu_activation: true,
    real_application_icon: true,
    gnome_rediscovery_verified: true,
    same_owner_watcher_recovery: true,
    tray_host_restart_keeps_window_hidden: true,
    background_start_hidden: process.env.COMPUTY_TEST_BACKGROUND === '1',
    login_entry_uses_background: true,
    background_relaunch_does_not_open_window: true,
    manual_relaunch_opens_existing_window: true,
    maintenance_spacing_verified: true,
    dbus_only_desktop_entry_survives: true,
    custom_popover: true,
    popover_keeps_ring: true,
    login_enabled_by_default: true,
    cleanup_dry_run_and_confirmation: true,
    cleanup_schedule_created: true,
    sidebar_stays_fixed: true,
    applications_inventory: true,
    fan_capabilities: true,
    quit_method: quitMethod,
    quit_exits: true,
    maintenance_consent_verified: true,
    maintenance_erasure_verified: quitMethod === 'dock',
    actual_process_telemetry: true,
    native_window_close_keeps_monitoring: true,
    reopening_keeps_same_process: true,
    themes: ['light', 'dark'],
    errors: [],
  };
  await writeFile(`${destination}/native-results.json`, JSON.stringify(result, null, 2) + '\n');
  console.log(
    'Native Tauri, maintenance consent, tray toggles, quit actions and maintenance WebKit captures verified.',
  );
  console.log(JSON.stringify(result));
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
  watcher.kill('SIGTERM');
  await rm(root, { recursive: true, force: true });
  await rm(cleanupFixture, { recursive: true, force: true });
}
