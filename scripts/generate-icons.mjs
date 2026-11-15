// Generates the Lucide SVG icons used by the native interface from the pinned
// lucide-react package. Each icon is written for every stroke width the UI uses,
// because GPUI rasterises SVG files as monochrome masks.
// Usage: node scripts/generate-icons.mjs
import { readdir, readFile, mkdir, writeFile, rm } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const icons = path.join(root, 'node_modules/lucide-react/dist/esm/icons');
const output = path.join(root, 'desktop/assets/icons');
const strokes = ['1', '1.5', '1.6', '1.7', '1.8', '2', '2.4'];
const kebab = (name) =>
  name
    .replace(/([a-z0-9])([A-Z])/g, '$1-$2')
    .replace(/([A-Z])([A-Z][a-z])/g, '$1-$2')
    .replace(/([a-zA-Z])(\d)/g, '$1-$2')
    .toLowerCase();

async function sources(directory) {
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const file = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...(await sources(file)));
    else if (/\.tsx?$/.test(entry.name)) files.push(file);
  }
  return files;
}

const names = new Set(
  // Icons the native interface needs beyond the React imports.
  ['chevron-down', 'chevron-up', 'minimize', 'maximize', 'copy'],
);
for (const file of await sources(path.join(root, 'src'))) {
  const text = await readFile(file, 'utf8');
  for (const match of text.matchAll(/import\s*\{([^}]+)\}\s*from\s*'lucide-react'/g))
    for (const raw of match[1].split(',')) {
      const name = raw.trim().split(/\s+as\s+/)[0];
      if (name && !name.startsWith('type ') && name !== 'LucideIcon') names.add(kebab(name));
    }
}
const attributes = (props) =>
  Object.entries(props)
    .filter(([key]) => key !== 'key')
    .map(([key, value]) => `${key.replace(/[A-Z]/g, (c) => '-' + c.toLowerCase())}="${value}"`)
    .join(' ');
await rm(output, { recursive: true, force: true });
await mkdir(output, { recursive: true });
const missing = [];
let count = 0;
for (const name of [...names].sort()) {
  let module;
  try {
    let file = path.join(icons, `${name}.mjs`);
    module = await import(pathToFileURL(file).href);
    // Aliases re-export another icon module.
    while (!module.__iconData) {
      const target = (await readFile(file, 'utf8')).match(/from '\.\/([\w-]+\.mjs)'/);
      if (!target) throw new Error(name);
      file = path.join(icons, target[1]);
      module = await import(pathToFileURL(file).href);
    }
  } catch {
    missing.push(name);
    continue;
  }
  const body = module.__iconData.node
    .map(([tag, props]) => `<${tag} ${attributes(props)}/>`)
    .join('');
  for (const stroke of strokes) {
    await writeFile(
      path.join(output, `${name}@${stroke}.svg`),
      `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="${stroke}" stroke-linecap="round" stroke-linejoin="round">${body}</svg>\n`,
    );
    count++;
  }
}
await writeFile(
  path.join(output, 'LICENSE'),
  'Lucide icons, ISC License. Copyright (c) for portions of Lucide are held by Cole Bemis 2013-2022 as part of Feather (MIT). All other copyright (c) for Lucide are held by Lucide Contributors 2022.\n',
);
console.log(JSON.stringify({ icons: names.size - missing.length, files: count, missing }));
