import { t, tx, currentLocale } from '../i18n.mjs';
import {
  Activity,
  ArrowRight,
  BookOpen,
  Check,
  CircleHelp,
  Cpu,
  HardDrive,
  Layers,
  MemoryStick,
  Play,
  RefreshCw,
  Thermometer,
  Wrench,
} from 'lucide-react';
import { useApp } from '../app-context';
import { CheckCard, Empty, Notice, PanelHeader, Score, SectionTitle, Spinner } from '../components';
import { componentSummary, formatBytes, formatDate } from '../domain.mjs';
export default function Hardware({ component }: { component: 'memory' | 'storage' | 'cpu' }) {
  const { inventory, loading, refresh, refreshing, reports, navigate } = useApp();
  const config = {
    memory: {
      name: 'Mémoire vive',
      description: 'Vos barrettes, leur capacité et les possibilités d’évolution.',
      icon: MemoryStick,
    },
    storage: {
      name: 'Stockage',
      description: 'Vos volumes, l’espace disponible et les indicateurs des disques.',
      icon: HardDrive,
    },
    cpu: {
      name: 'Processeur',
      description: 'Le cœur de vos calculs, ses caractéristiques et ses capteurs.',
      icon: Cpu,
    },
  }[component];
  const latest = reports.find((r) => r.checks.some((c) => c.component === component));
  const summary = componentSummary(latest?.checks || [], component);
  return (
    <div className="page-enter">
      <SectionTitle
        eyebrow={tx('VOTRE MATÉRIEL')}
        title={tx(config.name)}
        description={tx(config.description)}
        action={
          <button className="button primary" onClick={() => navigate('diagnostics')}>
            <Activity size={17} />
            {tx('Tester ce composant')}
          </button>
        }
      />
      {tx(
        !inventory ? (
          <div className="panel">
            <Empty
              icon={config.icon}
              title={tx(
                loading ? 'Identification du matériel…' : 'Le matériel n’est pas accessible',
              )}
            >
              {tx(
                loading ? (
                  <Spinner />
                ) : (
                  'Connectez le moteur Rust pour lire les informations réelles de cet ordinateur.'
                ),
              )}
            </Empty>
          </div>
        ) : (
          <>
            <div className="machine-banner">
              <span className="machine-icon">
                <config.icon size={23} />
              </span>
              <div>
                <b>{tx(inventory.model || inventory.host_name)}</b>
                <span>
                  {tx(inventory.os)} {tx(inventory.os_version)}
                  {tx(' \xB7 ')}
                  {tx(inventory.arch)}
                </span>
              </div>
              <button
                className="button secondary small"
                disabled={refreshing}
                onClick={() => void refresh()}
              >
                <RefreshCw size={14} className={refreshing ? 'spin' : ''} />
                {tx('Actualiser')}
              </button>
            </div>
            {component === 'memory' ? (
              <>
                <div className="stat-grid four">
                  <Stat
                    label={tx('Mémoire visible par l’OS')}
                    value={formatBytes(inventory.memory.total_bytes)}
                    foot="La mémoire réservée au matériel est exclue."
                  />
                  <Stat
                    label={tx('Actuellement disponible')}
                    value={formatBytes(inventory.memory.available_bytes)}
                    foot="Varie selon les applications ouvertes."
                  />
                  <Stat
                    label={tx('Capacité maximale déclarée')}
                    value={
                      inventory.memory.max_capacity_bytes
                        ? formatBytes(inventory.memory.max_capacity_bytes)
                        : 'Non communiquée'
                    }
                    foot="Déclaration SMBIOS, à confirmer par le fabricant."
                  />
                  <Stat
                    label={tx('Emplacements déclarés')}
                    value={inventory.memory.slots?.toString() || 'Non communiqués'}
                    foot={t('{{v0}} module(s) identifi\xE9(s) par le firmware.', {
                      v0: inventory.memory.modules.length || 'Aucun',
                    })}
                  />
                </div>
                <section className="panel">
                  <PanelHeader
                    title={tx('Vos modules de mémoire')}
                    subtitle={tx(`Source : ${inventory.memory.source || 'système d’exploitation'}`)}
                  />
                  {tx(
                    inventory.memory.modules.length ? (
                      <div className="module-grid">
                        {tx(
                          inventory.memory.modules.map((module, index) => (
                            <article key={index} className="module-card">
                              <div className="module-top">
                                <div className="ram-illustration" aria-hidden="true">
                                  <i />
                                  <i />
                                  <i />
                                  <i />
                                  <span />
                                </div>
                                <span className="badge neutral">
                                  {tx(module.locator || `Module ${index + 1}`)}
                                </span>
                              </div>
                              <h3>
                                {tx(formatBytes(module.capacity_bytes))}{' '}
                                <span>{tx(module.kind)}</span>
                              </h3>
                              <p>{tx(module.manufacturer || 'Fabricant non communiqué')}</p>
                              <dl className="spec-list">
                                <div>
                                  <dt>{tx('R\xE9f\xE9rence')}</dt>
                                  <dd>{tx(module.part_number || 'Non communiquée')}</dd>
                                </div>
                                <div>
                                  <dt>{tx('Format')}</dt>
                                  <dd>{tx(module.form_factor || 'Non communiqué')}</dd>
                                </div>
                                <div>
                                  <dt>{tx('D\xE9bit nominal de la barrette')}</dt>
                                  <dd>
                                    {tx(
                                      module.speed_mts
                                        ? `${module.speed_mts.toLocaleString(currentLocale())} MT/s`
                                        : 'Non communiqué',
                                    )}
                                  </dd>
                                </div>
                                <div>
                                  <dt>{tx('D\xE9bit configur\xE9')}</dt>
                                  <dd>
                                    {tx(
                                      module.configured_speed_mts
                                        ? `${module.configured_speed_mts.toLocaleString(currentLocale())} MT/s`
                                        : 'Non communiqué',
                                    )}
                                  </dd>
                                </div>
                                <div>
                                  <dt>{tx('Remplacement')}</dt>
                                  <dd>
                                    {tx(
                                      module.replaceable === true ? (
                                        <span className="inline-good">
                                          <Wrench size={13} />
                                          {tx('Format g\xE9n\xE9ralement rempla\xE7able')}
                                        </span>
                                      ) : module.replaceable === false ? (
                                        'Composant intégré / soudé'
                                      ) : (
                                        'À confirmer avec le fabricant'
                                      ),
                                    )}
                                  </dd>
                                </div>
                              </dl>
                            </article>
                          )),
                        )}
                      </div>
                    ) : (
                      <Empty
                        icon={MemoryStick}
                        title={tx('Le système garde ces détails pour lui.')}
                      >
                        {tx(
                          'L\u2019OS expose la quantit\xE9 de RAM, mais les informations sur les barrettes n\xE9cessitent un acc\xE8s firmware. Elles ne sont pas d\xE9duites ni invent\xE9es.',
                        )}
                      </Empty>
                    ),
                  )}
                </section>
                <div className="hardware-two-col">
                  <section className="panel">
                    <PanelHeader
                      title={tx('Une mise à niveau en tête ?')}
                      subtitle={tx('Vérifiez les trois points essentiels.')}
                    />
                    <dl className="spec-list padded">
                      <div>
                        <dt>{tx('Capacit\xE9 maximale de la plateforme')}</dt>
                        <dd>
                          {tx(
                            inventory.memory.max_capacity_bytes
                              ? formatBytes(inventory.memory.max_capacity_bytes)
                              : 'À consulter dans le manuel',
                          )}
                        </dd>
                      </div>
                      <div>
                        <dt>{tx('D\xE9bit maximal de la plateforme')}</dt>
                        <dd>{tx('Non communiqu\xE9 par ces interfaces')}</dd>
                      </div>
                      <div>
                        <dt>{tx('Possibilit\xE9 de remplacement')}</dt>
                        <dd>
                          {tx(
                            inventory.os.toLowerCase().includes('mac') &&
                              inventory.arch === 'aarch64'
                              ? 'Apple Silicon : mémoire intégrée'
                              : 'À confirmer dans le manuel du modèle',
                          )}
                        </dd>
                      </div>
                    </dl>
                    <div className="panel-notice">
                      <BookOpen size={17} />
                      <p>
                        {tx(
                          'La carte m\xE8re, le processeur et le firmware fixent ensemble la compatibilit\xE9. La vitesse d\u2019une barrette ne suffit pas \xE0 la d\xE9terminer.',
                        )}
                      </p>
                    </div>
                  </section>
                  <section className="panel notes-panel">
                    <PanelHeader title={tx('Ce que ces données signifient')} />
                    {tx(
                      inventory.memory.notes.map((note) => (
                        <p key={note}>
                          <CircleHelp size={15} />
                          {tx(note)}
                        </p>
                      )),
                    )}
                  </section>
                </div>
              </>
            ) : component === 'storage' ? (
              <>
                <Notice tone={inventory.smart_available ? 'info' : 'warn'}>
                  {tx(
                    inventory.smart_available ? (
                      <>
                        <b>{tx('smartctl est disponible.')}</b>
                        {tx(
                          ' Lancez un diagnostic pour lire la t\xE9l\xE9m\xE9trie des disques physiques accessibles.',
                        )}
                      </>
                    ) : (
                      <>
                        <b>
                          {tx('La t\xE9l\xE9m\xE9trie SMART n\u2019est pas encore accessible.')}
                        </b>
                        {tx(
                          ' Installez smartmontools sur votre syst\xE8me puis actualisez l\u2019inventaire. Selon l\u2019OS, des droits administrateur peuvent \xEAtre n\xE9cessaires.',
                        )}
                      </>
                    ),
                  )}
                </Notice>
                <section className="panel">
                  <PanelHeader
                    title={tx('Volumes montés')}
                    subtitle={tx('Un même disque physique peut contenir plusieurs volumes.')}
                  />
                  {tx(
                    inventory.disks.length ? (
                      <div className="disk-grid">
                        {tx(
                          inventory.disks.map((disk, index) => (
                            <article key={index} className="volume-card">
                              <span className="component-icon sand">
                                <HardDrive size={25} />
                              </span>
                              <div>
                                <div className="volume-title">
                                  <h3>{tx(disk.mount_point)}</h3>
                                  <span className="badge neutral">
                                    {tx(
                                      disk.kind === 'SSD'
                                        ? 'SSD'
                                        : disk.kind === 'HDD'
                                          ? 'HDD'
                                          : 'Type non exposé',
                                    )}
                                  </span>
                                </div>
                                <p className="truncate" title={tx(disk.name)}>
                                  {tx(disk.name)}
                                </p>
                                <b className="volume-size">{tx(formatBytes(disk.total_bytes))}</b>
                                <div className="usage-bar">
                                  <span
                                    style={{
                                      width: `${(1 - disk.available_bytes / disk.total_bytes) * 100}%`,
                                    }}
                                  />
                                </div>
                                <div className="volume-space">
                                  <span>
                                    {tx(formatBytes(disk.available_bytes))}
                                    {tx(' libres')}
                                  </span>
                                  <span>
                                    {tx(disk.file_system.toUpperCase())}
                                    {tx(disk.removable ? ' · Amovible' : '')}
                                  </span>
                                </div>
                              </div>
                            </article>
                          )),
                        )}
                      </div>
                    ) : (
                      <Empty icon={HardDrive} title={tx('Aucun volume exposé')}>
                        {tx(
                          'Le syst\xE8me ou le conteneur ne fournit pas d\u2019inventaire des volumes.',
                        )}
                      </Empty>
                    ),
                  )}
                </section>
                <div className="hardware-two-col">
                  <section className="panel">
                    <PanelHeader title={tx('Deux contrôles complémentaires')} />
                    <div className="explanation-steps">
                      <div>
                        <span>{tx('01')}</span>
                        <div>
                          <h3>{tx('Les indicateurs SMART / NVMe')}</h3>
                          <p>
                            {tx(
                              '\xC9tat d\xE9clar\xE9, erreurs de m\xE9dia, secteurs en attente, usure et journaux disponibles. Chaque disque physique accessible re\xE7oit son propre r\xE9sultat.',
                            )}
                          </p>
                        </div>
                      </div>
                      <div>
                        <span>{tx('02')}</span>
                        <div>
                          <h3>{tx('L\u2019int\xE9grit\xE9 d\u2019un fichier temporaire')}</h3>
                          <p>
                            {tx(
                              '\xC9criture de motifs, synchronisation sur le volume des donn\xE9es de Computy, puis relecture et comparaison de chaque octet. Le fichier est ensuite supprim\xE9.',
                            )}
                          </p>
                        </div>
                      </div>
                    </div>
                  </section>
                  <section className="panel notes-panel">
                    <PanelHeader title={tx('Comprendre la portée')} />
                    <p>
                      <CircleHelp size={15} />
                      {tx(
                        'Un contr\xF4le de fichier ne couvre pas tous les secteurs du disque. Une alerte SMART doit \xEAtre examin\xE9e m\xEAme si le fichier a \xE9t\xE9 relu sans erreur.',
                      )}
                    </p>
                    <p>
                      <CircleHelp size={15} />
                      {tx(
                        'Les caches, ponts USB et contr\xF4leurs RAID peuvent limiter ce qui est visible. Un \xE9tat inaccessible reste \xAB non v\xE9rifi\xE9 \xBB.',
                      )}
                    </p>
                    <p>
                      <CircleHelp size={15} />
                      {tx(
                        'Aucun formatage, aucune \xE9criture directe sur le p\xE9riph\xE9rique et aucune modification de vos fichiers.',
                      )}
                    </p>
                  </section>
                </div>
              </>
            ) : (
              <>
                <section className="processor-hero panel">
                  <span className="processor-visual" aria-hidden="true">
                    <Cpu size={68} strokeWidth={1} />
                  </span>
                  <div>
                    <p className="eyebrow">{tx(inventory.cpu.vendor || 'PROCESSEUR DÉTECTÉ')}</p>
                    <h2>{tx(inventory.cpu.name || 'Modèle non communiqué')}</h2>
                    <p>
                      {tx(inventory.arch)}
                      {tx(' \xB7 ')}
                      {tx(inventory.cpu.physical_cores || '—')}
                      {tx(' c\u0153urs physiques \xB7 ')}
                      {tx(inventory.cpu.logical_cores)}
                      {tx(' threads')}
                    </p>
                  </div>
                </section>
                <div className="stat-grid">
                  <Stat
                    label={tx('Cœurs physiques')}
                    value={inventory.cpu.physical_cores?.toString() || 'Non communiqué'}
                    foot="Valeur exposée par le système."
                  />
                  <Stat
                    label={tx('Processeurs logiques')}
                    value={String(inventory.cpu.logical_cores)}
                    foot="Threads visibles par l’OS."
                  />
                  <Stat
                    label={tx('Fréquence déclarée')}
                    value={
                      inventory.cpu.frequency_mhz
                        ? `${(inventory.cpu.frequency_mhz / 1000).toLocaleString(currentLocale(), {
                            maximumFractionDigits: 2,
                          })} GHz`
                        : 'Non communiquée'
                    }
                    foot="Mesure système ; ce n’est pas la fréquence maximale garantie."
                  />
                </div>
                <section className="panel">
                  <PanelHeader
                    title={tx('Capteurs de température')}
                    subtitle={tx(
                      'Relevé ponctuel au moment de l’actualisation, tous capteurs système confondus.',
                    )}
                  />
                  {tx(
                    inventory.temperatures.filter((sensor) => sensor.celsius !== null).length ? (
                      <div className="sensor-grid">
                        {tx(
                          inventory.temperatures
                            .filter((sensor) => sensor.celsius !== null)
                            .map((sensor, index) => (
                              <div className="sensor" key={index}>
                                <Thermometer size={22} />
                                <div>
                                  <span>{tx(sensor.name)}</span>
                                  <b>
                                    {tx(
                                      sensor.celsius?.toLocaleString('fr-FR', {
                                        maximumFractionDigits: 1,
                                      }),
                                    )}
                                    {tx(' \xB0C')}
                                  </b>
                                  {tx(
                                    sensor.critical_celsius && (
                                      <small>
                                        {tx('Seuil critique d\xE9clar\xE9 : ')}
                                        {tx(sensor.critical_celsius)}
                                        {tx(' \xB0C')}
                                      </small>
                                    ),
                                  )}
                                </div>
                              </div>
                            )),
                        )}
                      </div>
                    ) : (
                      <Empty
                        icon={Thermometer}
                        title={tx('Les températures ne sont pas exposées.')}
                      >
                        {tx(
                          'L\u2019acc\xE8s d\xE9pend de l\u2019OS, des pilotes et du mat\xE9riel. Aucune temp\xE9rature ne sera estim\xE9e.',
                        )}
                      </Empty>
                    ),
                  )}
                </section>
                <Notice>
                  {tx(
                    'Le test CPU compare des calculs parall\xE8les \xE0 des r\xE9ponses connues. Il ne certifie ni la stabilit\xE9 thermique prolong\xE9e, ni toutes les instructions du processeur.',
                  )}
                </Notice>
              </>
            )}
          </>
        ),
      )}
      <section className="panel hardware-results">
        <PanelHeader
          title={tx('Derniers contrôles enregistrés')}
          subtitle={tx(
            latest
              ? `${latest.label} · ${formatDate(latest.created_at)}`
              : 'Les résultats apparaîtront après votre premier diagnostic.',
          )}
          action={latest ? <Score score={summary.score} status={summary.status} /> : null}
        />
        {tx(
          latest ? (
            <div className="checks-stack">
              {tx(
                latest.checks
                  .filter((check) => check.component === component)
                  .map((check) => <CheckCard key={check.id} check={check} />),
              )}
            </div>
          ) : (
            <div className="no-results-inline">
              <Activity size={19} />
              <span>{tx('Ce composant n\u2019a pas encore \xE9t\xE9 test\xE9.')}</span>
              <button className="text-button" onClick={() => navigate('diagnostics')}>
                {tx('Configurer un diagnostic ')}
                <ArrowRight size={15} />
              </button>
            </div>
          ),
        )}
      </section>
    </div>
  );
}
function Stat({ label, value, foot }: { label: string; value: string; foot: string }) {
  return (
    <article className="stat-card">
      <span>{tx(label)}</span>
      <strong>{tx(value)}</strong>
      <p>{tx(foot)}</p>
    </article>
  );
}
