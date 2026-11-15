// Captures the React/Tauri interface as the visual baseline. The Tauri bridge is
// simulated in the page so the desktop variant renders with the frozen fixture.
// Usage: node scripts/visual/capture-react.mjs [output-dir] [scenario-filter]
import { chromium } from '@playwright/test';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { HEIGHT, STEP, WIDTH, captureName, scenarios } from './scenarios.mjs';

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '../..');
const output = path.resolve(root, process.argv[2] || 'tests/visual/baseline');
const filter = process.argv[3] ? new RegExp(process.argv[3]) : null;
const fixture = JSON.parse(await readFile(path.join(root, 'tests/visual/fixture.json'), 'utf8'));
await mkdir(output, { recursive: true });

const base = process.env.COMPUTY_TEST_BASE_URL || 'http://127.0.0.1:1420';
let server = null;
if (!process.env.COMPUTY_TEST_BASE_URL) {
  server = spawn('npx', ['vite', '--host', '127.0.0.1'], {
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
}

export function jobFixture(kind, data) {
  const latest = data.engine.history.reports[0];
  const report = latest ? data.reports[latest.id] : null;
  if (kind === 'running')
    return {
      id: 'visual-job',
      running: true,
      progress: 0.42,
      phase: 'Mémoire vive · motif 3 sur 8',
      component: 'memory',
      checks: report ? report.checks.slice(0, 1) : [],
      report: null,
      error: null,
      cancel_requested: false,
    };
  return {
    id: report?.id || 'visual-job',
    running: false,
    progress: 1,
    phase: 'Terminé',
    component: '',
    checks: report?.checks || [],
    report,
    error: null,
    cancel_requested: false,
  };
}

function bridgeScript({ fixture, setup, popover }) {
  const engine = structuredClone(fixture.engine);
  if (setup.emptyHistory) engine.history = { reports: [], unreadable: 0 };
  const job = setup.job ? setup.jobValue : null;
  engine.active = job;
  window.__COMPUTY_VISUAL__ = true;
  const respond = (cmd, args = {}) => {
    if (cmd === 'engine_call') {
      const { method, params = {} } = args;
      if (method === 'report') return fixture.reports[params.id];
      if (method === 'job') return job;
      if (method === 'save_preferences') return true;
      if (method in engine) return structuredClone(engine[method]);
      throw new Error(`fixture: ${method}`);
    }
    if (cmd === 'process_icons') return {};
    if (cmd === 'monitor_panel_target')
      return { metric: popover || 'memory', volume: '', visible: true };
    if (cmd in fixture.tauri) return fixture.tauri[cmd];
    throw new Error(`fixture: ${cmd}`);
  };
  window.__TAURI_INTERNALS__ = {
    invoke: async (cmd, args) => respond(cmd, args),
    transformCallback: () => 0,
    metadata: { currentWindow: { label: popover ? 'monitor-popover' : 'main' } },
  };
}

const actions = {
  'boot-mode': async (page) => {
    await page.locator('.mode-choice').nth(1).click();
  },
  'select-two': async (page) => {
    const boxes = page.locator('.table-checkbox input');
    await boxes.nth(0).check();
    await boxes.nth(1).check();
  },
  'open-compare': async (page) => {
    await page.getByRole('button', { name: 'Comparer les bilans' }).click();
  },
  'clean-preview': async (page) => {
    await page.getByRole('button', { name: 'Analyser sans supprimer' }).click();
    await page.waitForSelector('text=Vérifier et nettoyer');
  },
  'open-latest-report': async (page) => {
    await page.locator('.recent-row').first().click();
    await page.waitForSelector('.modal');
  },
};

// Compact computed-style tree used as the porting reference for each GPUI view.
function dumpStyles() {
  const skip = new Set(['SCRIPT', 'STYLE', 'HEAD', 'META', 'LINK', 'TITLE', 'BR']);
  const lines = [];
  const hex = (value) => {
    const m = value.match(/rgba?\(([^)]+)\)/);
    if (!m) return value;
    const [r, g, b, a = '1'] = m[1].split(/[ ,/]+/).filter(Boolean);
    const h = [r, g, b].map((n) => Number(n).toString(16).padStart(2, '0')).join('');
    return Number(a) >= 1
      ? `#${h}`
      : Number(a) === 0
        ? 'transparent'
        : `#${h}/${Number(a).toFixed(2)}`;
  };
  const walk = (el, depth) => {
    if (skip.has(el.tagName)) return;
    const s = getComputedStyle(el);
    if (s.display === 'none' || s.visibility === 'hidden') return;
    const r = el.getBoundingClientRect();
    if (r.width === 0 && r.height === 0 && !el.children.length) return;
    const parts = [];
    const cls =
      typeof el.className === 'string'
        ? el.className.trim().split(/\s+/).filter(Boolean).join('.')
        : '';
    parts.push(`${el.tagName.toLowerCase()}${cls ? '.' + cls : ''}`);
    parts.push(
      `[${Math.round(r.x)},${Math.round(r.y)} ${Math.round(r.width)}x${Math.round(r.height)}]`,
    );
    if (s.display.includes('flex') || s.display.includes('grid')) {
      parts.push(
        `${s.display}${s.flexDirection === 'column' ? ' col' : ''}${s.gap !== 'normal' ? ' gap=' + s.gap : ''}${s.display.includes('grid') ? ' cols=' + s.gridTemplateColumns : ''}`,
      );
      if (s.alignItems !== 'normal') parts.push(`items=${s.alignItems}`);
      if (s.justifyContent !== 'normal') parts.push(`justify=${s.justifyContent}`);
    }
    if (s.position !== 'static') parts.push(`pos=${s.position}`);
    const pad = [s.paddingTop, s.paddingRight, s.paddingBottom, s.paddingLeft].map((v) =>
      parseFloat(v),
    );
    if (pad.some(Boolean)) parts.push(`pad=${pad.join('/')}`);
    const bg = hex(s.backgroundColor);
    if (bg !== 'transparent') parts.push(`bg=${bg}`);
    if (s.backgroundImage !== 'none')
      parts.push(`bgimg=${s.backgroundImage.replace(/rgba?\([^)]+\)/g, hex).slice(0, 220)}`);
    const borders = ['Top', 'Right', 'Bottom', 'Left'].map((side) =>
      parseFloat(s[`border${side}Width`]) && s[`border${side}Style`] !== 'none'
        ? `${s[`border${side}Width`]} ${hex(s[`border${side}Color`])}`
        : '0',
    );
    if (borders.some((b) => b !== '0'))
      parts.push(`border=${new Set(borders).size === 1 ? borders[0] : borders.join('|')}`);
    if (s.borderRadius !== '0px') parts.push(`radius=${s.borderRadius}`);
    if (s.boxShadow !== 'none') parts.push(`shadow=${s.boxShadow.replace(/rgba?\([^)]+\)/g, hex)}`);
    if (s.opacity !== '1') parts.push(`opacity=${s.opacity}`);
    if (s.overflow !== 'visible') parts.push(`overflow=${s.overflow}`);
    const own = [...el.childNodes]
      .filter((n) => n.nodeType === 3 && n.textContent.trim())
      .map((n) => n.textContent.trim())
      .join(' ');
    if (own || el.tagName === 'svg') {
      parts.push(
        `font=${parseFloat(s.fontSize)}/${s.fontWeight}/${s.fontFamily.split(',')[0].replace(/"/g, '')}/lh=${s.lineHeight}${s.letterSpacing !== 'normal' ? '/ls=' + s.letterSpacing : ''}${s.textTransform !== 'none' ? '/' + s.textTransform : ''}`,
      );
      parts.push(`color=${hex(s.color)}`);
    }
    if (el.tagName === 'svg') {
      parts.push(`stroke=${el.getAttribute('stroke-width')} class=${el.getAttribute('class')}`);
      lines.push('  '.repeat(depth) + parts.join(' '));
      return;
    }
    if (own) parts.push(JSON.stringify(own.slice(0, 70)));
    lines.push('  '.repeat(depth) + parts.join(' '));
    for (const child of el.children) walk(child, depth + 1);
  };
  walk(document.body, 0);
  return lines.join('\n');
}

const freeze = `*,*::before,*::after{animation:none!important;transition:none!important;caret-color:transparent!important}html{scrollbar-width:none}::-webkit-scrollbar{display:none}`;
const browser = await chromium.launch({
  headless: true,
  executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE || '/usr/bin/google-chrome',
  args: ['--no-sandbox', '--font-render-hinting=none', '--hide-scrollbars'],
});
const manifest = [];
for (const scenario of scenarios) {
  if (filter && !filter.test(captureName(scenario))) continue;
  const width = scenario.popover ? 420 : scenario.width || WIDTH;
  const height = scenario.popover ? 720 : scenario.height || HEIGHT;
  const context = await browser.newContext({
    viewport: { width, height },
    deviceScaleFactor: 1,
    locale: scenario.locale,
    timezoneId: 'Europe/Paris',
    colorScheme: scenario.theme,
    reducedMotion: 'reduce',
  });
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.clock.setFixedTime(fixture.now);
  const setup = { ...scenario.setup };
  if (setup.job) setup.jobValue = jobFixture(setup.job, fixture);
  const preferences = { ...fixture.engine.preferences, language: scenario.locale };
  const data = { ...fixture, engine: { ...fixture.engine, preferences } };
  await page.addInitScript(bridgeScript, { fixture: data, setup, popover: scenario.popover });
  await page.addInitScript((value) => {
    localStorage.setItem('computy-preferences', JSON.stringify(value));
  }, preferences);
  const url = scenario.popover ? `${base}/?popover=1` : `${base}/#${scenario.page}`;
  await page.goto(url);
  await page.addStyleTag({ content: freeze });
  await page.waitForLoadState('networkidle');
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(600);
  for (const action of scenario.actions) {
    await actions[action](page);
    await page.waitForTimeout(400);
  }
  await page.mouse.move(width - 2, height - 2);
  await page.waitForTimeout(300);
  const total = await page.evaluate(() => document.documentElement.scrollHeight);
  const modal = await page.locator('.modal').count();
  const steps = scenario.popover || modal ? 1 : Math.max(1, Math.ceil((total - height) / STEP) + 1);
  const files = [];
  for (let step = 0; step < steps; step++) {
    await page.evaluate((y) => window.scrollTo(0, y), step * STEP);
    await page.waitForTimeout(150);
    const file = captureName(scenario, step);
    await page.screenshot({ path: path.join(output, file) });
    files.push(file);
  }
  if (process.env.COMPUTY_DUMP_STYLES) {
    await page.evaluate(() => window.scrollTo(0, 0));
    const dump = await page.evaluate(dumpStyles);
    await mkdir(path.join(output, 'styles'), { recursive: true });
    await writeFile(
      path.join(output, 'styles', captureName(scenario).replace('-s0.png', '.txt')),
      dump,
    );
  }
  manifest.push({ ...scenario, width, height, steps, files, errors });
  console.log(
    `${files.length}× ${scenario.id} ${scenario.theme} ${scenario.locale}${errors.length ? ' ERR ' + errors.join('; ') : ''}`,
  );
  await context.close();
}
await browser.close();
if (server) process.kill(-server.pid);
await writeFile(path.join(output, 'manifest.json'), JSON.stringify(manifest, null, 2));
console.log(`${manifest.reduce((n, s) => n + s.files.length, 0)} captures → ${output}`);
