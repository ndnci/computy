import { copyFile, mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { createHash } from 'node:crypto';
const platform = process.env.COMPUTY_RELEASE_PLATFORM;
if (!['linux-x86_64', 'windows-x86_64', 'macos-x86_64', 'macos-aarch64'].includes(platform))
  throw Error('Unsupported release platform');
const { version } = JSON.parse(await readFile('package.json', 'utf8'));
const destination = 'release-assets';
await mkdir(destination, { recursive: true });
const files = [];
async function visit(directory) {
  for (const item of await readdir(directory, { withFileTypes: true })) {
    const file = path.join(directory, item.name);
    if (item.isDirectory()) await visit(file);
    else if (/\.(deb|AppImage|exe|dmg)$/.test(item.name) && item.name.includes(version)) {
      await copyFile(file, path.join(destination, item.name));
      files.push(item.name);
    }
  }
}
await visit('src-tauri/target/release/bundle');
if (!files.length) throw Error('No installers were built');
const suffix = platform.startsWith('windows') ? '.exe' : '';
const cli = `computy-cli_${version}_${platform}${suffix}`;
await copyFile(`native/target/release/computy-cli${suffix}`, path.join(destination, cli));
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
