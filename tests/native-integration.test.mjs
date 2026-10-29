import test from 'node:test';
import assert from 'node:assert/strict';
import { execFile, spawn } from 'node:child_process';
import { promisify } from 'node:util';
import { mkdtemp, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { createServer } from 'node:net';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';
import { StreamableHTTPClientTransport } from '@modelcontextprotocol/sdk/client/streamableHttp.js';
const exec = promisify(execFile);
const binary = path.resolve(
  process.env.COMPUTY_TEST_BINARY ||
    `native/target/release/computy-cli${process.platform === 'win32' ? '.exe' : ''}`,
);
test('CLI data erasure requires consent and keeps exported files', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'computy-erase-cli-'));
  const call = async (args) =>
    JSON.parse((await exec(binary, ['--data-dir', root, '--json', ...args])).stdout);
  try {
    await call(['diagnose', '--components', 'cpu', '--cpu-seconds', '1']);
    await writeFile(path.join(root, 'export.json'), 'personal export');
    await assert.rejects(call(['erase-data']));
    assert.equal((await call(['history'])).reports.length, 1);
    assert.deepEqual(await call(['erase-data', '--yes']), { erased: true });
    assert.equal((await call(['history'])).reports.length, 0);
    assert.equal(await readFile(path.join(root, 'export.json'), 'utf8'), 'personal export');
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
test('CLI monitor returns real passive measurements and all accessible processes without creating history', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'computy-monitor-'));
  try {
    const { stdout } = await exec(binary, ['--data-dir', root, '--json', 'monitor']);
    const value = JSON.parse(stdout);
    assert.ok(value.captured_at > 0);
    assert.ok(value.cpu_percent >= 0 && value.cpu_percent <= 100);
    assert.ok(value.memory_percent > 0 && value.memory_percent <= 100);
    assert.ok(value.gpu_percent === null || (value.gpu_percent >= 0 && value.gpu_percent <= 100));
    assert.ok(value.processes.length > 0);
    const { stdout: changed } = await exec(binary, [
      '--data-dir',
      root,
      '--json',
      'settings',
      'set',
      '--tray-enabled',
      'false',
      '--tray-style',
      'fill',
      '--tray-components',
      'memory,cpu',
      '--tray-disks',
      '',
    ]);
    assert.equal(JSON.parse(changed).tray_enabled, false);
    assert.equal(JSON.parse(changed).tray_style, 'fill');
    assert.deepEqual(JSON.parse(changed).tray_components, ['memory', 'cpu']);
    assert.deepEqual(JSON.parse(changed).tray_disks, []);
    const { stdout: history } = await exec(binary, ['--data-dir', root, '--json', 'history']);
    assert.equal(JSON.parse(history).reports.length, 0);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
test(
  'boot mutations require explicit consent flags before any operation',
  { skip: process.platform !== 'linux' },
  async () => {
    const root = await mkdtemp(path.join(tmpdir(), 'computy-boot-consent-'));
    try {
      await assert.rejects(exec(binary, ['--data-dir', root, 'boot', 'reboot']), (error) =>
        error.stderr.includes('requires explicit --yes'),
      );
      await assert.rejects(
        exec(binary, ['--data-dir', root, 'boot', 'usb-prepare', '--id', 'not-a-device']),
        (error) => error.stderr.includes('requires explicit --yes'),
      );
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  },
);
test('CLI runs real checks, preserves snapshots and shares validated preferences', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'computy-cli-test-'));
  const call = async (args) => {
    const { stdout } = await exec(binary, ['--data-dir', root, '--json', ...args]);
    return JSON.parse(stdout);
  };
  try {
    const settings = await call(['settings', 'show']);
    assert.equal(settings.theme, 'system');
    assert.equal(settings.language, 'system');
    await call(['settings', 'set', '--language', 'ja', '--theme', 'dark']);
    assert.equal((await call(['settings', 'show'])).language, 'ja');
    const before = await call([
      'diagnose',
      '--components',
      'memory,cpu',
      '--memory-mib',
      '16',
      '--passes',
      '1',
      '--cpu-seconds',
      '1',
      '--label',
      'CLI validation A',
    ]);
    assert.equal(before.completed, true);
    const ram = before.checks.find((c) => c.id === 'memory-patterns');
    assert.equal(ram.status, 'passed');
    assert.ok(ram.metrics.verified_bytes > 16 * 1024 ** 2);
    assert.ok(before.checks.find((c) => c.id === 'cpu-known-answers').metrics.iterations > 0);
    const after = await call([
      'diagnose',
      '--components',
      'storage',
      '--disk-mib',
      '16',
      '--label',
      'CLI validation B',
    ]);
    assert.equal(
      after.checks.find((c) => c.id === 'disk-roundtrip').metrics.verified_bytes,
      16 * 1024 ** 2,
    );
    const compared = await call(['compare', before.id, after.id]);
    assert.equal(compared.reports.length, 2);
    assert.ok(compared.warnings.length > 0);
    await call(['rename', before.id, 'Renamed validation']);
    assert.equal((await call(['show', before.id])).label, 'Renamed validation');
    const output = path.join(root, 'export.json');
    await exec(binary, ['--data-dir', root, 'export', before.id, '--output', output]);
    assert.equal(JSON.parse(await readFile(output, 'utf8')).id, before.id);
    await assert.rejects(call(['delete', before.id]));
    await call(['delete', before.id, '--yes']);
    assert.equal((await call(['history'])).reports.length, 1);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
test('official MCP client negotiates stdio and respects default read-only permissions', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'computy-mcp-test-'));
  const client = new Client({ name: 'computy-integration', version: '1.0' });
  try {
    await client.connect(
      new StdioClientTransport({
        command: binary,
        args: ['--data-dir', root, 'mcp'],
        stderr: 'pipe',
      }),
    );
    const tools = await client.listTools();
    assert.ok(tools.tools.some((t) => t.name === 'computy_inventory'));
    assert.ok(tools.tools.some((t) => t.name === 'computy_monitor'));
    const telemetry = await client.callTool({ name: 'computy_monitor', arguments: {} });
    assert.equal(telemetry.isError, false);
    assert.ok(JSON.parse(telemetry.content[0].text).memory_percent > 0);
    assert.ok(!tools.tools.some((t) => t.name === 'computy_start_diagnostic'));
    const result = await client.callTool({
      name: 'computy_delete_report',
      arguments: { id: '123' },
    });
    assert.equal(result.isError, true);
    const history = await client.callTool({ name: 'computy_history', arguments: {} });
    assert.equal(history.isError, false);
    assert.equal(JSON.parse(history.content[0].text).total, 0);
  } finally {
    await client.close();
    await rm(root, { recursive: true, force: true });
  }
});
test('HTTP MCP authenticates real SDK requests and rejects missing credentials and foreign origins', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'computy-http-test-'));
  const tokenFile = path.join(root, 'access.token');
  const probe = createServer();
  await new Promise((resolve) => probe.listen(0, '127.0.0.1', resolve));
  const port = probe.address().port;
  await new Promise((resolve) => probe.close(resolve));
  const child = spawn(
    binary,
    [
      '--data-dir',
      root,
      'mcp',
      '--http',
      '--port',
      String(port),
      '--ttl',
      '60',
      '--token-file',
      tokenFile,
    ],
    { stdio: ['ignore', 'pipe', 'pipe'] },
  );
  const client = new Client({ name: 'computy-http-integration', version: '1.0' });
  try {
    let token;
    for (let n = 0; n < 100; n++) {
      try {
        token = (await readFile(tokenFile, 'utf8')).trim();
        break;
      } catch {
        await new Promise((r) => setTimeout(r, 50));
      }
    }
    assert.ok(token);
    if (process.platform !== 'win32') assert.equal((await stat(tokenFile)).mode & 0o777, 0o600);
    const endpoint = `http://127.0.0.1:${port}/mcp`;
    assert.equal(
      (
        await fetch(endpoint, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: '{}',
        })
      ).status,
      401,
    );
    assert.equal(
      (
        await fetch(endpoint, {
          method: 'POST',
          headers: {
            Authorization: `Bearer ${token}`,
            Origin: 'https://untrusted.example',
            'Content-Type': 'application/json',
          },
          body: '{}',
        })
      ).status,
      403,
    );
    assert.equal(
      (
        await fetch(endpoint, {
          method: 'POST',
          headers: {
            Authorization: `Bearer ${token}`,
            Origin: 'https://évil.example',
            'Content-Type': 'application/json',
          },
          body: '{}',
        })
      ).status,
      403,
    );
    await client.connect(
      new StreamableHTTPClientTransport(new URL(endpoint), {
        requestInit: { headers: { Authorization: `Bearer ${token}` } },
      }),
    );
    assert.ok((await client.listTools()).tools.some((t) => t.name === 'computy_report'));
  } finally {
    await client.close();
    if (child.exitCode === null) {
      const exited = new Promise((resolve) => child.once('exit', resolve));
      child.kill('SIGINT');
      await exited;
    }
    if (process.platform !== 'win32') await assert.rejects(readFile(tokenFile));
    await rm(root, { recursive: true, force: true });
  }
});
