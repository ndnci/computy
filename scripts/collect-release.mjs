import { copyFile, mkdir, mkdtemp, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
const platform = process.env.COMPUTY_RELEASE_PLATFORM;
if (!['linux-x86_64', 'windows-x86_64', 'macos-x86_64', 'macos-aarch64'].includes(platform))
  throw Error('Unsupported release platform');
const { version } = JSON.parse(await readFile('package.json', 'utf8'));
const destination = 'release-assets';
await rm(destination, { recursive: true, force: true });
await mkdir(destination, { recursive: true });
const files = [];
const run = (command, args) => promisify(execFile)(command, args, { maxBuffer: 1 << 24 });
const suffix = platform.startsWith('windows') ? '.exe' : '';
if (platform === 'linux-x86_64') {
  // Built by scripts/package-deb.sh.
  for (const name of await readdir('release'))
    if (name === `Computy_${version}_amd64.deb`) {
      await copyFile(path.join('release', name), path.join(destination, name));
      files.push(name);
    }
} else if (platform === 'windows-x86_64') {
  const name = `Computy_${version}_${platform}.zip`;
  const staging = await mkdtemp(path.join(tmpdir(), 'computy-app-'));
  await copyFile('desktop/target/release/computy.exe', path.join(staging, 'Computy.exe'));
  for (const file of ['LICENSE', 'NOTICE']) await copyFile(file, path.join(staging, file));
  await run('tar', [
    '-a',
    '-c',
    '-f',
    path.resolve(destination, name),
    '-C',
    staging,
    'Computy.exe',
    'LICENSE',
    'NOTICE',
  ]);
  await rm(staging, { recursive: true, force: true });
  files.push(name);
} else {
  const name = `Computy_${version}_${platform}.dmg`;
  await run('scripts/package-macos.sh', [path.resolve(destination, name)]);
  files.push(name);
}
if (!files.length) throw Error('No installers were built');
const cli = `computy-cli_${version}_${platform}.tar.gz`;
const staging = await mkdtemp(path.join(tmpdir(), 'computy-release-'));
try {
  await copyFile(
    `native/target/release/computy-cli${suffix}`,
    path.join(staging, `computy-cli${suffix}`),
  );
  for (const file of ['LICENSE', 'NOTICE']) await copyFile(file, path.join(staging, file));
  await promisify(execFile)('tar', [
    '-czf',
    path.resolve(destination, cli),
    '-C',
    staging,
    `computy-cli${suffix}`,
    'LICENSE',
    'NOTICE',
  ]);
} finally {
  await rm(staging, { recursive: true, force: true });
}
files.push(cli);
const checksums = [];
for (const file of files.sort())
  checksums.push(
    `${createHash('sha256')
      .update(await readFile(path.join(destination, file)))
      .digest('hex')}  ${file}`,
  );
await writeFile(path.join(destination, `SHA256SUMS_${platform}.txt`), checksums.join('\n') + '\n');
console.log(`${platform}: ${files.length} release assets collected`);
