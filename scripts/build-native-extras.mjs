import { spawnSync } from 'node:child_process';
import { mkdirSync } from 'node:fs';
if (process.platform === 'linux' && process.arch === 'x64') {
  const run = (command, args) => {
    const result = spawnSync(command, args, { stdio: 'inherit' });
    if (result.error) throw result.error;
    if (result.status !== 0) process.exit(result.status || 1);
  };
  run('make', [
    '-C',
    'third_party/computy-memory-boot/build/x86_64',
    'GIT=none',
    '-j2',
    'mt86plus',
  ]);
  run('cargo', [
    'build',
    '--release',
    '--locked',
    '--manifest-path',
    'native/Cargo.toml',
    '--no-default-features',
    '--features',
    'boot-helper',
    '--bin',
    'computy-boot-helper',
    '--target-dir',
    'src-tauri/target/boot-helper',
  ]);
  mkdirSync('src-tauri/target/boot-helper', { recursive: true });
  run('tar', [
    '--exclude=*.o',
    '--exclude=*.d',
    '--exclude=mt86plus',
    '--exclude=*.iso',
    '--exclude=*.debug',
    '-czf',
    'src-tauri/target/boot-helper/computy-memory-sources.tar.gz',
    'third_party/computy-memory-boot',
  ]);
}
