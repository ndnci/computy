import { useEffect, useState } from 'react';
import { Search } from 'lucide-react';
import { t, tx } from '../i18n.mjs';
import { rpc } from '../api';
import { formatBytes } from '../domain.mjs';
import { Notice, SectionTitle, Spinner } from '../components';
import type { MonitorSnapshot, ProcessSample } from '../types';
import { ProcessAvatar, useProcessIcons } from '../monitor-components';
type Metric = 'memory' | 'cpu' | 'gpu' | 'storage';
const metrics: Metric[] = ['memory', 'cpu', 'gpu', 'storage'];
function initialMetric(): Metric {
  const requested = new URLSearchParams(window.location.hash.split('?')[1]).get('metric') as Metric;
  return metrics.includes(requested) ? requested : 'memory';
}
export default function Processes() {
  const [snapshot, setSnapshot] = useState<MonitorSnapshot | null>(null);
  const [problem, setProblem] = useState('');
  const [query, setQuery] = useState('');
  const [metric, setMetric] = useState<Metric>(initialMetric);
  useEffect(() => {
    const change = () => setMetric(initialMetric());
    window.addEventListener('hashchange', change);
    return () => window.removeEventListener('hashchange', change);
  }, []);
  useEffect(() => {
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try {
        const value = await rpc<MonitorSnapshot>('monitor');
        if (!stopped) {
          setSnapshot(value);
          setProblem('');
        }
      } catch (error) {
        if (!stopped) {
          setProblem((error as Error).message);
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
  }, []);
  const value = (p: ProcessSample) =>
    metric === 'memory'
      ? p.memory_bytes
      : metric === 'cpu'
        ? p.cpu_percent
        : metric === 'gpu'
          ? p.gpu_memory_bytes
          : p.read_bytes_per_second === null || p.written_bytes_per_second === null
            ? null
            : p.read_bytes_per_second + p.written_bytes_per_second;
  const processes = (snapshot?.processes || [])
    .filter((p) => `${p.name} ${p.pid}`.toLowerCase().includes(query.toLowerCase()))
    .sort((a, b) => (value(b) ?? -1) - (value(a) ?? -1) || a.pid - b.pid);
  const icons = useProcessIcons(processes.slice(0, 32).map((p) => p.pid));
  const percent = (n: number | null) =>
    n === null ? '—' : t('{{value}} %', { value: Number(n.toFixed(1)) });
  const bytes = (n: number | null) => (n === null ? '—' : formatBytes(n));
  return (
    <div className="page-enter processes-page">
      <SectionTitle
        eyebrow={tx('EN DIRECT')}
        title={tx('Ce qui utilise votre ordinateur.')}
        description={tx(
          'Tous les processus accessibles, sans limite de nombre. Recherchez un nom ou un PID pour les retrouver.',
        )}
      />
      <Notice>
        {tx(
          'CPU : part de la capacité totale du processeur. RAM : mémoire résidente et part de la RAM physique ; les pages partagées peuvent être comptées plusieurs fois. Les processus protégés peuvent masquer leurs mesures.',
        )}
      </Notice>
      <Notice>
        {tx(
          'Disques : les débits par processus concernent tous les volumes ; le pourcentage est la part des entrées/sorties observables, pas le taux d’occupation d’un disque. GPU : la mémoire des processus de calcul NVIDIA est affichée quand le pilote la fournit ; cette liste ne couvre pas tous les processus graphiques et ne mesure pas leur charge GPU.',
        )}
      </Notice>
      {problem && <Notice tone="warn">{problem}</Notice>}
      {!snapshot && !problem && (
        <p role="status">
          <Spinner /> {tx('Lecture des processus…')}
        </p>
      )}
      <section className="panel history-panel">
        <div className="history-toolbar">
          <div className="search-input">
            <Search size={17} />
            <input
              aria-label={tx('Rechercher un processus')}
              placeholder={tx('Nom du processus ou PID…')}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
          </div>
          <select
            aria-label={tx('Trier les processus par')}
            value={metric}
            onChange={(event) => setMetric(event.target.value as Metric)}
          >
            <option value="memory">{tx('Mémoire vive')}</option>
            <option value="cpu">CPU</option>
            <option value="gpu">{tx('Mémoire GPU')}</option>
            <option value="storage">{tx('Activité disque')}</option>
          </select>
          <span>{t('Processus affichés : {{count}}', { count: processes.length })}</span>
        </div>
        <div className="table-scroll">
          <table className="history-table process-table">
            <thead>
              <tr>
                <th>PID</th>
                <th>{tx('Processus')}</th>
                <th>RAM</th>
                <th>RAM %</th>
                <th>CPU %</th>
                <th>{tx('Lecture / s')}</th>
                <th>{tx('Écriture / s')}</th>
                <th>{tx('Part E/S')}</th>
                <th>{tx('Mémoire GPU')}</th>
              </tr>
            </thead>
            <tbody>
              {processes.map((p) => (
                <tr key={p.pid}>
                  <td>{p.pid}</td>
                  <td className="process-name">
                    <span className="process-identity">
                      <ProcessAvatar pid={p.pid} icon={icons[p.pid]} />
                      {p.name || tx('Nom indisponible')}
                    </span>
                  </td>
                  <td>{bytes(p.memory_bytes)}</td>
                  <td>{percent(p.memory_percent)}</td>
                  <td>{percent(p.cpu_percent)}</td>
                  <td>{bytes(p.read_bytes_per_second)}</td>
                  <td>{bytes(p.written_bytes_per_second)}</td>
                  <td>{percent(p.io_percent)}</td>
                  <td>{bytes(p.gpu_memory_bytes)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        {snapshot && !processes.length && (
          <p className="monitor-content">{tx('Aucun processus ne correspond à la recherche.')}</p>
        )}
      </section>
    </div>
  );
}
