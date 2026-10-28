import { t, tx, currentLocale } from '../i18n.mjs';
import {
  Activity,
  ArrowRight,
  ArrowUpRight,
  Check,
  ChevronRight,
  Clock3,
  Cpu,
  HardDrive,
  History,
  MemoryStick,
  Play,
  Plus,
  ScanLine,
  ShieldCheck,
  Sparkles,
} from 'lucide-react';
import { useApp } from '../app-context';
import {
  Empty,
  Logo,
  Mood,
  PanelHeader,
  Progress,
  Score,
  SectionTitle,
  Spinner,
  StatusBadge,
} from '../components';
import { componentSummary, duration, formatBytes, formatDate } from '../domain.mjs';
export default function Overview() {
  const { inventory, loading, reports, navigate, openReport, job } = useApp();
  const latest = reports[0];
  const memory = inventory?.memory;
  const disk =
    inventory?.disks.find((d) => d.mount_point === '/' || d.mount_point.toLowerCase() === 'c:\\') ||
    inventory?.disks[0];
  const cards = [
    {
      id: 'memory' as const,
      name: 'Mémoire vive',
      caption: 'RAM',
      icon: MemoryStick,
      color: 'sage',
      value: memory ? formatBytes(memory.total_bytes) : '—',
      detail: memory?.modules.length
        ? t('{{v0}} × {{v2}}', {
            v0: memory.modules.length,
            v2: [...new Set(memory.modules.map((m) => m.kind))].join(' / '),
          })
        : memory
          ? 'Détails des modules non exposés'
          : 'Inventaire en cours',
      used: memory && memory.total_bytes ? 1 - memory.available_bytes / memory.total_bytes : 0,
      foot: memory
        ? t('{{v0}} disponibles', {
            v0: formatBytes(memory.available_bytes),
          })
        : 'Données du système',
    },
    {
      id: 'storage' as const,
      name: 'Stockage',
      caption: 'DISQUES',
      icon: HardDrive,
      color: 'sand',
      value: disk ? formatBytes(disk.total_bytes) : '—',
      detail: disk
        ? `${disk.kind === 'SSD' ? 'SSD' : disk.kind === 'HDD' ? 'Disque dur' : 'Volume'} · ${disk.file_system.toUpperCase()} · ${disk.mount_point}`
        : 'Aucun volume accessible',
      used: disk ? 1 - disk.available_bytes / disk.total_bytes : 0,
      foot: disk
        ? t('{{v0}} libres', {
            v0: formatBytes(disk.available_bytes),
          })
        : 'Volumes montés',
    },
    {
      id: 'cpu' as const,
      name: 'Processeur',
      caption: 'CPU',
      icon: Cpu,
      color: 'lilac',
      value: inventory?.cpu.physical_cores
        ? t('{{v0}} c\u0153urs', {
            v0: inventory.cpu.physical_cores,
          })
        : inventory
          ? `${inventory.cpu.logical_cores} threads`
          : '—',
      detail: inventory?.cpu.name || 'Identification en cours',
      used: null,
      foot: inventory
        ? `${inventory.cpu.logical_cores} threads · ${inventory.arch}`
        : 'Données du système',
    },
  ];
  return (
    <div className="page-enter">
      <SectionTitle
        eyebrow={tx('BIENVENUE DANS COMPUTY')}
        title={tx('Votre matériel, en clair.')}
        description={tx('Comprenez votre ordinateur. Prenez-en soin, au bon moment.')}
        action={
          <button className="button secondary" onClick={() => navigate('diagnostics')}>
            <Plus size={17} />
            {tx('Nouveau diagnostic')}
          </button>
        }
      />
      <div className="overview-hero-grid">
        <section className="hero-card">
          <div className="hero-copy">
            <span className="hero-kicker">
              <span className="sparkle-dot">
                <Sparkles size={12} />
              </span>
              {tx(' UN PEU D\u2019ATTENTION, BEAUCOUP DE S\xC9R\xC9NIT\xC9')}
            </span>
            <h2>
              {tx('Un check-up.')}
              <br />
              {tx('L\u2019esprit plus l\xE9ger')}
              <span>{tx('.')}</span>
            </h2>
            <p>
              {tx('RAM, disques, processeur : faites le point')}
              <br className="desktop-break" />
              {tx(' sur ce qui fait tourner votre quotidien.')}
            </p>
            <button className="button primary" onClick={() => navigate('diagnostics')}>
              <ScanLine size={17} />
              {tx(job?.running ? 'Voir le diagnostic' : 'Lancer un diagnostic')}
              <ArrowUpRight size={17} />
            </button>
            <span className="hero-reassurance">
              <ShieldCheck size={13} />
              {tx('Local. Transparent. Sans abonnement.')}
            </span>
          </div>
          <div className="hero-art" aria-hidden="true">
            <div className="orbit orbit-one" />
            <div className="orbit orbit-two" />
            <div className="orbit orbit-three" />
            <span className="art-dot dot-one" />
            <span className="art-dot dot-two" />
            <div className="hero-core">
              <Logo size={132} />
            </div>
            <span className="floating-chip chip-one">
              <MemoryStick size={16} />
              <span>{tx('RAM')}</span>
              <Check size={12} />
            </span>
            <span className="floating-chip chip-two">
              <HardDrive size={16} />
              <span>{tx('SSD')}</span>
              <Check size={12} />
            </span>
            <span className="floating-chip chip-three">
              <Cpu size={16} />
            </span>
          </div>
        </section>
        <section className="health-card">
          <div className="card-overline">
            <span>{tx('VOTRE DERNIER BILAN')}</span>
            <Activity size={17} />
          </div>
          {tx(
            latest ? (
              <>
                <div className={`health-ring ${latest.summary.status}`}>
                  <div>
                    <Mood status={latest.summary.status} size={32} />
                    <b>
                      {tx(latest.summary.score ?? '—')}
                      <small>{tx('/ 10')}</small>
                    </b>
                  </div>
                </div>
                <h3>
                  {tx(
                    latest.summary.status === 'failed'
                      ? 'Une anomalie à examiner'
                      : latest.summary.status === 'warning'
                        ? 'Un bilan à compléter'
                        : latest.completed
                          ? 'Vos contrôles sont conformes'
                          : 'Un bilan interrompu',
                  )}
                </h3>
                <p>
                  {tx(latest.summary.checked)}
                  {tx('/')}
                  {tx(latest.summary.total)}
                  {tx(' contr\xF4les exploitables \xB7 ')}
                  {tx(formatDate(latest.created_at, true))}
                </p>
                <button className="text-button" onClick={() => openReport(latest)}>
                  {tx('Comprendre mon r\xE9sultat ')}
                  <ArrowRight size={15} />
                </button>
              </>
            ) : (
              <>
                <div className="health-ring empty-ring">
                  <div>
                    <Mood status="unavailable" size={34} />
                    <b>
                      {tx('\u2014')}
                      <small>{tx('/ 10')}</small>
                    </b>
                  </div>
                </div>
                <h3>{tx('Faisons connaissance.')}</h3>
                <p>
                  {tx('Votre premier diagnostic r\xE9v\xE9lera')}
                  <br />
                  {tx('ce que votre mat\xE9riel a \xE0 vous dire.')}
                </p>
                <span className="pending-label">
                  <Clock3 size={13} />
                  {tx(' En attente du premier bilan')}
                </span>
              </>
            ),
          )}
        </section>
      </div>
      <div className="section-heading">
        <h2>
          {tx('\xC0 l\u2019int\xE9rieur de votre PC ')}
          <span className="count-pill">{tx('3 composants')}</span>
        </h2>
        <span className="subtle-label">
          {tx(
            loading ? (
              <>
                <Spinner size={13} />
                {tx(' Lecture du mat\xE9riel')}
              </>
            ) : (
              <>
                <span className="online-dot" />
                {tx(' Donn\xE9es syst\xE8me')}
              </>
            ),
          )}
        </span>
      </div>
      <div className="component-grid">
        {tx(
          cards.map((card) => {
            const status = componentSummary(latest?.checks || [], card.id);
            const Icon = card.icon;
            return (
              <button
                className={`component-card ${card.color}`}
                key={card.id}
                onClick={() => navigate(card.id)}
              >
                <div className="component-top">
                  <span className={`component-icon ${card.color}`}>
                    <Icon size={23} strokeWidth={1.6} />
                  </span>
                  <span className="component-caption">{tx(card.caption)}</span>
                  <ArrowUpRight size={18} className="card-arrow" />
                </div>
                <div className="component-label">{tx(card.name)}</div>
                <h3>{tx(card.value)}</h3>
                <p className="component-detail" title={tx(card.detail)}>
                  {tx(card.detail)}
                </p>
                {tx(
                  card.used !== null ? (
                    <div className="usage-bar">
                      <span
                        style={{
                          width: `${card.used * 100}%`,
                        }}
                      />
                    </div>
                  ) : (
                    <div className="core-bars" aria-hidden="true">
                      {tx(
                        Array.from(
                          {
                            length: 16,
                          },
                          (_, i) => (
                            <span
                              key={i}
                              className={i < (inventory?.cpu.logical_cores || 0) ? 'filled' : ''}
                            />
                          ),
                        ),
                      )}
                    </div>
                  ),
                )}
                <div className="component-bottom">
                  <span>{tx(card.foot)}</span>
                  <StatusBadge status={status.status} compact />
                </div>
              </button>
            );
          }),
        )}
      </div>
      <div className="overview-bottom-grid">
        <section className="panel recent-panel">
          <PanelHeader
            title={tx('Votre historique')}
            subtitle={tx('Les petits changements méritent d’être suivis.')}
            action={
              <button className="text-button" onClick={() => navigate('history')}>
                {tx('Tout voir ')}
                <ArrowRight size={15} />
              </button>
            }
          />
          {tx(
            reports.length ? (
              <div className="recent-list">
                {tx(
                  reports.slice(0, 3).map((item) => (
                    <button key={item.id} className="recent-row" onClick={() => openReport(item)}>
                      <span className="recent-icon">
                        <Activity size={18} />
                      </span>
                      <span className="recent-copy">
                        <b>{item.label}</b>
                        <span>
                          {tx(formatDate(item.created_at, true))} <i>{tx('\xB7')}</i>{' '}
                          {tx(duration(item.duration_ms))}
                        </span>
                      </span>
                      <Score score={item.summary.score} status={item.summary.status} />
                      <ChevronRight size={17} />
                    </button>
                  )),
                )}
              </div>
            ) : (
              <div className="history-blank">
                <div className="history-stack" aria-hidden="true">
                  <span />
                  <span />
                  <span>
                    <History size={20} />
                    <i />
                    <i />
                  </span>
                </div>
                <div>
                  <h3>{tx('Le d\xE9but d\u2019une belle histoire.')}</h3>
                  <p>
                    {tx('Vos diagnostics seront enregistr\xE9s ici.')}
                    <br />
                    {tx('Parfait pour comparer avant et apr\xE8s une mise \xE0 niveau.')}
                  </p>
                  <button className="text-button" onClick={() => navigate('diagnostics')}>
                    {tx('Cr\xE9er mon premier bilan ')}
                    <ArrowRight size={15} />
                  </button>
                </div>
              </div>
            ),
          )}
        </section>
        <section className="tip-card">
          <span className="tip-kicker">
            <Sparkles size={14} />
            {tx(' LE BON R\xC9FLEXE')}
          </span>
          <h3>
            {tx('De nouvelles barrettes ?')}
            <br />
            {tx('Gardez un point de rep\xE8re.')}
          </h3>
          <p>
            {tx(
              'Lancez un diagnostic avant le remplacement, puis un autre apr\xE8s. Computy met vos deux configurations c\xF4te \xE0 c\xF4te.',
            )}
          </p>
          <button className="text-button" onClick={() => navigate('compare')}>
            {tx('D\xE9couvrir la comparaison ')}
            <ArrowUpRight size={16} />
          </button>
          <div className="tip-comparison" aria-hidden="true">
            <span>
              {tx('AVANT')}
              <MemoryStick size={21} />
              <b>
                {tx('16 ')}
                <small>{tx('Gio')}</small>
              </b>
            </span>
            <ArrowRight size={18} />
            <span>
              {tx('APR\xC8S')}
              <MemoryStick size={21} />
              <b>
                {tx('32 ')}
                <small>{tx('Gio')}</small>
              </b>
            </span>
          </div>
        </section>
      </div>
      <div className="transparency-line">
        <ShieldCheck size={15} />
        <p>
          {tx('Des r\xE9sultats utiles, des limites claires. ')}
          <span>{tx('Un diagnostic sans erreur ne garantit pas l\u2019absence de panne.')}</span>
        </p>
        <button onClick={() => navigate('about')}>
          {tx('Notre m\xE9thode ')}
          <ArrowUpRight size={13} />
        </button>
      </div>
    </div>
  );
}
