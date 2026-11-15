// Exports the rotated decorative pieces of the former stylesheet as transparent
// PNGs (2x) for the native interface, which cannot rotate arbitrary boxes.
// Usage: node scripts/visual/export-art.mjs
import { chromium } from '@playwright/test';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '../..');
const output = path.join(root, 'desktop/assets/art');
const fixture = JSON.parse(await readFile(path.join(root, 'tests/visual/fixture.json'), 'utf8'));
await mkdir(output, { recursive: true });
const base = 'http://127.0.0.1:1420';
const server = spawn('npx', ['vite', '--host', '127.0.0.1'], {
  cwd: root,
  stdio: 'ignore',
  detached: true,
});
for (let i = 0; i < 100; i++) {
  try {
    if ((await fetch(base)).ok) break;
  } catch {}
  await new Promise((resolve) => setTimeout(resolve, 200));
}
const parts = [
  // [name, page hash, selector, container selector, setup]
  ['hero-chip-one', 'overview', '.chip-one', '.hero-art', {}],
  ['hero-chip-two', 'overview', '.chip-two', '.hero-art', {}],
  ['hero-chip-three', 'overview', '.chip-three', '.hero-art', {}],
  ['hero-core', 'overview', '.hero-core', '.hero-art', {}],
  ['history-stack', 'overview', '.history-stack', '.history-stack', { emptyHistory: true }],
];
const browser = await chromium.launch({
  headless: true,
  executablePath: '/usr/bin/google-chrome',
  args: ['--no-sandbox', '--font-render-hinting=none', '--hide-scrollbars'],
});
const anchors = {};
for (const theme of ['light', 'dark']) {
  for (const [name, hash, selector, container, setup] of parts) {
    const context = await browser.newContext({
      viewport: { width: 1440, height: 960 },
      deviceScaleFactor: 2,
      colorScheme: theme,
      reducedMotion: 'reduce',
    });
    const page = await context.newPage();
    const engine = structuredClone(fixture.engine);
    if (setup.emptyHistory) engine.history = { reports: [], unreadable: 0 };
    engine.preferences = { ...engine.preferences, language: 'fr', theme };
    await page.addInitScript(
      ({ engine, reports }) => {
        window.__TAURI_INTERNALS__ = {
          invoke: async (cmd, args = {}) => {
            if (cmd !== 'engine_call') return cmd === 'startup_status' ? true : {};
            if (args.method === 'report') return reports[args.params.id];
            return engine[args.method] ?? null;
          },
          transformCallback: () => 0,
        };
        localStorage.setItem('computy-preferences', JSON.stringify(engine.preferences));
      },
      { engine, reports: fixture.reports },
    );
    await page.goto(`${base}/#${hash}`);
    await page.addStyleTag({
      content: `*,*::before,*::after{animation:none!important;transition:none!important}
        html,body{background:transparent!important}
        body *{visibility:hidden!important}
        ${selector},${selector} *{visibility:visible!important}`,
    });
    await page.waitForSelector(selector);
    await page.evaluate((selector) => {
      const element = document.querySelector(selector);
      window.scrollTo(0, Math.max(0, element.getBoundingClientRect().top + window.scrollY - 200));
    }, selector);
    await page.evaluate(() => document.fonts.ready);
    await page.waitForTimeout(500);
    const box = await page.evaluate(
      ([selector, container]) => {
        const r = document.querySelector(selector).getBoundingClientRect();
        const c = document.querySelector(container).getBoundingClientRect();
        return {
          x: r.x,
          y: r.y,
          w: r.width,
          h: r.height,
          cx: c.x,
          cy: c.y,
          cw: c.width,
          ch: c.height,
        };
      },
      [selector, container],
    );
    const pad = 24;
    const clip = {
      x: box.x - pad,
      y: box.y - pad,
      width: box.w + 2 * pad,
      height: box.h + 2 * pad,
    };
    await page.screenshot({
      path: path.join(output, `${name}-${theme}.png`),
      clip,
      omitBackground: true,
    });
    anchors[name] = {
      width: clip.width,
      height: clip.height,
      left: clip.x - box.cx,
      top: clip.y - box.cy,
      right: box.cx + box.cw - (clip.x + clip.width),
      bottom: box.cy + box.ch - (clip.y + clip.height),
      container: { width: box.cw, height: box.ch },
    };
    await context.close();
  }
}
await browser.close();
process.kill(-server.pid);
await writeFile(path.join(output, 'anchors.json'), JSON.stringify(anchors, null, 2) + '\n');
console.log(JSON.stringify(anchors, null, 1));
