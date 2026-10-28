import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
const root = new URL('../third_party/computy-memory-boot/', import.meta.url);
test('the integrated upstream test algorithms are unchanged and GPL source is retained', async () => {
  const upstream = JSON.parse(await readFile(new URL('COMPUTY-UPSTREAM.json', root), 'utf8'));
  assert.equal(upstream.tag, 'v9.00');
  assert.equal(upstream.license, 'GPL-2.0');
  assert.ok(
    (await readFile(new URL('LICENSE', root), 'utf8')).includes('GNU GENERAL PUBLIC LICENSE'),
  );
  for (const [file, expected] of Object.entries(upstream.algorithm_files))
    assert.equal(
      createHash('sha256')
        .update(await readFile(new URL(file, root)))
        .digest('hex'),
      expected,
      file,
    );
});
