import test from 'node:test';
import assert from 'node:assert/strict';
import { findLeaks } from '../scripts/check-privacy.mjs';

test('personal home directories are reported', () => {
  for (const text of [
    '"binary": "/home/alex/projects/computy/target/release/computy"',
    'saved to /Users/alex/Library/Caches',
    'C:\\Users\\alex\\AppData\\Local',
    '"C:\\\\Users\\\\alex\\\\AppData"',
  ])
    assert.equal(findLeaks(text).length, 1, text);
});

test('placeholder home directories are allowed', () => {
  for (const text of [
    '"roots": ["/home/you/.cache"]',
    "plan.policy.roots = ['/home/user/Downloads'];",
    'C:\\Users\\runner\\work',
    'see /home/ for user data',
  ])
    assert.deepEqual(findLeaks(text), [], text);
});

test('private terms such as the host name are reported case-insensitively', () => {
  const leaks = findLeaks('Diagnostic\nMy-Laptop · Ubuntu', { terms: ['my-laptop'] });
  assert.deepEqual(leaks, [{ line: 2, match: 'my-laptop', reason: 'private term' }]);
});

test('hardware descriptions are not treated as private', () => {
  const text = '"gpus": ["NVIDIA Corporation GA106M [GeForce RTX 3060 Mobile / Max-Q]"]';
  assert.deepEqual(findLeaks(text, { terms: ['my-laptop'] }), []);
});
