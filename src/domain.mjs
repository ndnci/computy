import { t, currentLocale } from './i18n.mjs';
export function formatBytes(value, digits = 1) {
  if (value === null || value === undefined || !Number.isFinite(value)) return t('Non communiqué');
  const units =
    currentLocale() === 'fr'
      ? ['o', 'Kio', 'Mio', 'Gio', 'Tio']
      : ['B', 'KiB', 'MiB', 'GiB', 'TiB'];
  if (value === 0) return `${(0).toLocaleString(currentLocale())} ${units[0]}`;
  const index = Math.min(
    Math.max(Math.floor(Math.log(value) / Math.log(1024)), 0),
    units.length - 1,
  );
  return `${(value / 1024 ** index).toLocaleString(currentLocale(), { maximumFractionDigits: digits })} ${units[index]}`;
}
export function statusInfo(status) {
  return (
    {
      passed: { label: 'Conforme au test', short: 'Conforme', tone: 'good' },
      warning: { label: 'À examiner', short: 'À examiner', tone: 'warn' },
      failed: { label: 'Anomalie détectée', short: 'Anomalie', tone: 'bad' },
      cancelled: { label: 'Interrompu', short: 'Interrompu', tone: 'muted' },
      unavailable: { label: 'Non vérifié', short: 'Non vérifié', tone: 'muted' },
    }[status] || { label: 'Non vérifié', short: 'Non vérifié', tone: 'muted' }
  );
}
export function componentSummary(checks, component) {
  const relevant = checks.filter((check) => check.component === component);
  const known = relevant.filter((check) => ['passed', 'warning', 'failed'].includes(check.status));
  const score = known.length
    ? Math.min(...known.map((check) => ({ passed: 10, warning: 5, failed: 0 })[check.status]))
    : null;
  const status =
    score === 0
      ? 'failed'
      : score === null
        ? 'unavailable'
        : score === 5 || known.length < relevant.length
          ? 'warning'
          : 'passed';
  return { score, status, checked: known.length, total: relevant.length };
}
export function comparisonWarnings(reports) {
  const warnings = [];
  if (new Set(reports.map((r) => r.inventory.machine_id)).size > 1)
    warnings.push(
      'Les rapports proviennent de machines différentes, ou leur identité système a changé.',
    );
  const protocol = (report) =>
    JSON.stringify({
      source: report.source || 'computy_live',
      components: report.options.components?.slice().sort(),
      memory_mib: report.options.memory_mib,
      passes: report.options.passes,
      cpu_seconds: report.options.cpu_seconds,
      disk_mib: report.options.disk_mib,
    });
  if (new Set(reports.map(protocol)).size > 1)
    warnings.push(
      'Les paramètres de test diffèrent. Les performances ne sont pas directement comparables.',
    );
  if (reports.some((r) => !r.completed))
    warnings.push('Un diagnostic a été interrompu : sa couverture est incomplète.');
  return warnings;
}
export function formatDate(value, compact = false) {
  return new Intl.DateTimeFormat(
    currentLocale(),
    compact
      ? { day: '2-digit', month: 'short', hour: '2-digit', minute: '2-digit' }
      : { day: 'numeric', month: 'long', year: 'numeric', hour: '2-digit', minute: '2-digit' },
  ).format(new Date(value));
}
export function duration(ms) {
  const unit = (value, name) =>
    new Intl.NumberFormat(currentLocale(), {
      style: 'unit',
      unit: name,
      unitDisplay: 'short',
      maximumFractionDigits: 1,
    }).format(value);
  return ms < 1000
    ? unit(Math.round(ms), 'millisecond')
    : ms < 60000
      ? unit(ms / 1000, 'second')
      : `${unit(Math.floor(ms / 60000), 'minute')} ${unit(Math.round((ms % 60000) / 1000), 'second')}`;
}
