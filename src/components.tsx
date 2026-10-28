import { t, tx, currentLocale } from './i18n.mjs';
import { useEffect, useRef, type ReactNode } from 'react';
import {
  ArrowDownToLine,
  Check,
  ChevronRight,
  CircleHelp,
  Frown,
  Meh,
  Smile,
  X,
  LoaderCircle,
  Info,
  TriangleAlert,
  Minus,
  ShieldCheck,
  SearchCheck,
} from 'lucide-react';
import {
  duration,
  formatDate,
  statusInfo,
  scoreStatus,
  checkOutcome,
  formatBytes,
} from './domain.mjs';
import type { Check as DiagnosticCheck, Report, Status } from './types';
export function Logo({ size = 44 }: { size?: number }) {
  return (
    <span
      className="brand-mark"
      style={{
        width: size,
        height: size,
      }}
    >
      <img
        src="./computy-logo.png"
        alt={tx('')}
        onError={(e) => {
          e.currentTarget.style.display = 'none';
          e.currentTarget.parentElement?.classList.add('logo-fallback');
        }}
      />
      <span aria-hidden="true" className="fallback-letter">
        {tx('c')}
      </span>
    </span>
  );
}
export function StatusBadge({
  status,
  compact = false,
  checks,
  completed = true,
}: {
  status: Status;
  compact?: boolean;
  checks?: DiagnosticCheck[];
  completed?: boolean;
}) {
  const outcome = checks ? checkOutcome(checks, completed) : null;
  status = outcome?.status ?? status;
  const info = statusInfo(status);
  const Icon =
    status === 'passed'
      ? Check
      : status === 'failed' || status === 'warning'
        ? TriangleAlert
        : Minus;
  return (
    <span className="result-badges" data-result-status={status}>
      <span
        className={`badge ${info.tone}`}
        title={checks
          ?.filter((check) => check.status === status)
          .map((check) => `${tx(check.name)}: ${tx(check.message)}`)
          .join('\n')}
      >
        <Icon size={12} />
        {tx(
          outcome && status === 'passed'
            ? 'Aucune erreur détectée'
            : compact
              ? info.short
              : info.label,
        )}
      </span>
      {!!outcome?.unavailable.length && (
        <span
          className="badge muted evidence-missing"
          title={outcome.unavailable.map(tx).join('\n')}
        >
          {t('Non vérifiés : {{count}}', { count: outcome.unavailable.length })}
        </span>
      )}
    </span>
  );
}
export function Mood({ status, size = 24 }: { status: Status; size?: number }) {
  const Icon = status === 'passed' ? Smile : status === 'failed' ? Frown : Meh;
  return (
    <Icon
      size={size}
      className={`mood ${statusInfo(status).tone}`}
      strokeWidth={1.7}
      aria-label={tx(statusInfo(status).label)}
    />
  );
}
export function Score({
  score,
  status,
  large = false,
}: {
  score: number | null;
  status: Status;
  large?: boolean;
}) {
  return (
    <div className={`score ${large ? 'large' : ''}`}>
      <Mood status={scoreStatus(score, status)} size={large ? 38 : 22} />
      <span>
        {tx(score === null ? '—' : score)}
        <small>{tx('/ 10')}</small>
      </span>
    </div>
  );
}
export function Empty({
  icon: Icon = SearchCheck,
  title,
  children,
  action,
}: {
  icon?: typeof SearchCheck;
  title: string;
  children: ReactNode;
  action?: ReactNode;
}) {
  return (
    <div className="empty-state">
      <span className="empty-icon">
        <Icon size={27} strokeWidth={1.5} />
      </span>
      <h3>{tx(title)}</h3>
      <p>{tx(children)}</p>
      {tx(action)}
    </div>
  );
}
export function Notice({
  children,
  tone = 'info',
}: {
  children: ReactNode;
  tone?: 'info' | 'warn';
}) {
  const Icon = tone === 'warn' ? TriangleAlert : Info;
  return (
    <div className={`notice ${tone}`}>
      <Icon size={17} />
      <div>{tx(children)}</div>
    </div>
  );
}
export function SectionTitle({
  eyebrow,
  title,
  description,
  action,
}: {
  eyebrow?: string;
  title: string;
  description?: string;
  action?: ReactNode;
}) {
  return (
    <div className="page-heading">
      <div>
        {tx(eyebrow && <p className="eyebrow">{tx(eyebrow)}</p>)}
        <h1>{tx(title)}</h1>
        {tx(description && <p className="page-description">{tx(description)}</p>)}
      </div>
      {tx(action)}
    </div>
  );
}
export function PanelHeader({
  title,
  subtitle,
  action,
}: {
  title: string;
  subtitle?: string;
  action?: ReactNode;
}) {
  return (
    <div className="panel-header">
      <div>
        <h2>{tx(title)}</h2>
        {tx(subtitle && <p>{tx(subtitle)}</p>)}
      </div>
      {tx(action)}
    </div>
  );
}
export function Progress({ value, label }: { value: number; label?: string }) {
  return (
    <div className="progress-group">
      {tx(
        label && (
          <div className="progress-label">
            <span>{tx(label)}</span>
            <b>
              {tx(Math.round(value * 100))}
              {tx(' %')}
            </b>
          </div>
        ),
      )}
      <div
        className="progress-track"
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(value * 100)}
        aria-label={tx(label || 'Progression du diagnostic')}
      >
        <span
          style={{
            width: `${Math.max(0, Math.min(100, value * 100))}%`,
          }}
        />
      </div>
    </div>
  );
}
export function Spinner({ size = 18 }: { size?: number }) {
  return <LoaderCircle className="spin" size={size} aria-hidden="true" />;
}
export function Modal({
  title,
  children,
  onClose,
  wide = false,
}: {
  title: string;
  children: ReactNode;
  onClose: () => void;
  wide?: boolean;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const closeRef = useRef(onClose);
  closeRef.current = onClose;
  useEffect(() => {
    const previous = document.activeElement as HTMLElement;
    const dialog = ref.current;
    dialog?.querySelector<HTMLElement>('button,input,a')?.focus();
    const key = (event: KeyboardEvent) => {
      if (event.key === 'Escape') closeRef.current();
      if (event.key === 'Tab' && dialog) {
        const focusable = Array.from(
          dialog.querySelectorAll<HTMLElement>(
            'button:not([disabled]),a,input,select,textarea,[tabindex="0"]',
          ),
        );
        const first = focusable[0],
          last = focusable.at(-1);
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last?.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first?.focus();
        }
      }
    };
    document.addEventListener('keydown', key);
    const overflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    return () => {
      document.removeEventListener('keydown', key);
      document.body.style.overflow = overflow;
      previous?.focus();
    };
  }, []);
  return (
    <div
      className="modal-backdrop"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className={`modal ${wide ? 'wide' : ''}`}
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-label={tx(title)}
      >
        <div className="modal-heading">
          <h2>{tx(title)}</h2>
          <button className="icon-button" onClick={onClose} aria-label={tx('Fermer')}>
            <X size={20} />
          </button>
        </div>
        {tx(children)}
      </div>
    </div>
  );
}
const metricLabels: Record<string, string> = {
  allocated_bytes: 'Zone allouée (octets)',
  verified_bytes: 'Octets vérifiés',
  errors: 'Erreurs',
  patterns_completed: 'Motifs terminés',
  patterns_planned: 'Motifs prévus',
  passes_completed: 'Passes terminées',
  locked: 'Pages verrouillées en RAM',
  corrected: 'Erreurs ECC corrigées',
  uncorrected: 'Erreurs ECC non corrigées',
  smartctl_exit_status: 'Code SMART (masque de bits)',
  temperature_c: 'Température (°C)',
  power_on_hours: 'Heures de fonctionnement',
  device: 'Périphérique',
  model: 'Modèle',
  file_size_bytes: 'Taille du fichier (octets)',
  mismatched_bytes: 'Octets divergents',
  write_ms: 'Écriture + synchronisation (ms)',
  read_ms: 'Relecture + comparaison (ms)',
  directory: 'Volume de test',
  cache_eviction_advised: 'Éviction du cache demandée',
  workers: 'Threads de calcul',
  iterations: 'Calculs vérifiés',
  hashed_bytes: 'Octets hachés',
  algorithm: 'Algorithme',
  first_error: 'Première incohérence',
};
export function CheckCard({
  check,
  expanded = false,
}: {
  check: DiagnosticCheck;
  expanded?: boolean;
}) {
  return (
    <details className="check-card" open={expanded || undefined}>
      <summary>
        <span className={`check-icon ${statusInfo(check.status).tone}`}>
          {tx(
            check.status === 'passed' ? (
              <ShieldCheck size={20} />
            ) : check.status === 'failed' ? (
              <TriangleAlert size={20} />
            ) : (
              <CircleHelp size={20} />
            ),
          )}
        </span>
        <span className="check-title">
          <b>{tx(check.name)}</b>
          <span>{tx(check.message)}</span>
        </span>
        <StatusBadge status={check.status} />
        <ChevronRight size={17} className="details-chevron" />
      </summary>
      <div className="check-details">
        <dl className="metric-list">
          {tx(
            Object.entries(check.metrics)
              .filter(([, value]) => value !== null && typeof value !== 'object')
              .map(([key, value]) => (
                <div key={key}>
                  <dt>{tx(metricLabels[key] || key)}</dt>
                  <dd>
                    {tx(
                      typeof value === 'boolean'
                        ? value
                          ? 'Oui'
                          : 'Non'
                        : typeof value === 'number'
                          ? value.toLocaleString(currentLocale())
                          : String(value),
                    )}
                  </dd>
                </div>
              )),
          )}
          {tx(
            check.duration_ms > 0 && (
              <div>
                <dt>{tx('Dur\xE9e mesur\xE9e')}</dt>
                <dd>{tx(duration(check.duration_ms))}</dd>
              </div>
            ),
          )}
        </dl>
        {tx(
          check.limitations.length > 0 && (
            <div className="limits">
              <b>{tx('Port\xE9e du contr\xF4le')}</b>
              <ul>{tx(check.limitations.map((note) => <li key={note}>{tx(note)}</li>))}</ul>
            </div>
          ),
        )}
        {tx(
          Object.entries(check.metrics).some(([, v]) => v !== null && typeof v === 'object') && (
            <details className="raw-metrics">
              <summary>{tx('Donn\xE9es techniques compl\xE8tes')}</summary>
              <pre>{tx(JSON.stringify(check.metrics, null, 2))}</pre>
            </details>
          ),
        )}
      </div>
    </details>
  );
}
export function ReportDetails({
  report,
  onExport,
}: {
  report: Report;
  onExport: (report: Report) => void;
}) {
  const outcome = checkOutcome(report.checks, report.completed);
  const memory = report.checks.find((check) => check.id === 'memory-patterns');
  return (
    <>
      <div className="report-intro">
        <div>
          <p className="eyebrow">{tx(formatDate(report.created_at))}</p>
          <h3>{report.label}</h3>
          <p>
            {tx(report.inventory.host_name)}
            {tx(' \xB7 ')}
            {tx(report.inventory.os)}
            {tx(' \xB7 ')}
            {tx(duration(report.duration_ms))}
          </p>
        </div>
        <Score score={report.summary.score} status={report.summary.status} large />
      </div>
      {memory && (
        <div className="report-memory-scope">
          <b>{tx('Mémoire réellement testée')}</b>
          <span>
            {formatBytes(Number(memory.metrics.allocated_bytes ?? 0))} /{' '}
            {formatBytes(report.inventory.memory.total_bytes)} ·{' '}
            {t('Passes terminées : {{count}}', {
              count: Number(memory.metrics.passes_completed ?? 0),
            })}
          </span>
          <p>
            {tx(
              'Ce test s’exécute dans votre session, sans redémarrage. Il ne parcourt pas toute la RAM physique.',
            )}
          </p>
        </div>
      )}
      {outcome.unavailable.length > 0 && (
        <Notice>
          <b>{tx('Informations indisponibles, pas erreurs détectées')}</b>
          <p>
            {tx(
              'Ces contrôles ne sont pas notés. Leur absence ne signifie pas que votre matériel est défectueux.',
            )}
          </p>
          <ul>
            {report.checks
              .filter((check) => check.status === 'unavailable')
              .map((check) => (
                <li key={check.id}>
                  <b>{tx(check.name)}</b> — {tx(check.message)}
                </li>
              ))}
          </ul>
        </Notice>
      )}
      <Notice>
        {tx('La note r\xE9sume les contr\xF4les effectu\xE9s. Couverture : ')}
        <b>
          {tx(report.summary.checked)}
          {tx('/')}
          {tx(report.summary.total)}
          {tx(' contr\xF4les exploitables')}
        </b>
        {tx('. Elle n\u2019est pas une probabilit\xE9 de bon fonctionnement.')}
      </Notice>
      <div className="report-actions">
        <StatusBadge
          status={report.summary.status}
          checks={report.checks}
          completed={report.completed}
        />
        <button className="button secondary small" onClick={() => onExport(report)}>
          <ArrowDownToLine size={15} />
          {tx('Exporter en JSON')}
        </button>
      </div>
      <div className="checks-stack">
        {tx(report.checks.map((check) => <CheckCard key={check.id} check={check} />))}
      </div>
      <details className="report-footnote">
        <summary>{tx('Détails du rapport')}</summary>
        {tx('Moteur Rust \xB7 sch\xE9ma v')}
        {tx(report.schema_version)}
        {tx(' \xB7 ID ')}
        {tx(report.id)}
      </details>
    </>
  );
}
