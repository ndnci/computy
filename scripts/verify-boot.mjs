// Runs only in an isolated QEMU environment; never probes or changes host boot state.
import { spawn } from 'node:child_process';
import net from 'node:net';
import { mkdtemp, readFile, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import zlib from 'node:zlib';
import assert from 'node:assert/strict';
const root = await mkdtemp(path.join(tmpdir(), 'computy-boot-qa-'));
const results = [];
function pngFromPpm(bytes) {
  const match = bytes.toString('ascii', 0, 100).match(/^P6\s+(\d+)\s+(\d+)\s+255\s/);
  if (!match) throw new Error('Unexpected QEMU image');
  const width = Number(match[1]),
    height = Number(match[2]),
    data = bytes.subarray(match[0].length);
  assert.equal(data.length, width * height * 3);
  const crc = (bytes) => {
    let c = 0xffffffff;
    for (const b of bytes) {
      c ^= b;
      for (let n = 0; n < 8; n++) c = (c >>> 1) ^ (c & 1 ? 0xedb88320 : 0);
    }
    return (c ^ 0xffffffff) >>> 0;
  };
  const chunk = (type, data) => {
    const label = Buffer.from(type);
    const out = Buffer.alloc(12 + data.length);
    out.writeUInt32BE(data.length);
    label.copy(out, 4);
    data.copy(out, 8);
    out.writeUInt32BE(crc(Buffer.concat([label, data])), 8 + data.length);
    return out;
  };
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width);
  header.writeUInt32BE(height, 4);
  header[8] = 8;
  header[9] = 2;
  const rows = Buffer.alloc((width * 3 + 1) * height);
  for (let y = 0; y < height; y++)
    data.copy(rows, y * (width * 3 + 1) + 1, y * width * 3, (y + 1) * width * 3);
  return Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk('IHDR', header),
    chunk('IDAT', zlib.deflateSync(rows)),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}
async function monitor(socketPath, filename) {
  const socket = net.createConnection(socketPath);
  let buffer = '';
  const pending = new Map();
  let ready;
  const greeting = new Promise((resolve) => (ready = resolve));
  socket.on('data', (data) => {
    buffer += data;
    for (;;) {
      const n = buffer.indexOf('\n');
      if (n < 0) break;
      const line = buffer.slice(0, n);
      buffer = buffer.slice(n + 1);
      try {
        const message = JSON.parse(line);
        if (message.QMP) ready();
        if (message.id && pending.has(message.id)) {
          pending.get(message.id)(message);
          pending.delete(message.id);
        }
      } catch {}
    }
  });
  await greeting;
  let seq = 0;
  const send = (execute, args) =>
    new Promise((resolve, reject) => {
      const id = ++seq;
      pending.set(id, (message) =>
        message.error ? reject(new Error(JSON.stringify(message.error))) : resolve(message),
      );
      socket.write(JSON.stringify({ execute, arguments: args, id }) + '\n');
    });
  await send('qmp_capabilities');
  await send('screendump', { filename });
  socket.end();
}
try {
  for (const firmware of ['bios', 'uefi']) {
    const serial = path.join(root, firmware + '.log'),
      qmp = path.join(root, firmware + '.sock'),
      image = path.join(root, firmware + '.ppm');
    const args = [
      '-machine',
      'q35,accel=tcg',
      '-m',
      '128',
      '-smp',
      '2',
      '-cpu',
      'max',
      '-kernel',
      path.resolve('third_party/computy-memory-boot/build/x86_64/mt86plus'),
      '-append',
      'nopause dark console=ttyS0,115200 testlist=0,1,2',
      '-display',
      'none',
      '-serial',
      `file:${serial}`,
      '-qmp',
      `unix:${qmp},server=on,wait=off`,
      '-no-reboot',
    ];
    if (firmware === 'uefi') args.push('-bios', '/usr/share/OVMF/OVMF_CODE.fd');
    const child = spawn('qemu-system-x86_64', args, { stdio: ['ignore', 'ignore', 'pipe'] });
    let errors = '';
    child.stderr.on('data', (data) => (errors += data));
    try {
      const deadline = Date.now() + 120000;
      let text = '';
      let passed = false;
      while (Date.now() < deadline) {
        if (child.exitCode !== null) throw new Error(errors || 'QEMU exited early');
        try {
          text = (await readFile(serial, 'utf8')).replace(/\x1b\[[0-9;]*[a-zA-Z]/g, '');
          passed = /Pass:\s*[1-9]/.test(text);
          if (passed) break;
        } catch {}
        await new Promise((resolve) => setTimeout(resolve, 250));
      }
      assert.ok(
        passed,
        `${firmware}: no completed validation pass\n${text.slice(-1200)}\n${errors}`,
      );
      assert.ok(text.includes('COMPUTY'));
      await monitor(qmp, image);
      await writeFile(
        `docs/screenshots/boot-engine-${firmware}.png`,
        pngFromPpm(await readFile(image)),
      );
      results.push({
        firmware,
        completed_pass: true,
        tests: [0, 1, 2],
        virtual_memory_mib: 128,
        scope: 'QEMU boot smoke test; not physical hardware certification',
      });
    } finally {
      if (child.exitCode === null) {
        const stopped = new Promise((resolve) => child.once('exit', resolve));
        child.kill('SIGTERM');
        await stopped;
      }
    }
  }
  await writeFile('docs/screenshots/boot-results.json', JSON.stringify(results, null, 2) + '\n');
  console.log(
    'Computy boot engine verified in BIOS and UEFI VMs; selected real test passes completed.',
  );
} finally {
  await rm(root, { recursive: true, force: true });
}
