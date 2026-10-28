import { t, tx, currentLocale } from '../i18n.mjs';
import { useState } from 'react';
import {
  Activity,
  ArrowRight,
  Check,
  CheckCheck,
  CircleHelp,
  Cpu,
  HardDrive,
  LockKeyhole,
  MemoryStick,
  Play,
  RotateCcw,
  ScanLine,
  ShieldCheck,
  Square,
  Timer,
  Zap,
} from 'lucide-react';
import { useApp } from '../app-context';
import {
  CheckCard,
  Notice,
  PanelHeader,
  Progress,
  ReportDetails,
  SectionTitle,
  Spinner,
} from '../components';
import { formatBytes } from '../domain.mjs';
import type { RunOptions } from '../types';
import OfflineDiagnostic from './OfflineDiagnostic';
const components = [
  {
    id: 'memory',
    name: 'Mémoire vive',
    description: 'Motifs, inversions mobiles et compteurs ECC accessibles.',
    icon: MemoryStick,
  },
  {
    id: 'storage',
    name: 'Stockage',
    description: 'SMART / NVMe et intégrité d’un fichier temporaire.',
    icon: HardDrive,
  },
  {
    id: 'cpu',
    name: 'Processeur',
    description: 'Calculs parallèles comparés à des réponses connues.',
    icon: Cpu,
  },
];
export default function Diagnostics() {
  const { preferences, job, launching, start, cancel, inventory, error, exportReport } = useApp();
  const [selected, setSelected] = useState(['memory', 'storage', 'cpu']);
  const [profile, setProfile] = useState('standard');
  const [options, setOptions] = useState({
    ...preferences,
  });
  const [label, setLabel] = useState('');
  const [showConfiguration, setShowConfiguration] = useState(true);
  const presets = {
    quick: {
      memory_mib: 64,
      passes: 1,
      disk_mib: 16,
      cpu_seconds: 2,
    },
    standard: preferences,
    extended: {
      memory_mib: 512,
      passes: 4,
      disk_mib: 128,
      cpu_seconds: 15,
    },
  };
  const chooseProfile = (value: keyof typeof presets) => {
    setProfile(value);
    setOptions({
      ...options,
      ...presets[value],
    });
  };
  const budget = inventory
    ? Math.floor(
        (inventory.memory.test_budget_bytes ??
          Math.min(inventory.memory.available_bytes / 3, inventory.memory.total_bytes / 4)) /
          1024 ** 2,
      )
    : 0;
  const exceeds = selected.includes('memory') && !!inventory && options.memory_mib > budget;
  const run = async () => {
    setShowConfiguration(false);
    const params: RunOptions = {
      components: selected,
      memory_mib: options.memory_mib,
      passes: options.passes,
      disk_mib: options.disk_mib,
      cpu_seconds: options.cpu_seconds,
      label,
    };
    await start(params);
  };
  const edit = (key: string, value: number) => {
    setOptions({
      ...options,
      [key]: value,
    });
    setProfile('custom');
  };
  return (
    <div className="page-enter">
      <SectionTitle
        eyebrow={tx('PRENDRE SOIN DE SON PC')}
        title={tx('Un diagnostic qui a du sens.')}
        description={tx(
          'Des contrôles réels, des résultats expliqués et un historique pour avancer.',
        )}
      />
      {tx(
        job?.running ? (
          <>
            <section className="running-panel">
              <div className="running-header">
                <span className="running-orb">
                  <ScanLine size={29} />
                </span>
                <div>
                  <span className="eyebrow">{tx('LE MOTEUR RUST EST AU TRAVAIL')}</span>
                  <h2>
                    {tx(
                      job.cancel_requested
                        ? 'Arrêt du diagnostic…'
                        : 'On regarde tout ça de plus près.',
                    )}
                  </h2>
                  <p>{tx(job.phase)}</p>
                </div>
                <span className="running-percentage">
                  {tx(Math.round(job.progress * 100))}
                  <small>{tx('%')}</small>
                </span>
              </div>
              <Progress value={job.progress} label={tx('Progression mesurée')} />
              <div className="running-footer">
                <span>
                  <LockKeyhole size={14} />
                  {tx('Vos fichiers personnels ne sont pas modifi\xE9s.')}
                </span>
                <button
                  className="button secondary small"
                  disabled={job.cancel_requested}
                  onClick={() => void cancel()}
                >
                  {tx(job.cancel_requested ? <Spinner size={14} /> : <Square size={13} />)}{' '}
                  {tx(job.cancel_requested ? 'Arrêt en cours' : 'Arrêter le diagnostic')}
                </button>
              </div>
            </section>
            <section className="panel">
              <PanelHeader
                title={tx('Les résultats arrivent au fil des contrôles')}
                subtitle={tx('La progression suit les opérations réellement exécutées.')}
              />
              {tx(
                job.checks.length ? (
                  <div className="checks-stack">
                    {tx(job.checks.map((check) => <CheckCard key={check.id} check={check} />))}
                  </div>
                ) : (
                  <div className="live-wait">
                    <Spinner />
                    <span>
                      {tx('Pr\xE9paration de l\u2019inventaire et du premier contr\xF4le\u2026')}
                    </span>
                  </div>
                ),
              )}
            </section>
            <Notice>
              {tx(
                'Vous pouvez continuer \xE0 parcourir Computy. L\u2019arr\xEAt lib\xE8re les ressources et conserve les r\xE9sultats partiels ; un contr\xF4le interrompu ne re\xE7oit pas de note de r\xE9ussite.',
              )}
            </Notice>
          </>
        ) : job?.report && !showConfiguration ? (
          <section className="panel completed-panel">
            <PanelHeader
              title={tx(
                job.report.completed
                  ? 'Votre bilan est prêt.'
                  : 'Votre diagnostic a été interrompu.',
              )}
              action={
                <button className="button primary small" onClick={() => setShowConfiguration(true)}>
                  <RotateCcw size={15} />
                  {tx('Nouveau diagnostic')}
                </button>
              }
            />
            {tx(job.error && <Notice tone="warn">{tx(job.error)}</Notice>)}
            <ReportDetails report={job.report} onExport={exportReport} />
          </section>
        ) : (
          <>
            <div className="diagnostic-layout">
              <div className="diagnostic-form">
                <section className="panel">
                  <PanelHeader
                    title={tx('01. Que souhaitez-vous vérifier ?')}
                    subtitle={tx('Chaque composant conserve ses résultats et ses limites.')}
                  />
                  <div className="test-component-options">
                    {tx(
                      components.map(({ id, name, description, icon: Icon }) => (
                        <label
                          key={id}
                          className={`test-component-option ${selected.includes(id) ? 'selected' : ''}`}
                        >
                          <input
                            type="checkbox"
                            checked={selected.includes(id)}
                            onChange={() =>
                              setSelected((previous) =>
                                previous.includes(id)
                                  ? previous.filter((v) => v !== id)
                                  : [...previous, id],
                              )
                            }
                          />
                          <span className="checkbox-visual">
                            <Check size={13} />
                          </span>
                          <span
                            className={`component-icon ${id === 'memory' ? 'sage' : id === 'storage' ? 'sand' : 'lilac'}`}
                          >
                            <Icon size={23} />
                          </span>
                          <span>
                            <b>{tx(name)}</b>
                            <small>{tx(description)}</small>
                          </span>
                        </label>
                      )),
                    )}
                  </div>
                </section>
                <section className="panel">
                  <PanelHeader
                    title={tx('02. Choisissez votre profondeur de test')}
                    subtitle={tx(
                      'Plus de mémoire et de passes augmentent la couverture, ainsi que la durée.',
                    )}
                  />
                  <div className="profile-grid">
                    {tx(
                      (
                        [
                          {
                            id: 'quick',
                            name: 'Premier regard',
                            icon: Zap,
                            detail: '64 Mio · 1 passe',
                            tag: 'LÉGER',
                          },
                          {
                            id: 'standard',
                            name: 'L’essentiel',
                            icon: ScanLine,
                            detail: t('{{v0}} Mio \xB7 {{v1}} passes', {
                              v0: preferences.memory_mib,
                              v1: preferences.passes,
                            }),
                            tag: 'CONSEILLÉ',
                          },
                          {
                            id: 'extended',
                            name: 'Plus en détail',
                            icon: LayersIcon,
                            detail: '512 Mio · 4 passes',
                            tag: 'APPROFONDI',
                          },
                        ] as const
                      ).map(({ id, name, icon: Icon, detail, tag }) => (
                        <button
                          key={id}
                          className={`profile ${profile === id ? 'selected' : ''}`}
                          onClick={() => chooseProfile(id)}
                        >
                          <span className="profile-top">
                            <Icon size={20} />
                            <span className="profile-radio">{tx(profile === id && <i />)}</span>
                          </span>
                          <span className="profile-tag">{tx(tag)}</span>
                          <b>{tx(name)}</b>
                          <small>{tx(detail)}</small>
                        </button>
                      )),
                    )}
                  </div>
                  <details className="advanced-options">
                    <summary>
                      {tx('Personnaliser les param\xE8tres ')}
                      <span>
                        {tx(profile === 'custom' ? 'Paramètres personnalisés' : 'Optionnel')}
                      </span>
                    </summary>
                    <div className="fields-grid">
                      <label>
                        {tx('M\xE9moire \xE0 allouer')}
                        <select
                          value={options.memory_mib}
                          onChange={(e) => edit('memory_mib', Number(e.target.value))}
                        >
                          {tx(
                            [16, 32, 64, 128, 256, 512, 1024, 2048].map((value) => (
                              <option value={value} key={value}>
                                {tx(value)}
                                {tx(' Mio')}
                              </option>
                            )),
                          )}
                        </select>
                      </label>
                      <label>
                        {tx('Passes m\xE9moire')}
                        <select
                          value={options.passes}
                          onChange={(e) => edit('passes', Number(e.target.value))}
                        >
                          {tx(
                            [1, 2, 3, 4, 6, 8].map((value) => (
                              <option value={value} key={value}>
                                {tx(value)}
                              </option>
                            )),
                          )}
                        </select>
                      </label>
                      <label>
                        {tx('Fichier d\u2019int\xE9grit\xE9')}
                        <select
                          value={options.disk_mib}
                          onChange={(e) => edit('disk_mib', Number(e.target.value))}
                        >
                          {tx(
                            [16, 32, 64, 128, 256, 512].map((value) => (
                              <option value={value} key={value}>
                                {tx(value)}
                                {tx(' Mio')}
                              </option>
                            )),
                          )}
                        </select>
                      </label>
                      <label>
                        {tx('Dur\xE9e des calculs CPU')}
                        <select
                          value={options.cpu_seconds}
                          onChange={(e) => edit('cpu_seconds', Number(e.target.value))}
                        >
                          {tx(
                            [1, 2, 5, 10, 15, 30, 60].map((value) => (
                              <option value={value} key={value}>
                                {tx(value)}
                                {tx(' secondes')}
                              </option>
                            )),
                          )}
                        </select>
                      </label>
                    </div>
                  </details>
                </section>
                <section className="panel label-panel">
                  <PanelHeader
                    title={tx('03. Donnez un nom à ce moment')}
                    subtitle={tx('Un repère simple pour vos futures comparaisons.')}
                  />
                  <label className="field-label" htmlFor="run-label">
                    {tx('Nom du diagnostic ')}
                    <span>{tx('facultatif')}</span>
                  </label>
                  <input
                    id="run-label"
                    className="text-input"
                    placeholder={tx('Ex. Avant remplacement de la RAM')}
                    value={label}
                    maxLength={100}
                    onChange={(e) => setLabel(e.target.value)}
                  />
                </section>
              </div>
              <aside className="run-summary">
                <div className="panel summary-panel">
                  <span className="summary-symbol">
                    <ShieldCheck size={25} />
                  </span>
                  <h2>{tx('Pr\xEAt \xE0 faire le point ?')}</h2>
                  <p>
                    {tx(
                      'Un instantan\xE9 du mat\xE9riel et des r\xE9sultats sera enregistr\xE9 sur cet ordinateur.',
                    )}
                  </p>
                  <dl className="run-summary-list">
                    <div>
                      <dt>{tx('Composants')}</dt>
                      <dd>{tx(selected.length)}</dd>
                    </div>
                    {tx(
                      selected.includes('memory') && (
                        <>
                          <div>
                            <dt>{tx('M\xE9moire test\xE9e')}</dt>
                            <dd>
                              {tx(options.memory_mib)}
                              {tx(' Mio')}
                            </dd>
                          </div>
                          <div>
                            <dt>{tx('Passes m\xE9moire')}</dt>
                            <dd>{tx(options.passes)}</dd>
                          </div>
                        </>
                      ),
                    )}
                    {tx(
                      selected.includes('storage') && (
                        <div>
                          <dt>{tx('Fichier temporaire')}</dt>
                          <dd>
                            {tx(options.disk_mib)}
                            {tx(' Mio')}
                          </dd>
                        </div>
                      ),
                    )}
                    {tx(
                      selected.includes('cpu') && (
                        <div>
                          <dt>{tx('Calculs CPU')}</dt>
                          <dd>
                            {tx(options.cpu_seconds)}
                            {tx(' s \xB7 jusqu\u2019\xE0 4 threads')}
                          </dd>
                        </div>
                      ),
                    )}
                  </dl>
                  {tx(
                    exceeds && (
                      <Notice tone="warn">
                        {tx(
                          'Allocation trop \xE9lev\xE9e pour la m\xE9moire disponible. Choisissez au maximum ',
                        )}
                        {tx(budget)}
                        {tx(' Mio dans les param\xE8tres.')}
                      </Notice>
                    ),
                  )}
                  <button
                    className="button primary full launch-button"
                    disabled={launching || !selected.length || !!error || !inventory || exceeds}
                    onClick={() => void run()}
                  >
                    {tx(launching ? <Spinner size={18} /> : <Play size={17} fill="currentColor" />)}
                    {tx(launching ? 'Préparation…' : 'Lancer le diagnostic')}
                    <ArrowRight size={17} />
                  </button>
                  <span className="summary-reassurance">
                    <LockKeyhole size={12} />
                    {tx('Ex\xE9cution locale \xB7 Historique automatique')}
                  </span>
                </div>
                <div className="scope-card">
                  <CircleHelp size={20} />
                  <h3>{tx('Ce qu\u2019on peut vraiment v\xE9rifier')}</h3>
                  <p>
                    {tx(
                      'Dans l\u2019OS, le test m\xE9moire couvre une allocation limit\xE9e. Pour aller plus loin, compl\xE9tez-le par un test au d\xE9marrage.',
                    )}
                  </p>
                  <p>
                    {tx(
                      'La note r\xE9sume les contr\xF4les observ\xE9s, sans promettre une fiabilit\xE9 absolue.',
                    )}
                  </p>
                </div>
              </aside>
            </div>
            <OfflineDiagnostic />
          </>
        ),
      )}
    </div>
  );
}
function LayersIcon(props: React.ComponentProps<typeof Activity>) {
  return <CheckCheck {...props} />;
}
