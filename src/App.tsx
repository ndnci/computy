import { t, tx, currentLocale, setLanguage, languages } from './i18n.mjs';
import { version } from '../package.json';
import { useTranslation } from 'react-i18next';
import { useCallback, useEffect, useRef, useState } from 'react';
import {
  Activity,
  ArrowLeftRight,
  ArrowUpRight,
  Check,
  ChevronRight,
  CircleHelp,
  Cpu,
  HardDrive,
  History,
  LayoutDashboard,
  LockKeyhole,
  MemoryStick,
  Monitor,
  Plus,
  Power,
  RefreshCw,
  Settings2,
  ShieldCheck,
  X,
} from 'lucide-react';
import { AppContext } from './app-context';
import {
  exportReport as exportFile,
  isDesktop,
  loadPreferences,
  quitApplication,
  rpc,
} from './api';
import { Logo, Modal, Progress, ReportDetails, Spinner } from './components';
import Overview from './pages/Overview';
import Hardware from './pages/Hardware';
import Diagnostics from './pages/Diagnostics';
import HistoryPage from './pages/History';
import Compare from './pages/Compare';
import SettingsPage from './pages/Settings';
import About from './pages/About';
import Processes from './pages/Processes';
import type { Inventory, Job, Page, Preferences, Report, RunOptions } from './types';
const navigation: {
  id: Page;
  label: string;
  icon: typeof Activity;
  group: number;
}[] = [
  {
    id: 'overview',
    label: 'Vue d’ensemble',
    icon: LayoutDashboard,
    group: 0,
  },
  {
    id: 'diagnostics',
    label: 'Diagnostics',
    icon: Activity,
    group: 0,
  },
  {
    id: 'memory',
    label: 'Mémoire vive',
    icon: MemoryStick,
    group: 1,
  },
  {
    id: 'storage',
    label: 'Stockage',
    icon: HardDrive,
    group: 1,
  },
  {
    id: 'cpu',
    label: 'Processeur',
    icon: Cpu,
    group: 1,
  },
  {
    id: 'history',
    label: 'Historique',
    icon: History,
    group: 2,
  },
  { id: 'processes', label: 'Processus', icon: Activity, group: 1 },
  {
    id: 'compare',
    label: 'Comparaison',
    icon: ArrowLeftRight,
    group: 2,
  },
];
function pageFromHash(): Page {
  const hash = window.location.hash.slice(1).split('?')[0];
  return [...navigation.map((n) => n.id), 'settings', 'about'].includes(hash as Page)
    ? (hash as Page)
    : 'overview';
}
export default function App() {
  const { i18n } = useTranslation();
  const [page, setPage] = useState<Page>(pageFromHash);
  const [inventory, setInventory] = useState<Inventory | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [reports, setReports] = useState<Report[]>([]);
  const [unreadable, setUnreadable] = useState(0);
  const [job, setJob] = useState<Job | null>(null);
  const [launching, setLaunching] = useState(false);
  const starting = useRef(false);
  const [diagnosticMode, setDiagnosticMode] = useState<'session' | 'boot'>('session');
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [report, setReport] = useState<Report | null>(null);
  const [toast, setToast] = useState<{
    message: string;
    error: boolean;
  } | null>(null);
  const [preferences, setPreferences] = useState<Preferences>(loadPreferences);
  const navigate = useCallback((destination: Page) => {
    setPage(destination);
    window.location.hash = destination;
    window.scrollTo({
      top: 0,
    });
  }, []);
  const notify = useCallback(
    (message: string, isError = false) =>
      setToast({
        message,
        error: isError,
      }),
    [],
  );
  const reloadHistory = useCallback(async () => {
    try {
      const data = await rpc<{
        reports: Report[];
        unreadable: number;
      }>('history');
      setReports(data.reports);
      setUnreadable(data.unreadable);
    } catch (e) {
      notify((e as Error).message, true);
    }
  }, [notify]);
  const refresh = useCallback(
    async (announce = true) => {
      setRefreshing(true);
      try {
        setInventory(
          await rpc<Inventory>('inventory', {
            refresh: true,
          }),
        );
        setError(null);
        if (announce) notify('Informations matérielles actualisées.');
      } catch (e) {
        setError((e as Error).message);
        if (announce) notify('Impossible d’actualiser les informations matérielles.', true);
      } finally {
        setRefreshing(false);
        setLoading(false);
      }
    },
    [notify],
  );
  useEffect(() => {
    void refresh(false);
    void reloadHistory();
    void rpc<Job | null>('active')
      .then(setJob)
      .catch(() => {});
    void rpc<Preferences>('preferences')
      .then((value) => {
        setPreferences(value);
        try {
          localStorage.setItem('computy-preferences', JSON.stringify(value));
        } catch {}
      })
      .catch(() => {});
    const listener = () => setPage(pageFromHash());
    window.addEventListener('hashchange', listener);
    return () => window.removeEventListener('hashchange', listener);
  }, [refresh, reloadHistory]);
  useEffect(() => {
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;
    const collect = async () => {
      try {
        const state = await rpc<{
          pending: boolean;
        }>('offline_status');
        if (!state.pending || stopped) return;
        const result = await rpc<{
          imported: boolean;
          report_id?: string;
        }>('offline_collect');
        if (stopped) return;
        if (result.imported && result.report_id) {
          await reloadHistory();
          const imported = await rpc<Report>('report', {
            id: result.report_id,
          });
          if (!stopped) {
            setReport(imported);
            notify('Votre résultat mémoire Windows a été récupéré.');
          }
        } else timer = setTimeout(collect, 15000);
      } catch (e) {
        if (!stopped) notify((e as Error).message, true);
      }
    };
    void collect();
    return () => {
      stopped = true;
      clearTimeout(timer);
    };
  }, [reloadHistory, notify]);
  useEffect(() => {
    const query = window.matchMedia('(prefers-color-scheme: dark)');
    const applyTheme = () => {
      document.documentElement.dataset.theme =
        preferences.theme === 'system' ? (query.matches ? 'dark' : 'light') : preferences.theme;
      document.documentElement.style.colorScheme = document.documentElement.dataset.theme;
    };
    applyTheme();
    query.addEventListener('change', applyTheme);
    return () => query.removeEventListener('change', applyTheme);
  }, [preferences.theme]);
  useEffect(() => {
    const applyLocale = () => {
      document.documentElement.lang = currentLocale();
      document.documentElement.dir =
        languages.find((language) => language.code === currentLocale())?.dir || 'ltr';
      document.title = `Computy — ${t('Votre matériel, en clair.')}`;
    };
    const systemChanged = () => {
      if (preferences.language === 'system') void setLanguage('system');
    };
    i18n.on('languageChanged', applyLocale);
    void setLanguage(preferences.language).then(applyLocale);
    window.addEventListener('languagechange', systemChanged);
    return () => {
      i18n.off('languageChanged', applyLocale);
      window.removeEventListener('languagechange', systemChanged);
    };
  }, [preferences.language, i18n]);
  useEffect(() => {
    if (!toast) return;
    const timeout = setTimeout(() => setToast(null), 6500);
    return () => clearTimeout(timeout);
  }, [toast]);
  useEffect(() => {
    if (!job?.running) return;
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try {
        const next = await rpc<Job>('job', {
          id: job.id,
        });
        if (stopped) return;
        setJob(next);
        if (!next.running) {
          await reloadHistory();
          if (next.error) notify(next.error, true);
          else
            notify(
              next.report?.completed
                ? 'Diagnostic terminé et enregistré.'
                : 'Diagnostic interrompu. Le résultat partiel a été conservé.',
            );
        } else timer = setTimeout(poll, 350);
      } catch (e) {
        if (!stopped) {
          notify((e as Error).message, true);
          timer = setTimeout(poll, 2500);
        }
      }
    };
    timer = setTimeout(poll, 200);
    return () => {
      stopped = true;
      clearTimeout(timer);
    };
  }, [job?.id, job?.running, reloadHistory, notify]);
  const start = async (options: RunOptions) => {
    if (starting.current || job?.running) return;
    starting.current = true;
    setLaunching(true);
    setJob(null);
    try {
      const result = await rpc<{
        id: string;
      }>('start', options);
      setJob(
        await rpc<Job>('job', {
          id: result.id,
        }),
      );
    } catch (e) {
      notify((e as Error).message, true);
    } finally {
      starting.current = false;
      setLaunching(false);
    }
  };
  const relaunch = async (item: Report) => {
    if (starting.current || job?.running) return;
    setReport(null);
    navigate('diagnostics');
    if (item.source && item.source !== 'computy_live') {
      setDiagnosticMode('boot');
      notify('Choisissez et confirmez le prochain test avant démarrage dans l’assistant.');
      return;
    }
    setDiagnosticMode('session');
    await start({ ...item.options, components: [...item.options.components], label: item.label });
  };
  const cancel = async () => {
    if (!job) return;
    try {
      await rpc('cancel', {
        id: job.id,
      });
      setJob(
        await rpc<Job>('job', {
          id: job.id,
        }),
      );
    } catch (e) {
      notify((e as Error).message, true);
    }
  };
  const toggleSelection = (id: string) => {
    setSelectedIds((previous) => {
      if (previous.includes(id)) return previous.filter((value) => value !== id);
      if (previous.length >= 4) {
        notify('Vous pouvez comparer jusqu’à quatre diagnostics.');
        return previous;
      }
      return [...previous, id];
    });
  };
  const exportReport = async (item: Report) => {
    try {
      if (await exportFile(item)) notify('Rapport exporté.');
    } catch (e) {
      notify((e as Error).message, true);
    }
  };
  const savePreferences = async (value: Preferences) => {
    try {
      await rpc('save_preferences', value);
      try {
        localStorage.setItem('computy-preferences', JSON.stringify(value));
      } catch {}
      setPreferences(value);
      notify('Préférences enregistrées sur cet appareil.');
    } catch {
      notify('Impossible d’enregistrer les préférences.', true);
    }
  };
  const label =
    page === 'settings'
      ? 'Préférences'
      : page === 'about'
        ? 'À propos'
        : navigation.find((n) => n.id === page)?.label;
  const closeReport = useCallback(() => setReport(null), []);
  return (
    <AppContext.Provider
      value={{
        page,
        navigate,
        inventory,
        loading,
        error,
        refreshing,
        refresh,
        reports,
        unreadable,
        reloadHistory,
        job,
        launching,
        start,
        relaunch,
        diagnosticMode,
        setDiagnosticMode,
        cancel,
        selectedIds,
        toggleSelection,
        setSelectedIds,
        openReport: setReport,
        exportReport,
        notify,
        preferences,
        savePreferences,
      }}
    >
      <a
        href="#main-content"
        className="skip-link"
        onClick={(e) => {
          e.preventDefault();
          document.getElementById('main-content')?.focus();
        }}
      >
        {tx('Aller au contenu')}
      </a>
      <div className="app-shell">
        <aside className="sidebar">
          <button
            className="brand"
            onClick={() => navigate('overview')}
            aria-label={tx('Computy, accueil')}
          >
            <Logo />
            <span>
              {tx('computy')}
              <span className="brand-dot">{tx('.')}</span>
              <small>{tx('VOTRE MAT\xC9RIEL, EN CLAIR')}</small>
            </span>
          </button>
          <div className="workspace-label">
            <span className="online-dot" />
            {tx(' Espace personnel ')}
            <span className="local-tag">{tx('LOCAL')}</span>
          </div>
          <nav aria-label={tx('Navigation principale')}>
            {tx(
              [0, 1, 2].map((group) => (
                <div className="nav-group" key={group}>
                  {tx(
                    group > 0 && (
                      <p className="nav-group-label">
                        {tx(group === 1 ? 'VOTRE MATÉRIEL' : 'DANS LE TEMPS')}
                      </p>
                    ),
                  )}
                  {tx(
                    navigation
                      .filter((item) => item.group === group)
                      .map(({ id, label, icon: Icon }) => (
                        <button
                          key={id}
                          className={`nav-item ${page === id ? 'active' : ''}`}
                          aria-current={page === id ? 'page' : undefined}
                          onClick={() => navigate(id)}
                        >
                          <Icon size={19} strokeWidth={1.7} />
                          <span>{tx(label)}</span>
                          {tx(
                            id === 'diagnostics' && job?.running ? (
                              <span className="live-dot" />
                            ) : id === 'history' && reports.length > 0 ? (
                              <span className="nav-count">{tx(reports.length)}</span>
                            ) : id === 'compare' && selectedIds.length > 0 ? (
                              <span className="nav-count">{tx(selectedIds.length)}</span>
                            ) : null,
                          )}
                        </button>
                      )),
                  )}
                </div>
              )),
            )}
          </nav>
          <div className="sidebar-bottom">
            <div className="local-card">
              <div className="inline-icon-heading">
                <span className="local-card-icon">
                  <LockKeyhole size={18} />
                </span>
                <b>{tx('Votre PC. Vos donn\xE9es.')}</b>
              </div>
              <p>
                {tx('Les diagnostics restent')}
                <br />
                {tx('sur votre ordinateur.')}
              </p>
              <span>
                <span className="online-dot" />
                {tx(' Aucune t\xE9l\xE9m\xE9trie')}
              </span>
            </div>
            <button
              className={`nav-item ${page === 'settings' ? 'active' : ''}`}
              onClick={() => navigate('settings')}
            >
              <Settings2 size={18} />
              <span>{tx('Pr\xE9f\xE9rences')}</span>
            </button>
            <button
              className={`nav-item ${page === 'about' ? 'active' : ''}`}
              onClick={() => navigate('about')}
            >
              <CircleHelp size={18} />
              <span>{tx('\xC0 propos de Computy')}</span>
            </button>
            <a className="made-by" href="https://www.ndnci.com" target="_blank" rel="noreferrer">
              {tx('Con\xE7u par ')}
              <b>{tx('NDNCI')}</b>
              <ArrowUpRight size={13} />
            </a>
            {isDesktop() && (
              <button
                className="nav-item quit-application"
                onClick={() => void quitApplication().catch((e) => notify(e.message, true))}
              >
                <Power size={18} />
                <span>{tx('Quitter Computy')}</span>
              </button>
            )}
          </div>
        </aside>
        <div className="main-shell">
          <header className="topbar">
            <div className="breadcrumb">
              <Monitor size={16} />
              <span>{tx('Mon ordinateur')}</span>
              <ChevronRight size={14} />
              <b>{tx(label)}</b>
            </div>
            <div className="topbar-right">
              <span className="connection">
                <span className={`online-dot ${error ? 'offline' : ''}`} />
                {tx(
                  loading
                    ? 'Connexion au moteur'
                    : error
                      ? 'Moteur indisponible'
                      : 'Moteur Rust connecté',
                )}
              </span>
              <span className="topbar-divider" />
              <button
                className="icon-button refresh-button"
                onClick={() => void refresh()}
                disabled={refreshing || !!job?.running}
                aria-label={tx('Actualiser les informations')}
                title={tx('Actualiser les informations')}
              >
                <RefreshCw size={17} className={refreshing ? 'spin' : ''} />
                <span>{tx('Actualiser les informations')}</span>
              </button>
              <div className="user-monogram" title={tx('Cet ordinateur')}>
                {tx(inventory?.host_name.slice(0, 2).toUpperCase() || 'PC')}
              </div>
            </div>
          </header>
          <main id="main-content" className="main-content" tabIndex={-1}>
            {tx(
              error && (
                <div className="engine-error" role="alert">
                  <div>
                    <b>{tx('Connexion au moteur natif impossible')}</b>
                    <p>{tx(error)}</p>
                  </div>
                  <button className="button secondary small" onClick={() => void refresh()}>
                    {tx('R\xE9essayer')}
                  </button>
                </div>
              ),
            )}
            {tx(
              page === 'overview' ? (
                <Overview />
              ) : ['memory', 'storage', 'cpu'].includes(page) ? (
                <Hardware component={page as 'memory' | 'storage' | 'cpu'} />
              ) : page === 'diagnostics' ? (
                <Diagnostics />
              ) : page === 'history' ? (
                <HistoryPage />
              ) : page === 'compare' ? (
                <Compare />
              ) : page === 'processes' ? (
                <Processes />
              ) : page === 'settings' ? (
                <SettingsPage />
              ) : (
                <About />
              ),
            )}
          </main>
          <footer className="main-footer">
            <span>
              <ShieldCheck size={14} />
              {tx(' Diagnostics locaux, donn\xE9es priv\xE9es.')}
            </span>
            <span>
              {tx('Computy ')}
              <b>{version}</b>
              <i />
              {tx(' Fait avec soin par ')}
              <a href="https://www.ndnci.com" target="_blank" rel="noreferrer">
                {tx('NDNCI ')}
                <ArrowUpRight size={12} />
              </a>
            </span>
          </footer>
        </div>
      </div>
      {tx(
        job?.running && page !== 'diagnostics' && (
          <button className="floating-job" onClick={() => navigate('diagnostics')}>
            <Spinner />
            <div>
              <b>{tx('Diagnostic en cours')}</b>
              <span>{tx(job.phase)}</span>
              <Progress value={job.progress} />
            </div>
            <b>
              {tx(Math.round(job.progress * 100))}
              {tx(' %')}
            </b>
            <ChevronRight size={18} />
          </button>
        ),
      )}
      {tx(
        toast && (
          <div
            className={`toast ${toast.error ? 'error' : ''}`}
            role={toast.error ? 'alert' : 'status'}
          >
            <span>{tx(toast.error ? <CircleHelp size={18} /> : <Check size={18} />)}</span>
            <p>{tx(toast.message)}</p>
            <button
              className="icon-button"
              onClick={() => setToast(null)}
              aria-label={tx('Fermer la notification')}
            >
              <X size={16} />
            </button>
          </div>
        ),
      )}
      {tx(
        report && (
          <Modal title={tx('Rapport de diagnostic')} onClose={closeReport} wide>
            <ReportDetails
              report={report}
              onExport={exportReport}
              onRelaunch={relaunch}
              relaunchDisabled={launching || !!job?.running}
            />
          </Modal>
        ),
      )}
    </AppContext.Provider>
  );
}
