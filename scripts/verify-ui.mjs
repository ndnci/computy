import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
const dataDir = await mkdtemp(path.join(tmpdir(), 'computy-ui-'));
const probe = createServer();
await new Promise((resolve) => probe.listen(0, '127.0.0.1', resolve));
const port = probe.address().port;
await new Promise((resolve) => probe.close(resolve));
const env = {
  ...process.env,
  COMPUTY_DATA_DIR: dataDir,
  COMPUTY_TEST_BASE_URL: `http://127.0.0.1:${port}`,
};
const server = spawn(
  process.execPath,
  ['node_modules/vite/bin/vite.js', '--host', '127.0.0.1', '--port', String(port)],
  { env, stdio: ['ignore', 'pipe', 'pipe'] },
);
let logs = '';
server.stdout.on('data', (data) => (logs += data));
server.stderr.on('data', (data) => (logs += data));
const run = (script) =>
  new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [script], { env, stdio: 'inherit' });
    child.on('error', reject);
    child.on('exit', (code) =>
      code === 0 ? resolve() : reject(new Error(`${script} failed with ${code}`)),
    );
  });
try {
  let ready = false;
  for (let n = 0; n < 100; n++) {
    try {
      const response = await fetch(env.COMPUTY_TEST_BASE_URL);
      if (response.ok) {
        ready = true;
        break;
      }
    } catch {}
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  if (!ready) throw new Error(logs || 'UI server did not start');
  await run('scripts/capture-ui.mjs');
  await run('scripts/check-locales-ui.mjs');
  await run('scripts/check-diagnostic-ux.mjs');
  await run('scripts/check-history-monitor-ux.mjs');
  await run('scripts/presentation-screens.mjs');
} finally {
  if (server.exitCode === null) {
    const closed = new Promise((resolve) => server.once('exit', resolve));
    server.kill('SIGINT');
    await closed;
  }
  await rm(dataDir, { recursive: true, force: true });
}
