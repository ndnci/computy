// Shared visual-regression scenarios. The React baseline and the GPUI capture mode
// implement the same scenario ids, actions and fixture overrides.
export const WIDTH = 1440;
export const HEIGHT = 960;
export const STEP = 860; // vertical scroll step between viewport captures

const pages = [
  ['overview', 'overview', []],
  ['overview-empty', 'overview', [], { emptyHistory: true }],
  ['diagnostics', 'diagnostics', []],
  ['diagnostics-boot', 'diagnostics', ['boot-mode']],
  ['diagnostics-running', 'diagnostics', [], { job: 'running' }],
  ['diagnostics-done', 'diagnostics', [], { job: 'done' }],
  ['floating-job', 'overview', [], { job: 'running' }],
  ['memory', 'memory', []],
  ['storage', 'storage', []],
  ['cpu', 'cpu', []],
  ['fans', 'fans', []],
  ['processes', 'processes', []],
  ['history', 'history', []],
  ['compare-empty', 'compare', []],
  ['compare', 'history', ['select-two', 'open-compare']],
  ['applications', 'applications', []],
  ['clean', 'clean', []],
  ['clean-preview', 'clean', ['clean-preview']],
  ['settings', 'settings', []],
  ['about', 'about', []],
  ['report-modal', 'overview', ['open-latest-report']],
];

export const scenarios = [
  ...pages.flatMap(([id, page, actions, setup = {}]) =>
    ['light', 'dark'].map((theme) => ({ id, page, actions, setup, theme, locale: 'fr' })),
  ),
  ...['en', 'ar', 'ja', 'zh-CN', 'hi'].flatMap((locale) =>
    ['overview', 'diagnostics', 'settings', 'processes'].map((page) => ({
      id: page,
      page,
      actions: [],
      setup: {},
      theme: 'light',
      locale,
    })),
  ),
  ...['overview', 'processes', 'history'].map((page) => ({
    id: `${page}-narrow`,
    page,
    actions: [],
    setup: {},
    theme: 'light',
    locale: 'fr',
    width: 1000,
    height: 680,
  })),
  { id: 'popover-memory', popover: 'memory', actions: [], setup: {}, theme: 'light', locale: 'fr' },
  { id: 'popover-memory', popover: 'memory', actions: [], setup: {}, theme: 'dark', locale: 'fr' },
  { id: 'popover-cpu', popover: 'cpu', actions: [], setup: {}, theme: 'light', locale: 'fr' },
];

export const captureName = (scenario, step = 0) =>
  `${scenario.id}-${scenario.theme}-${scenario.locale}-s${step}.png`;
