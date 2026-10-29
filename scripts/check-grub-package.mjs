// Mutates only an explicitly isolated Docker filesystem, never the host boot files.
import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { access, mkdir, readFile, writeFile, chmod } from 'node:fs/promises';
const exec = promisify(execFile);
assert.equal(process.env.COMPUTY_PACKAGING_TEST_SANDBOX, '1');
await access('/.dockerenv');
assert.equal(process.getuid(), 0);
for (const dir of ['/boot/grub', '/etc/default/grub.d', '/usr/sbin', '/usr/bin'])
  await mkdir(dir, { recursive: true });
await writeFile('/boot/computy-memory.efi', 'fixture payload, never executed');
const original = 'menuentry "Windows fixture" { true; }\nmenuentry "Linux fixture" { true; }\n';
await writeFile('/boot/grub/grub.cfg', original);
const probe = `#!/bin/sh
if [ "\$COMPUTY_TEST_PROBE_FAIL" = 1 ]; then printf 'invalid partial output'; exit 42; fi
case "\$*" in
  *--target=partmap*) printf 'gpt\\n';;
  *--target=abstraction*) :;;
  *--target=fs_uuid*) printf '00000000-0000-0000-0000-000000000001\\n';;
  *--target=fs*) printf 'ext2\\n';;
  *--target=compatibility_hint*) printf 'hd0,gpt1\\n';;
  *--target=hints_string*) printf '%s\\n' '--hint-bios=hd0,gpt1';;
  *) exit 1;;
esac
`;
await writeFile('/usr/sbin/grub-probe', probe);
await chmod('/usr/sbin/grub-probe', 0o755);
await writeFile(
  '/usr/bin/grub-mkrelpath',
  '#!/bin/sh\nprintf "%s\\n" "${COMPUTY_TEST_RELATIVE:-/boot/computy-memory.efi}"\n',
);
await chmod('/usr/bin/grub-mkrelpath', 0o755);
const libs = [['Debian', await readFile('/usr/lib/grub/grub-mkconfig_lib', 'utf8')]];
if (process.env.COMPUTY_HOST_GRUB_LIB)
  libs.push(['Ubuntu host', await readFile(process.env.COMPUTY_HOST_GRUB_LIB, 'utf8')]);
const generator =
  process.env.COMPUTY_TEST_GRUB_GENERATOR || '/workspace/packaging/41_computy_memory';
const base = {
  PATH: '/usr/sbin:/usr/bin:/sbin:/bin',
  pkgdatadir: '/usr/share/grub',
  GRUB_DEVICE_BOOT: '/dev/computy-fixture',
};
const results = [];
for (const [name, library] of libs) {
  await writeFile('/usr/lib/grub/grub-mkconfig_lib', library);
  for (const relative of ['/boot/computy-memory.efi', '/computy-memory.efi']) {
    await writeFile('/etc/default/grub', 'GRUB_DEFAULT=0\n');
    const result = await exec('/bin/sh', [generator], {
      env: { ...base, COMPUTY_TEST_RELATIVE: relative },
    });
    assert.ok(result.stdout.includes("--id 'computy-memory-test'"), name);
    assert.ok(result.stdout.includes(`linux ${relative} nopause dark`), name);
    assert.ok(result.stdout.includes('insmod part_gpt'), name);
    await writeFile('/tmp/computy-entry.cfg', result.stdout);
    await exec('/usr/bin/grub-script-check', ['/tmp/computy-entry.cfg']);
  }
  await writeFile('/etc/default/grub', 'GRUB_DEFAULT=2\n');
  assert.equal(
    (await exec('/bin/sh', [generator], { env: base })).stdout,
    '',
    name + ' custom default must remain untouched',
  );
  await writeFile(
    '/boot/grub/grub.cfg',
    original + "menuentry 'Computy' --id 'computy-memory-test' { true; }\n",
  );
  assert.ok(
    (await exec('/bin/sh', [generator], { env: base })).stdout.includes('computy-memory-test'),
    name + ' preserve existing entry',
  );
  await writeFile('/boot/grub/grub.cfg', original);
  await writeFile('/etc/default/grub', 'GRUB_DEFAULT=0\n');
  await assert.rejects(
    exec('/bin/sh', [generator], { env: { ...base, COMPUTY_TEST_PROBE_FAIL: '1' } }),
    (error) => {
      assert.equal(error.stdout, '', 'failure must not write a partial entry');
      return true;
    },
  );
  assert.equal(await readFile('/boot/grub/grub.cfg', 'utf8'), original);
  assert.equal(
    (
      await exec('/bin/sh', [generator], {
        env: { ...base, COMPUTY_TEST_RELATIVE: '/unsupported/path' },
      })
    ).stdout,
    '',
  );
  const noDevice = { ...base };
  delete noDevice.GRUB_DEVICE_BOOT;
  assert.equal((await exec('/bin/sh', [generator], { env: noDevice })).stdout, '');
  results.push({
    library: name,
    root_and_separate_boot: true,
    grub_syntax: true,
    custom_default_preserved: true,
    existing_entry_preserved: true,
    failed_probe_has_no_output: true,
    original_configuration_preserved: true,
  });
}
await writeFile('/usr/lib/grub/grub-mkconfig_lib', libs[0][1]);
console.log(JSON.stringify({ grub_packaging: results }, null, 2));
