// Real Linux Tauri/WebKit smoke test. Run under Xvfb, with tauri-driver installed.
import assert from 'node:assert/strict';
import { spawn, execFile } from 'node:child_process';
import { promisify } from 'node:util';
const exec = promisify(execFile);
import { mkdtemp, mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
const root = await mkdtemp(path.join(tmpdir(), 'computy-desktop-'));
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
const watcher = spawn('python3', ['scripts/mock-status-watcher.py'], {
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
        },
      },
    },
  });
  session = created.sessionId;
  assert.ok(session);
  await until("return document.body.innerText.includes('Moteur Rust connecté')");
  assert.equal(
    await execute("return document.querySelector('.main-footer b')?.textContent"),
    expectedVersion,
    'The packaged interface must display the current application version',
  );
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
  const native = async (command, args = {}) => {
    const envelope = await request('POST', `/session/${session}/execute/async`, {
      script:
        'const done=arguments[arguments.length-1];window.__TAURI_INTERNALS__.invoke(arguments[0],arguments[1]).then(value=>done({value})).catch(error=>done({error:String(error)}));',
      args: [command, args],
    });
    if (envelope.error) throw Error(envelope.error);
    return envelope.value;
  };
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
  await click('Préférences');
  await until("return document.querySelectorAll('.monitor-metrics input:checked').length===3");
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
  assert.ok(
    (
      await dbus(memory, 'org.freedesktop.DBus.Properties.Get', [
        'org.kde.StatusNotifierItem',
        'Status',
      ])
    ).includes('Active'),
  );
  await native('monitor_panel_action', { destination: 'overview' });
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
  await click('Préférences');
  await until("return !!document.querySelector('.maintenance-settings')");
  await execute("document.querySelector('.maintenance-settings').scrollIntoView({block:'center'})");
  await capture('maintenance-dark', false);
  await click('Clair');
  await click('Enregistrer les préférences');
  await until("return document.documentElement.dataset.theme==='light'");
  await execute("document.querySelector('.maintenance-settings').scrollIntoView({block:'center'})");
  await capture('maintenance-light', false);
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
    dbus_only_desktop_entry_survives: true,
    custom_popover: true,
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
    'Native Tauri, maintenance consent, tray toggles, quit actions and 10 real WebKit captures verified.',
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
}
