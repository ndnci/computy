import { useEffect, useRef, useState, type CSSProperties } from 'react';
import { AppWindow, Cpu, HardDrive, MemoryStick, Monitor } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { isDesktop } from './api';
import { t, tx } from './i18n.mjs';
import { usageColor } from './monitor-presentation.mjs';
export const metricIcons = { memory: MemoryStick, cpu: Cpu, gpu: Monitor, storage: HardDrive };
export function MetricVisual({
  value,
  metric = 'memory',
  style = 'ring',
}: {
  value: number | null;
  metric?: string;
  style?: 'icon_text' | 'ring' | 'fill';
}) {
  const Icon = metricIcons[metric as keyof typeof metricIcons] || HardDrive;
  const color = usageColor(value);
  const text = value === null ? '—' : t('{{value}} %', { value: Math.round(value) });
  return (
    <span
      className={`metric-visual metric-visual-${style}`}
      style={{ '--usage-color': color, '--usage-fill': `${value ?? 0}%` } as CSSProperties}
      aria-label={value === null ? tx('Indisponible') : text}
    >
      {style === 'ring' ? (
        <svg viewBox="0 0 100 100" aria-hidden="true">
          <circle className="metric-ring-track" cx="50" cy="50" r="43" />
          {value !== null && (
            <circle
              className="metric-ring-value"
              cx="50"
              cy="50"
              r="43"
              pathLength="100"
              strokeDasharray={`${value} 100`}
              transform="rotate(-90 50 50)"
            />
          )}
          <text x="50" y="55">
            {text}
          </text>
        </svg>
      ) : style === 'fill' ? (
        <span className="metric-fill">
          <Icon className="metric-fill-base" strokeWidth={1.8} />
          <span className="metric-fill-liquid">
            <Icon strokeWidth={1.8} />
          </span>
        </span>
      ) : (
        <>
          <Icon size={25} strokeWidth={1.8} />
          <b>{text}</b>
        </>
      )}
    </span>
  );
}
const icons = new Map<number, string | null>();
export function ProcessAvatar({ pid, icon }: { pid: number; icon?: string | null }) {
  const ref = useRef<HTMLSpanElement>(null);
  const [loaded, setLoaded] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    if (icon !== undefined || !isDesktop() || !ref.current) return;
    let stopped = false;
    const observer = new IntersectionObserver((entries) => {
      if (!entries.some((entry) => entry.isIntersecting)) return;
      observer.disconnect();
      if (icons.has(pid)) {
        setLoaded(icons.get(pid) ?? null);
        return;
      }
      void invoke<Record<string, string | null>>('process_icons', { pids: [pid] })
        .then((result) => {
          icons.set(pid, result[pid] ?? null);
          if (!stopped) setLoaded(result[pid] ?? null);
        })
        .catch(() => {});
    });
    observer.observe(ref.current);
    return () => {
      stopped = true;
      observer.disconnect();
    };
  }, [pid, icon]);
  const src = icon ?? loaded;
  return (
    <span className="process-avatar" ref={ref}>
      {src && !failed ? (
        <img src={src} alt="" onError={() => setFailed(true)} />
      ) : (
        <AppWindow size={17} />
      )}
    </span>
  );
}
export function useProcessIcons(pids: number[]) {
  const [values, setValues] = useState<Record<number, string | null>>({});
  const key = pids.slice(0, 32).join(',');
  useEffect(() => {
    if (!isDesktop()) return;
    let stopped = false;
    const selected = key.split(',').filter(Boolean).map(Number);
    const missing = selected.filter((pid) => !icons.has(pid));
    const update = () =>
      setValues(Object.fromEntries(selected.map((pid) => [pid, icons.get(pid) ?? null])));
    update();
    if (missing.length)
      void invoke<Record<string, string | null>>('process_icons', { pids: missing })
        .then((result) => {
          if (icons.size > 512) icons.clear();
          for (const pid of missing) icons.set(pid, result[pid] ?? null);
          if (!stopped) update();
        })
        .catch(() => {});
    return () => {
      stopped = true;
    };
  }, [key]);
  return values;
}
