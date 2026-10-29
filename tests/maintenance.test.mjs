import test from 'node:test';
import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { mkdtemp, mkdir, writeFile, readFile, rm, realpath } from 'node:fs/promises';
import path from 'node:path';
import { tmpdir } from 'node:os';
const exec = promisify(execFile);
const binary = path.resolve(
  process.env.COMPUTY_TEST_BINARY ||
    `native/target/release/computy-cli${process.platform === 'win32' ? '.exe' : ''}`,
);
test('real CLI cleanup requires a preview and confirmation, preserves exclusions and records automations', async () => {
  const root = await realpath(await mkdtemp(path.join(tmpdir(), 'computy-clean-cli-')));
  const call = async (args) =>
    JSON.parse(
      (await exec(binary, ['--data-dir', path.join(root, 'state'), '--json', ...args])).stdout,
    );
  try {
    await mkdir(path.join(root, 'cache'));
    await writeFile(path.join(root, 'cache/delete.log'), 'temporary');
    await writeFile(path.join(root, 'cache/keep.log'), 'important');
    const policy = path.join(root, 'policy.json');
    await writeFile(
      policy,
      JSON.stringify({
        roots: [path.join(root, 'cache')],
        exclude: [path.join(root, 'cache/keep.log')],
        rule: {
          op: 'all',
          rules: [
            { op: 'extension', value: 'log' },
            { op: 'path_regex', value: 'delete' },
          ],
        },
      }),
    );
    const preview = await call(['clean', 'preview', '--policy', policy]);
    assert.equal(preview.files.length, 1);
    assert.equal(preview.total_bytes, 9);
    await assert.rejects(call(['clean', 'execute', preview.id]));
    assert.equal(await readFile(path.join(root, 'cache/delete.log'), 'utf8'), 'temporary');
    const schedule = await call([
      'clean',
      'schedule',
      preview.id,
      '--hours',
      '24',
      '--max-bytes',
      '1024',
      '--yes',
    ]);
    assert.equal((await call(['clean', 'schedules']))[0].id, schedule.id);
    const result = await call(['clean', 'execute', preview.id, '--yes']);
    assert.equal(result.deleted, 1);
    assert.equal(await readFile(path.join(root, 'cache/keep.log'), 'utf8'), 'important');
    await assert.rejects(call(['clean', 'execute', preview.id, '--yes']));
    await exec(binary, [
      '--data-dir',
      path.join(root, 'state'),
      'clean',
      'unschedule',
      schedule.id,
    ]);
    assert.equal((await call(['clean', 'schedules'])).length, 0);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
test('CLI fan and application inventory expose actual capabilities without modifying the host', async () => {
  for (const [command, key] of [
    ['applications', 'apps'],
    ['fans', 'fans'],
  ]) {
    const { stdout } = await exec(binary, ['--json', command], {
      timeout: 60000,
      maxBuffer: 8 * 1024 * 1024,
    });
    const result = JSON.parse(stdout);
    assert.ok(Array.isArray(result[key]));
    if (command === 'fans') assert.equal(result.control_available, false);
  }
});
