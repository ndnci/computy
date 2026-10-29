import type { MonitorSnapshot, ProcessSample } from './types';
export function usageColor(value: number | null): string;
export const metricNames: Record<string, string>;
export function metricValue(
  snapshot: MonitorSnapshot | null,
  metric: string,
  volume?: string,
): number | null;
export function processValue(process: ProcessSample, metric: string): number | null;
export function rankedProcesses(processes: ProcessSample[], metric: string): ProcessSample[];
