import type { Preferences, Report } from './types';
import { invoke } from '@tauri-apps/api/core';
export const isDesktop = () => !!window.__TAURI_INTERNALS__;
export async function rpc<T>(method: string, params: unknown = {}): Promise<T> {
  if (isDesktop()) return invoke<T>('engine_call', { method, params });
  const response = await fetch('/api/computy', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'X-Computy-Client': 'local-ui' },
    body: JSON.stringify({ method, params }),
  });
  const data = await response.json();
  if (!response.ok || data.error) throw new Error(data.error || 'Le moteur est inaccessible.');
  return data.result;
}
export async function exportReport(report: Report) {
  if (isDesktop()) return invoke<boolean>('export_report', { id: report.id });
  const url = URL.createObjectURL(
    new Blob([JSON.stringify(report, null, 2)], { type: 'application/json' }),
  );
  const link = document.createElement('a');
  link.href = url;
  link.download = `computy-${report.id}.json`;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
  return true;
}
export const defaultPreferences: Preferences = {
  memory_mib: 256,
  passes: 2,
  disk_mib: 64,
  cpu_seconds: 5,
  theme: 'system',
  language: 'system',
  tray_enabled: true,
  tray_style: 'icon_text',
  tray_components: ['memory', 'cpu', 'gpu'],
  tray_disks: [],
};
export function loadPreferences(): Preferences {
  try {
    const saved = JSON.parse(localStorage.getItem('computy-preferences') || '{}');
    return { ...defaultPreferences, ...saved };
  } catch {
    return { ...defaultPreferences };
  }
}
export async function startupStatus(): Promise<boolean> {
  return isDesktop() ? invoke<boolean>('startup_status') : false;
}
export async function setStartup(enabled: boolean): Promise<void> {
  if (!isDesktop())
    throw new Error('Cette option est disponible dans l’application Tauri installée.');
  return invoke('set_startup', { enabled });
}
export async function openOfflineTool(): Promise<void> {
  if (!isDesktop())
    throw new Error('Ouvrez l’application Tauri installée pour lancer le diagnostic système.');
  return invoke('open_offline_tool');
}
