#!/usr/bin/env node
// Refuses commits and CI runs that would publish details of a contributor's computer:
// absolute home-directory paths, the local host name or user name, and any extra
// private terms listed in the untracked `.privacy-terms` file (one per line) or in
// COMPUTY_PRIVATE_TERMS (comma separated). Hardware descriptions (CPU, GPU, kernel)
// are allowed. Text inside screenshots cannot be read here: anonymise recorded
// fixtures (scripts/visual/record-fixture.mjs does) before capturing.
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';

// Placeholder account names used on purpose in documentation, tests and CI.
const PLACEHOLDER_USERS = new Set([
  'user',
  'you',
  'me',
  'runner',
  'test',
  'demo',
  'example',
  'username',
  'name',
  'alice',
  'bob',
  'shared',
  'public',
  'default',
  'admin',
  'administrator',
  'all users',
  '$user',
  '${user}',
  '%username%',
  '<user>',
  '<you>',
]);

const HOME_PATTERNS = [
  /(?<![\w.])\/home\/([^/\s'"`)\]<>,;:…]+)/g,
  /(?<![\w.])\/Users\/([^/\s'"`)\]<>,;:…]+)/g,
  /\b[A-Za-z]:\\{1,2}Users\\{1,2}([^\\\s'"`)\]<>,;:…]+)/gi,
];
// The checker's own tests contain deliberate examples of leaks.
const SELF_TEST = 'tests/privacy.test.mjs';

function escape(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/** Returns `{ line, match, reason }` for each suspected leak in `text`. */
export function findLeaks(text, { terms = [] } = {}) {
  const leaks = [];
  const lines = text.split('\n');
  const termPatterns = terms
    .map((term) => term.trim())
    .filter((term) => term.length >= 3)
    .map((term) => ({ term, pattern: new RegExp(escape(term), 'i') }));
  lines.forEach((content, index) => {
    for (const pattern of HOME_PATTERNS) {
      for (const match of content.matchAll(pattern)) {
        if (!PLACEHOLDER_USERS.has(match[1].toLowerCase()))
          leaks.push({ line: index + 1, match: match[0], reason: 'personal home directory' });
      }
    }
    for (const { term, pattern } of termPatterns) {
      if (pattern.test(content))
        leaks.push({ line: index + 1, match: term, reason: 'private term' });
    }
  });
  return leaks;
}

/** Identifiers of the machine running the check, plus configured private terms. */
export function localTerms(root, environment = process.env) {
  const terms = [];
  const host = os.hostname();
  if (host && !['localhost', 'runner'].includes(host.toLowerCase())) terms.push(host);
  for (const value of (environment.COMPUTY_PRIVATE_TERMS || '').split(',')) terms.push(value);
  const file = path.join(root, '.privacy-terms');
  if (existsSync(file))
    for (const value of readFileSync(file, 'utf8').split('\n'))
      if (!value.trim().startsWith('#')) terms.push(value);
  const unique = new Map(terms.map((term) => [term.trim().toLowerCase(), term.trim()]));
  unique.delete('');
  return [...unique.values()];
}

function isBinary(buffer) {
  return buffer.subarray(0, 8000).includes(0);
}

// PNG tEXt/iTXt/zTXt chunks can carry the capturing tool, user or file path.
function pngText(buffer) {
  const chunks = [];
  let offset = 8;
  while (offset + 8 <= buffer.length) {
    const size = buffer.readUInt32BE(offset);
    const type = buffer.toString('latin1', offset + 4, offset + 8);
    if (['tEXt', 'iTXt', 'zTXt'].includes(type))
      chunks.push(buffer.toString('latin1', offset + 8, offset + 8 + size).replace(/\0/g, ' '));
    if (type === 'IEND') break;
    offset += size + 12;
  }
  return chunks.join('\n');
}

function main() {
  const root = execFileSync('git', ['rev-parse', '--show-toplevel'], { encoding: 'utf8' }).trim();
  const files = execFileSync('git', ['ls-files', '-z', '--cached'], { cwd: root, encoding: 'utf8' })
    .split('\0')
    .filter(Boolean)
    .filter((file) => file !== '.privacy-terms' && file !== SELF_TEST);
  const terms = localTerms(root);
  const problems = [];
  for (const file of files) {
    const absolute = path.join(root, file);
    if (!existsSync(absolute)) continue;
    const buffer = readFileSync(absolute);
    let text;
    if (file.toLowerCase().endsWith('.png')) text = pngText(buffer);
    else if (isBinary(buffer)) continue;
    else text = buffer.toString('utf8');
    for (const leak of findLeaks(text, { terms })) problems.push({ file, ...leak });
  }
  if (problems.length) {
    console.error('Private machine details found in tracked files:');
    for (const { file, line, match, reason } of problems)
      console.error(`  ${file}:${line}  ${reason}: ${match}`);
    console.error(
      'Use a relative path or a placeholder (/home/user, demo-machine) before committing.',
    );
    process.exit(1);
  }
  console.log(`Privacy check passed (${files.length} tracked files).`);
}

if (import.meta.url === `file://${process.argv[1]}`) main();
