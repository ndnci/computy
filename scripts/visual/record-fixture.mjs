// Records one frozen response per engine method so the React baseline and the GPUI
// interface render identical data. Output stays local (tests/visual is ignored by Git).
import { createRequire } from 'node:module';
import { cp, mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
const require = createRequire(import.meta.url);
const { NativeBridge } = require('../native-bridge.cjs');
const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '../..');
const engine =
  process.env.COMPUTY_ENGINE_PATH ||
  [
    path.join(root, 'native/target/release/computy-cli'),
    path.join(root, 'release/computy-cli'),
  ].find(existsSync);
if (!engine) throw new Error('computy-cli is required (set COMPUTY_ENGINE_PATH).');
const data = await mkdtemp(path.join(os.tmpdir(), 'computy-fixture-'));
if (existsSync(path.join(root, '.computy/reports')))
  await cp(path.join(root, '.computy/reports'), path.join(data, 'reports'), { recursive: true });
const bridge = new NativeBridge({ binary: engine, dataDir: data });
const call = (method, params = {}) => bridge.call(method, params);
const fixture = { now: Date.now(), engine: {}, reports: {}, tauri: {} };
try {
  for (const method of [
    'inventory',
    'history',
    'preferences',
    'offline_status',
    'applications',
    'fans',
    'clean_sources',
    'clean_schedules',
  ])
    fixture.engine[method] = await call(method, method === 'inventory' ? { refresh: true } : {});
  fixture.engine.active = null;
  // Two monitor samples: the first initialises CPU deltas.
  await call('monitor');
  await new Promise((resolve) => setTimeout(resolve, 1200));
  const monitor = await call('monitor');
  monitor.processes = monitor.processes.slice(0, 40);
  fixture.engine.monitor = monitor;
  for (const report of fixture.engine.history.reports)
    fixture.reports[report.id] = await call('report', { id: report.id });
  // A deterministic cleanup preview over a scratch tree.
  const sample = path.join(data, 'cleanup-sample');
  await mkdir(path.join(sample, 'Downloads/archives'), { recursive: true });
  for (const [name, size] of [
    ['Downloads/setup-old.iso', 4_200_000],
    ['Downloads/archives/photos-2021.zip', 2_400_000],
    ['Downloads/notes.tmp', 120_000],
    ['Downloads/video.mkv', 3_100_000],
  ])
    await writeFile(path.join(sample, name), Buffer.alloc(size, 7));
  fixture.engine.clean_preview = await call('clean_preview', {
    roots: [sample],
    exclude: [],
    rule: { op: 'all', rules: [{ op: 'min_size', value: 100000 }] },
    node_modules: false,
    docker: false,
    docker_volumes: false,
  });
  // Freshly written files are protected by the engine; freeze a representative plan instead.
  const plan = fixture.engine.clean_preview;
  const day = 86_400_000;
  plan.policy.roots = ['/home/user/Downloads'];
  plan.files = [
    ['/home/user/Downloads/setup-old.iso', 4_200_000, 'application/x-iso9660-image', 220],
    ['/home/user/Downloads/archives/photos-2021.zip', 2_400_000, 'application/zip', 410],
    ['/home/user/Downloads/video.mkv', 3_100_000, 'video/x-matroska', 95],
    ['/home/user/Downloads/notes.tmp', 120_000, 'text/plain', 31],
  ].map(([file, bytes, mime, age]) => ({
    path: file,
    bytes,
    mime,
    modified_ms: fixture.now - age * day,
    created_ms: fixture.now - (age + 2) * day,
  }));
  plan.total_bytes = plan.files.reduce((sum, file) => sum + file.bytes, 0);
} finally {
  bridge.close();
  await rm(data, { recursive: true, force: true });
}
fixture.engine.preferences = {
  ...fixture.engine.preferences,
  theme: 'system',
  language: 'fr',
  tray_enabled: true,
};
fixture.tauri = {
  startup_status: true,
  maintenance_info: 'deb',
  usb_targets: [],
  monitor_panel_target: { metric: 'memory', volume: '', visible: true },
  monitor_tray_status: { active: true, items: [] },
};
// Screenshots are published: replace the host name, user name and home directory
// with placeholders. Hardware descriptions stay as recorded.
const placeholders = [
  [os.homedir(), '/home/user'],
  [os.hostname(), 'demo-machine'],
  [os.userInfo().username, 'user'],
].filter(([value]) => value && value.length >= 3);
let serialized = JSON.stringify(fixture, null, 2);
for (const [value, placeholder] of placeholders)
  serialized = serialized.split(value).join(placeholder);
await mkdir(path.join(root, 'tests/visual'), { recursive: true });
await writeFile(path.join(root, 'tests/visual/fixture.json'), serialized);
console.log(
  JSON.stringify({
    reports: Object.keys(fixture.reports).length,
    processes: fixture.engine.monitor.processes.length,
    applications: fixture.engine.applications.apps?.length,
    cleanup_files: fixture.engine.clean_preview.files?.length,
  }),
);
