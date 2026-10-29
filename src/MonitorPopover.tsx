import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ArrowUpRight, CircleHelp, ExternalLink, Power, Settings2, X } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { setLanguage, currentLocale, languages, t, tx } from './i18n.mjs';
import { rpc, defaultPreferences, isDesktop } from './api';
import { Spinner } from './components';
import { MetricVisual, metricIcons, ProcessAvatar, useProcessIcons } from './monitor-components';
import {
  metricNames,
  metricValue,
  processValue,
  rankedProcesses,
  usageColor,
} from './monitor-presentation.mjs';
import { formatBytes } from './domain.mjs';
import type { MonitorSnapshot, Preferences, ProcessSample } from './types';
export default function MonitorPopover() {
  useTranslation();
  useEffect(() => {
    const close = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && isDesktop())
        void invoke('monitor_panel_action', { destination: 'hide' });
    };
    window.addEventListener('keydown', close);
    return () => window.removeEventListener('keydown', close);
  }, []);
  const [preferences, setPreferences] = useState<Preferences>(defaultPreferences);
  const [snapshot, setSnapshot] = useState<MonitorSnapshot | null>(null);
  const [metric, setMetric] = useState('memory');
  const [volume, setVolume] = useState('');
  const [problem, setProblem] = useState(false);
  useEffect(() => {
    document.documentElement.classList.add('popover-document');
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;
    let request = 0;
    const refresh = async () => {
      const sequence = ++request;
      clearTimeout(timer);
      try {
        const target = isDesktop()
          ? await invoke<{ metric: string; volume: string; visible: boolean }>(
              'monitor_panel_target',
            )
          : {
              metric: new URLSearchParams(location.search).get('metric') || 'memory',
              volume: '',
              visible: true,
            };
        const settings = await rpc<Preferences>('preferences');
        if (stopped || sequence !== request) return;
        setPreferences(settings);
        await setLanguage(settings.language);
        if (!target.visible) return;
        setMetric(target.metric);
        setVolume(target.volume);
        const state = await rpc<MonitorSnapshot>('monitor');
        if (stopped || sequence !== request) return;
        setSnapshot(state);
        setProblem(false);
      } catch {
        if (!stopped) setProblem(true);
      } finally {
        if (!stopped && sequence === request) timer = setTimeout(refresh, 2000);
      }
    };
    void refresh();
    const focus = () => void refresh();
    window.addEventListener('focus', focus);
    return () => {
      stopped = true;
      clearTimeout(timer);
      window.removeEventListener('focus', focus);
    };
  }, []);
  useEffect(() => {
    const query = matchMedia('(prefers-color-scheme: dark)');
    const apply = () => {
      document.documentElement.dataset.theme =
        preferences.theme === 'system' ? (query.matches ? 'dark' : 'light') : preferences.theme;
      document.documentElement.lang = currentLocale();
      document.documentElement.dir =
        languages.find((l) => l.code === currentLocale())?.dir || 'ltr';
    };
    apply();
    query.addEventListener('change', apply);
    return () => query.removeEventListener('change', apply);
  }, [preferences.theme, preferences.language, currentLocale()]);
  const value = metricValue(snapshot, metric, volume);
  const Icon = metricIcons[metric as keyof typeof metricIcons] || metricIcons.storage;
  const top = rankedProcesses(snapshot?.processes || [], metric)
    .filter((p) => processValue(p, metric) !== null)
    .slice(0, 10);
  const icons = useProcessIcons(top.map((p) => p.pid));
  const action = (destination: string) => {
    if (isDesktop())
      void invoke('monitor_panel_action', { destination }).catch(() => setProblem(true));
    else if (destination !== 'hide' && destination !== 'quit')
      location.href =
        destination === 'processes'
          ? `/#processes?metric=${metric}`
          : '/#' + (destination === 'overview' ? 'overview' : 'settings');
  };
  const amount = (p: ProcessSample) =>
    metric === 'memory'
      ? formatBytes(p.memory_bytes)
      : metric === 'gpu'
        ? formatBytes(p.gpu_memory_bytes)
        : metric === 'storage'
          ? `${formatBytes(processValue(p, metric))}/s`
          : t('{{value}} %', { value: Number((p.cpu_percent || 0).toFixed(1)) });
  const share = (p: ProcessSample) =>
    metric === 'memory' ? p.memory_percent : metric === 'storage' ? p.io_percent : null;
  return (
    <main className="monitor-popover">
      <header className="popover-toolbar">
        <span>
          <i className="online-dot" />
          {tx('EN DIRECT')}
          <b>Computy</b>
        </span>
        <button className="icon-button" aria-label={tx('Fermer')} onClick={() => action('hide')}>
          <X size={17} />
        </button>
      </header>
      <section className="popover-hero">
        <MetricVisual value={value} metric={metric} />
        <div>
          <span className="popover-device">
            <Icon size={22} />
            {metric === 'memory'
              ? 'RAM'
              : metric === 'storage'
                ? 'SSD / HDD'
                : metric.toUpperCase()}
          </span>
          <h1>{tx(metricNames[metric])}</h1>
          <p>{volume || tx('Mon ordinateur')}</p>
          <small style={{ color: usageColor(value) }}>
            {value === null
              ? tx('Indisponible')
              : t('Utilisation : {{value}} %', { value: Math.round(value) })}
          </small>
        </div>
      </section>
      <section className="popover-processes">
        <div className="popover-section-heading">
          <h2>{tx('Processus les plus consommateurs')}</h2>
          <span>{top.length}</span>
        </div>
        <div className="popover-process-list">
          {!snapshot && !problem ? (
            <div className="popover-empty">
              <Spinner />
            </div>
          ) : top.length ? (
            top.map((p) => (
              <div className="popover-process" key={p.pid}>
                <ProcessAvatar pid={p.pid} icon={icons[p.pid]} />
                <div>
                  <b title={p.name}>{p.name || tx('Nom indisponible')}</b>
                  <small>PID {p.pid}</small>
                </div>
                <span>
                  <b>{amount(p)}</b>
                  <small>
                    {share(p) === null
                      ? metric === 'cpu'
                        ? formatBytes(p.memory_bytes)
                        : ''
                      : t('{{value}} %', { value: Number(share(p)!.toFixed(1)) })}
                  </small>
                </span>
              </div>
            ))
          ) : (
            <p className="popover-empty">
              <CircleHelp size={20} />
              {tx('Aucune mesure par processus disponible.')}
            </p>
          )}
        </div>
        {metric === 'gpu' && (
          <small className="popover-scope">
            {tx('Mémoire GPU des processus de calcul NVIDIA uniquement')}
          </small>
        )}
        {metric === 'storage' && (
          <small className="popover-scope">{tx('Activité par processus : tous les volumes')}</small>
        )}
        <button className="popover-action popover-view-all" onClick={() => action('processes')}>
          <span>
            <ExternalLink size={17} />
            {tx('Voir tout')}
          </span>
          <ArrowUpRight size={16} />
        </button>
      </section>
      {problem && (
        <p className="popover-scope" role="status">
          {tx('Les mesures en direct sont indisponibles. Computy réessaie automatiquement.')}
        </p>
      )}
      <footer className="popover-actions">
        <button className="popover-action" onClick={() => action('overview')}>
          <span>
            <ExternalLink size={17} />
            {tx('Ouvrir Computy')}
          </span>
          <ArrowUpRight size={15} />
        </button>
        <button className="popover-action" onClick={() => action('settings')}>
          <span>
            <Settings2 size={17} />
            {tx('Préférences')}
          </span>
          <ArrowUpRight size={15} />
        </button>
        <button className="popover-action popover-quit" onClick={() => action('quit')}>
          <span>
            <Power size={17} />
            {tx('Quitter Computy')}
          </span>
        </button>
      </footer>
    </main>
  );
}
