import test from 'node:test';
import assert from 'node:assert/strict';
import {
  formatBytes,
  statusInfo,
  comparisonWarnings,
  componentSummary,
  scoreStatus,
  checkOutcome,
} from '../src/domain.mjs';
import { setLanguage } from '../src/i18n.mjs';
await setLanguage('fr');

test('unavailable and cancelled checks never look healthy', () => {
  assert.equal(statusInfo('unavailable').tone, 'muted');
  assert.equal(statusInfo('cancelled').tone, 'muted');
  assert.equal(componentSummary([], 'memory').score, null);
});
test('a single failed control dominates the component', () => {
  assert.equal(
    componentSummary(
      [
        { component: 'storage', status: 'passed' },
        { component: 'storage', status: 'failed' },
      ],
      'storage',
    ).score,
    0,
  );
});
test('bytes are formatted in binary units with a visible unknown state', () => {
  assert.equal(formatBytes(null), 'Non communiqué');
  assert.equal(formatBytes(16 * 1024 ** 3), '16 Gio');
  assert.equal(formatBytes(0), '0 o');
});
test('comparison warns for different machines and protocols', () => {
  const before = {
    inventory: { machine_id: 'a' },
    options: { memory_mib: 64, passes: 1, cpu_seconds: 1, disk_mib: 16 },
    completed: true,
  };
  const after = {
    inventory: { machine_id: 'b' },
    options: { memory_mib: 256, passes: 2, cpu_seconds: 5, disk_mib: 64 },
    completed: false,
  };
  assert.equal(comparisonWarnings([before, after]).length, 3);
  assert.equal(comparisonWarnings([before, { ...before }]).length, 0);
});
test('a successful score smiles while unavailable evidence remains separate', () => {
  const checks = [
    { status: 'passed', name: 'RAM' },
    { status: 'unavailable', name: 'ECC' },
  ];
  assert.equal(scoreStatus(10, 'warning'), 'passed');
  assert.deepEqual(checkOutcome(checks), { status: 'passed', unavailable: ['ECC'] });
  assert.equal(checkOutcome([...checks, { status: 'warning', name: 'wear' }]).status, 'warning');
  assert.equal(checkOutcome([...checks, { status: 'failed', name: 'fault' }]).status, 'failed');
  assert.equal(checkOutcome([{ status: 'unavailable', name: 'ECC' }]).status, 'unavailable');
  assert.equal(checkOutcome(checks, false).status, 'cancelled');
  assert.equal(scoreStatus(10, 'cancelled'), 'cancelled');
  assert.equal(scoreStatus(10, 'failed'), 'failed');
});
