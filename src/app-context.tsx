import { createContext, useContext } from 'react';
import type { Inventory, Job, Page, Preferences, Report, RunOptions } from './types';
export interface AppModel {
  page: Page;
  navigate: (page: Page) => void;
  inventory: Inventory | null;
  loading: boolean;
  error: string | null;
  refreshing: boolean;
  refresh: () => Promise<void>;
  reports: Report[];
  unreadable: number;
  reloadHistory: () => Promise<void>;
  job: Job | null;
  launching: boolean;
  start: (options: RunOptions) => Promise<void>;
  relaunch: (report: Report) => Promise<void>;
  diagnosticMode: 'session' | 'boot';
  setDiagnosticMode: (mode: 'session' | 'boot') => void;
  cancel: () => Promise<void>;
  selectedIds: string[];
  toggleSelection: (id: string) => void;
  setSelectedIds: (ids: string[]) => void;
  openReport: (report: Report) => void;
  exportReport: (report: Report) => void;
  notify: (message: string, error?: boolean) => void;
  preferences: Preferences;
  savePreferences: (preferences: Preferences) => void;
}
export const AppContext = createContext<AppModel | null>(null);
export function useApp() {
  const context = useContext(AppContext);
  if (!context) throw new Error('AppContext manquant');
  return context;
}
