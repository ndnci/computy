import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
const { version } = JSON.parse(await readFile('package.json', 'utf8'));
const config = JSON.parse(await readFile('src-tauri/tauri.conf.json', 'utf8'));
assert.equal(config.version, version);
for (const file of ['native/Cargo.toml', 'src-tauri/Cargo.toml']) {
  assert.equal((await readFile(file, 'utf8')).match(/^version = "([^"]+)"/m)?.[1], version, file);
}
const changelog = await readFile('CHANGELOG.md', 'utf8');
assert.ok(changelog.includes(`## [${version}] - `), 'Add a dated changelog entry before releasing');
if (process.env.COMPUTY_RELEASE_TAG)
  assert.equal(
    process.env.COMPUTY_RELEASE_TAG,
    `v${version}`,
    'Release tag must match the application version',
  );
console.log(`Release metadata verified: v${version}`);
