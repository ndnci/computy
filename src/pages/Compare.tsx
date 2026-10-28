import { t, tx, currentLocale } from '../i18n.mjs';
import {
  ArrowDown,
  ArrowLeftRight,
  ArrowRight,
  ArrowUp,
  Check,
  History,
  Minus,
  Pin,
  Plus,
  X,
} from 'lucide-react';
import { useApp } from '../app-context';
import { Empty, Notice, Score, SectionTitle } from '../components';
import {
  comparisonWarnings,
  componentSummary,
  duration,
  formatBytes,
  formatDate,
} from '../domain.mjs';
import type { Report } from '../types';
const check = (report: Report, id: string) => report.checks.find((c) => c.id === id);
export default function Compare() {
  const { reports, selectedIds, setSelectedIds, navigate, openReport } = useApp();
  const selected = selectedIds
    .map((id) => reports.find((r) => r.id === id))
    .filter((r): r is Report => !!r);
  const warnings = comparisonWarnings(selected);
  const speed = (report: Report) => {
    const values = [
      ...new Set(
        report.inventory.memory.modules.map((m) => m.configured_speed_mts).filter(Boolean),
      ),
    ];
    return values.length
      ? values.map((value) => `${value?.toLocaleString('fr-FR')} MT/s`).join(' / ')
      : 'Non communiqué';
  };
  const rows: {
    section?: string;
    label?: string;
    get?: (report: Report) => any;
    numeric?: (report: Report) => number | null;
    format?: (value: number) => string;
  }[] = [
    {
      section: 'Configuration du matériel',
    },
    {
      label: 'RAM visible par l’OS',
      get: (r) => formatBytes(r.inventory.memory.total_bytes),
      numeric: (r) => r.inventory.memory.total_bytes,
      format: (v) => formatBytes(Math.abs(v)),
    },
    {
      label: 'Modules identifiés',
      get: (r) => r.inventory.memory.modules.length || 'Non communiqués',
    },
    {
      label: 'Type de mémoire',
      get: (r) =>
        [...new Set(r.inventory.memory.modules.map((m) => m.kind))].join(' / ') || 'Non communiqué',
    },
    {
      label: 'Débit mémoire configuré',
      get: speed,
    },
    {
      label: 'Références des modules',
      get: (r) =>
        r.inventory.memory.modules.map((m) => m.part_number || 'Non communiquée').join(' · ') ||
        'Non communiquées',
    },
    {
      label: 'Processeur',
      get: (r) => r.inventory.cpu.name || 'Non communiqué',
    },
    {
      label: 'Système',
      get: (r) => `${r.inventory.os} ${r.inventory.os_version} · ${r.inventory.arch}`,
    },
    {
      section: 'Ce qui a réellement été testé',
    },
    {
      label: 'Allocation mémoire vérifiée',
      get: (r) =>
        check(r, 'memory-patterns')
          ? formatBytes(check(r, 'memory-patterns')?.metrics.allocated_bytes)
          : 'Non exécuté',
    },
    {
      label: 'Passes mémoire terminées',
      get: (r) => check(r, 'memory-patterns')?.metrics.passes_completed ?? 'Non exécuté',
    },
    {
      label: 'Incohérences mémoire',
      get: (r) => check(r, 'memory-patterns')?.metrics.errors ?? 'Non mesurées',
    },
    {
      label: 'Résultat mémoire',
      get: (r) => <Score {...componentSummary(r.checks, 'memory')} />,
    },
    {
      label: 'Octets disque vérifiés',
      get: (r) =>
        check(r, 'disk-roundtrip')
          ? formatBytes(check(r, 'disk-roundtrip')?.metrics.verified_bytes)
          : 'Non exécuté',
    },
    {
      label: 'Octets disque divergents',
      get: (r) => check(r, 'disk-roundtrip')?.metrics.mismatched_bytes ?? 'Non mesurés',
    },
    {
      label: 'Résultat stockage',
      get: (r) => <Score {...componentSummary(r.checks, 'storage')} />,
    },
    {
      label: 'Calculs CPU vérifiés',
      get: (r) =>
        check(r, 'cpu-known-answers')?.metrics.iterations?.toLocaleString('fr-FR') ?? 'Non exécuté',
    },
    {
      label: 'Durée du contrôle CPU',
      get: (r) =>
        check(r, 'cpu-known-answers')
          ? duration(check(r, 'cpu-known-answers')!.duration_ms)
          : 'Non exécuté',
    },
    {
      label: 'Erreurs de calcul',
      get: (r) => check(r, 'cpu-known-answers')?.metrics.errors ?? 'Non mesurées',
    },
    {
      label: 'Résultat processeur',
      get: (r) => <Score {...componentSummary(r.checks, 'cpu')} />,
    },
    {
      section: 'Le bilan dans son contexte',
    },
    {
      label: 'Durée totale',
      get: (r) => duration(r.duration_ms),
    },
    {
      label: 'Contrôles exploitables',
      get: (r) => `${r.summary.checked} / ${r.summary.total}`,
    },
    {
      label: 'Note des contrôles',
      get: (r) => <Score score={r.summary.score} status={r.summary.status} />,
    },
  ];
  return (
    <div className="page-enter">
      <SectionTitle
        eyebrow={tx('AVANT, APRÈS, ET PLUS ENCORE')}
        title={tx('Les changements, côte à côte.')}
        description={tx(
          'Comparez vos configurations et les résultats observés, sans perdre le contexte.',
        )}
        action={
          <button className="button secondary" onClick={() => navigate('history')}>
            <Plus size={17} />
            {tx('Choisir des bilans')}
          </button>
        }
      />
      {tx(
        selected.length < 2 ? (
          <section className="panel compare-empty">
            <Empty
              icon={ArrowLeftRight}
              title={tx('Deux moments. Une vision plus claire.')}
              action={
                <button className="button primary" onClick={() => navigate('history')}>
                  <History size={16} />
                  {tx('S\xE9lectionner des diagnostics ')}
                  <ArrowRight size={16} />
                </button>
              }
            >
              {tx(
                'S\xE9lectionnez entre deux et quatre diagnostics dans votre historique. Id\xE9al avant et apr\xE8s un changement de mat\xE9riel.',
              )}
            </Empty>
          </section>
        ) : (
          <>
            {warnings.map((warning) => (
              <Notice tone="warn" key={warning}>
                {tx(warning)}
              </Notice>
            ))}
            <div className="compare-legend">
              <span>
                <span className="reference-dot" />
                {tx('Le premier bilan est la r\xE9f\xE9rence.')}
              </span>
              <p>
                {tx(
                  'Un \xE9cart de capacit\xE9 ou de performance n\u2019est pas une preuve de meilleure sant\xE9.',
                )}
              </p>
            </div>
            <div className="panel comparison-panel table-scroll">
              <table className="comparison-table">
                <thead>
                  <tr>
                    <th>
                      <span className="eyebrow">{tx('VOS REP\xC8RES')}</span>
                      <h3>
                        {tx(selected.length)}
                        {tx(' diagnostics')}
                      </h3>
                      <p>
                        {tx('Les m\xEAmes questions,')}
                        <br />
                        {tx('\xE0 diff\xE9rents moments.')}
                      </p>
                    </th>
                    {tx(
                      selected.map((report, index) => (
                        <th key={report.id} className={index === 0 ? 'reference-column' : ''}>
                          <div className="compare-column-top">
                            <span className={`badge ${index === 0 ? 'good' : 'neutral'}`}>
                              {tx(
                                index === 0 ? (
                                  <>
                                    <Pin size={11} />
                                    {tx('R\xE9f\xE9rence')}
                                  </>
                                ) : (
                                  `Bilan ${index + 1}`
                                ),
                              )}
                            </span>
                            <button
                              className="icon-button"
                              aria-label={tx(`Retirer ${report.label}`)}
                              onClick={() =>
                                setSelectedIds(selectedIds.filter((id) => id !== report.id))
                              }
                            >
                              <X size={14} />
                            </button>
                          </div>
                          <h3>{report.label}</h3>
                          <p>{tx(formatDate(report.created_at, true))}</p>
                          <span className="compare-machine">{tx(report.inventory.host_name)}</span>
                          {tx(
                            index > 0 && (
                              <button
                                className="text-button small"
                                onClick={() =>
                                  setSelectedIds([
                                    report.id,
                                    ...selectedIds.filter((id) => id !== report.id),
                                  ])
                                }
                              >
                                {tx('D\xE9finir comme r\xE9f\xE9rence')}
                              </button>
                            ),
                          )}
                        </th>
                      )),
                    )}
                  </tr>
                </thead>
                <tbody>
                  {tx(
                    rows.map((row, index) =>
                      row.section ? (
                        <tr className="comparison-section" key={row.section}>
                          <th colSpan={selected.length + 1}>{tx(row.section)}</th>
                        </tr>
                      ) : (
                        <tr key={index}>
                          <th>{tx(row.label)}</th>
                          {tx(
                            selected.map((report, column) => {
                              const value = row.numeric?.(report),
                                base = row.numeric?.(selected[0]);
                              const delta =
                                value !== null &&
                                value !== undefined &&
                                base !== null &&
                                base !== undefined
                                  ? value - base
                                  : 0;
                              return (
                                <td
                                  key={report.id}
                                  className={column === 0 ? 'reference-column' : ''}
                                >
                                  <span className="comparison-value">{tx(row.get?.(report))}</span>
                                  {tx(
                                    column > 0 && row.numeric && delta !== 0 && (
                                      <span className="delta">
                                        {tx(
                                          delta > 0 ? (
                                            <ArrowUp size={11} />
                                          ) : (
                                            <ArrowDown size={11} />
                                          ),
                                        )}{' '}
                                        {tx(row.format?.(delta))}
                                        {tx(' par rapport \xE0 la r\xE9f\xE9rence')}
                                      </span>
                                    ),
                                  )}
                                </td>
                              );
                            }),
                          )}
                        </tr>
                      ),
                    ),
                  )}
                </tbody>
                <tfoot>
                  <tr>
                    <th>{tx('Consulter les preuves')}</th>
                    {tx(
                      selected.map((report) => (
                        <td key={report.id}>
                          <button className="text-button" onClick={() => openReport(report)}>
                            {tx('Ouvrir le rapport ')}
                            <ArrowRight size={14} />
                          </button>
                        </td>
                      )),
                    )}
                  </tr>
                </tfoot>
              </table>
            </div>
            <Notice>
              {tx(
                'Pour comparer les performances, gardez les m\xEAmes param\xE8tres et des conditions proches : alimentation, charge du syst\xE8me, temp\xE9rature et applications ouvertes. Les r\xE9sultats d\u2019int\xE9grit\xE9 et les indicateurs SMART restent \xE0 examiner s\xE9par\xE9ment.',
              )}
            </Notice>
          </>
        ),
      )}
    </div>
  );
}
