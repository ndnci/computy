import { useEffect, useState } from 'react';
import { Activity, Cpu, HardDrive, MemoryStick, Monitor } from 'lucide-react';
import { t, tx } from '../i18n.mjs';
import { isDesktop, rpc } from '../api';
import { useApp } from '../app-context';
import { Notice, Panel, PanelHeader, Spinner } from '../components';
import { MetricVisual } from '../monitor-components';
import type { MonitorSnapshot, Preferences } from '../types';

export default function MonitorSettings({
  draft,
  onChange,
}: {
  draft: Preferences;
  onChange: (draft: Preferences) => void;
}) {
  const { navigate } = useApp();
  const [snapshot, setSnapshot] = useState<MonitorSnapshot | null>(null);
  const [problem, setProblem] = useState(false);
  useEffect(() => {
    if (!draft.tray_enabled) return;
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try {
        const value = await rpc<MonitorSnapshot>('monitor');
        if (!stopped) {
          setSnapshot(value);
          setProblem(false);
        }
      } catch {
        if (!stopped) {
          setProblem(true);
          setSnapshot(null);
        }
      } finally {
        if (!stopped) timer = setTimeout(poll, 2000);
      }
    };
    void poll();
    return () => {
      stopped = true;
      clearTimeout(timer);
    };
  }, [draft.tray_enabled]);
  const toggle = (key: 'tray_components' | 'tray_disks', id: string) =>
    onChange({
      ...draft,
      [key]: draft[key].includes(id)
        ? draft[key].filter((value) => value !== id)
        : [...draft[key], id],
    });
  const metrics = [
    {
      id: 'memory',
      label: 'RAM',
      name: 'Mémoire vive',
      icon: MemoryStick,
      value: snapshot?.memory_percent ?? null,
    },
    {
      id: 'cpu',
      label: 'CPU',
      name: 'Processeur',
      icon: Cpu,
      value: snapshot?.cpu_percent ?? null,
    },
    {
      id: 'gpu',
      label: 'GPU',
      name: 'Processeur graphique',
      icon: Monitor,
      value: snapshot?.gpu_percent ?? null,
    },
  ];
  const disks = [
    ...(snapshot?.disks || []),
    ...draft.tray_disks
      .filter((id) => !snapshot?.disks.some((d) => d.id === id))
      .map((id) => ({
        id,
        name: '',
        used_percent: null,
        read_bytes_per_second: null,
        written_bytes_per_second: null,
      })),
  ];
  return (
    <Panel className="monitor-settings">
      <PanelHeader
        title={tx('Moniteur dans la barre système')}
        subtitle={tx('En direct dans votre barre système.')}
      />
      <div className="setting-row">
        <span className="setting-icon">
          <Activity size={22} />
        </span>
        <div>
          <b>{tx('Afficher les indicateurs en temps réel')}</b>
          <p>
            {tx(
              'Activé par défaut : RAM, CPU et GPU. Les changements s’appliquent après l’enregistrement des préférences.',
            )}
          </p>
        </div>
        <button
          role="switch"
          aria-label={tx('Afficher les indicateurs en temps réel')}
          aria-checked={draft.tray_enabled}
          className={`toggle ${draft.tray_enabled ? 'on' : ''}`}
          onClick={() => onChange({ ...draft, tray_enabled: !draft.tray_enabled })}
        >
          <span />
        </button>
      </div>
      {draft.tray_enabled && (
        <div className="monitor-content">
          <fieldset className="tray-style-options">
            <legend>{tx('Style de la barre système')}</legend>
            {(
              [
                ['icon_text', 'Icône + pourcentage'],
                ['ring', 'Anneau'],
                ['fill', 'Icône remplie'],
              ] as const
            ).map(([id, label]) => (
              <label
                className={`tray-style-option ${draft.tray_style === id ? 'selected' : ''}`}
                key={id}
              >
                <input
                  type="radio"
                  name="tray-style"
                  value={id}
                  checked={draft.tray_style === id}
                  onChange={() => onChange({ ...draft, tray_style: id })}
                />
                <MetricVisual value={42} metric="memory" style={id} />
                <b>{tx(label)}</b>
                {id === 'icon_text' && <small>{tx('Par défaut')}</small>}
              </label>
            ))}
          </fieldset>
          <p className="monitor-note tray-colors">
            <i style={{ background: '#429feb' }} /> {tx('Bleu')} &lt; 65 %{' '}
            <i style={{ background: '#dca842' }} /> {tx('Jaune')} 65–89 %{' '}
            <i style={{ background: '#e96565' }} /> {tx('Rouge')} ≥ 90 %
          </p>
          {!snapshot && !problem && (
            <p role="status">
              <Spinner size={16} /> {tx('Lecture des mesures en direct…')}
            </p>
          )}
          {problem && (
            <Notice tone="warn">
              {tx('Les mesures en direct sont indisponibles. Computy réessaie automatiquement.')}
            </Notice>
          )}
          <div className="monitor-metrics">
            {metrics.map(({ id, label, name, icon: Icon, value }) => (
              <label
                className={`monitor-option ${draft.tray_components.includes(id) ? 'selected' : ''}`}
                key={id}
              >
                <input
                  type="checkbox"
                  checked={draft.tray_components.includes(id)}
                  onChange={() => toggle('tray_components', id)}
                  aria-label={t('Indicateur : {{name}}', { name: tx(name) })}
                />
                <MetricVisual value={value} metric={id} style={draft.tray_style} />
                <b>
                  <Icon size={15} /> {tx(name)}
                </b>
                <small>
                  {value === null
                    ? tx('Indisponible')
                    : t('Utilisation : {{value}} %', { value: Math.round(value) })}
                </small>
              </label>
            ))}
          </div>
          <p className="monitor-note">
            {tx(
              'CPU et RAM : actualisation toutes les 2 secondes. GPU : jusqu’à 6 secondes selon le pilote. « — » signifie qu’aucune mesure n’est disponible, pas 0 %.',
            )}
          </p>
          <button className="button secondary small" onClick={() => navigate('processes')}>
            {tx('Voir tous les processus')}
          </button>
          <p className="monitor-note">
            {tx(
              'Le GPU dépend des compteurs exposés par votre pilote : AMD/NVIDIA sous Linux, compteurs Windows et certains pilotes IOAccelerator sous macOS. Les GPU non pris en charge restent indiqués comme indisponibles.',
            )}
          </p>
          <h3 className="monitor-disks-heading">
            <HardDrive size={17} />
            {tx('Disques à afficher')}
          </h3>
          <p className="monitor-note">
            {tx(
              'Un indicateur par volume sélectionné : l’anneau montre l’espace occupé ; le menu affiche aussi les débits de lecture et d’écriture. Ce ne sont pas des notes de santé.',
            )}
          </p>
          <div className="monitor-disk-options">
            {disks.map((disk) => (
              <label className="monitor-disk-option" key={disk.id}>
                <input
                  type="checkbox"
                  aria-label={t('Afficher le volume : {{volume}}', { volume: disk.id })}
                  checked={draft.tray_disks.includes(disk.id)}
                  disabled={!draft.tray_disks.includes(disk.id) && draft.tray_disks.length >= 16}
                  onChange={() => toggle('tray_disks', disk.id)}
                />
                <HardDrive size={18} />
                <span>
                  <b>{disk.id}</b>
                  <small>{disk.name}</small>
                </span>
                <span>
                  {disk.used_percent === null
                    ? tx('Indisponible')
                    : t('Espace occupé : {{value}} %', { value: Math.round(disk.used_percent) })}
                </span>
              </label>
            ))}
          </div>
          {snapshot && !disks.length && <p>{tx('Aucun volume accessible.')}</p>}
          <Notice>
            {tx(
              isDesktop()
                ? 'Fermer la fenêtre conserve les indicateurs en arrière-plan. Le menu de chaque indicateur permet de rouvrir Computy, d’accéder aux préférences ou de quitter complètement.'
                : 'L’aperçu utilise les mesures du moteur Rust. Les icônes de la barre système sont disponibles dans l’application Tauri installée.',
            )}
          </Notice>
          <p className="monitor-note">
            {tx(
              'Linux : le bureau doit prendre en charge les indicateurs AppIndicator/StatusNotifier. Sous GNOME, une extension de prise en charge peut être nécessaire. Windows peut ranger les icônes dans le menu des icônes masquées.',
            )}
          </p>
        </div>
      )}
    </Panel>
  );
}
