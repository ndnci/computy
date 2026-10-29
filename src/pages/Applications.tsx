import { useEffect, useMemo, useState } from 'react';
import {
  AppWindow,
  ExternalLink,
  FolderOpen,
  Package,
  Play,
  RefreshCw,
  Search,
  Trash2,
} from 'lucide-react';
import { useApp } from '../app-context';
import { rpc } from '../api';
import { tx, t } from '../i18n.mjs';
import { Modal, SectionTitle, Spinner } from '../components';
interface Application {
  id: string;
  name: string;
  version: string;
  manager: string;
  package: string;
  location: string | null;
  launcher: string | null;
  launch_target: string | null;
  can_uninstall: boolean;
  can_remove_data: boolean;
  system: boolean;
}
export default function Applications() {
  const { notify } = useApp();
  const [apps, setApps] = useState<Application[]>([]);
  const [warnings, setWarnings] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState('');
  const [query, setQuery] = useState('');
  const [manager, setManager] = useState('all');
  const [showPackages, setShowPackages] = useState(false);
  const [visible, setVisible] = useState(60);
  const [selected, setSelected] = useState<Application | null>(null);
  const [removal, setRemoval] = useState(false);
  const [details, setDetails] = useState('');
  const [removeData, setRemoveData] = useState(false);
  const load = async () => {
    setLoading(true);
    try {
      const result = await rpc<{ apps: Application[]; warnings: string[] }>('applications');
      setApps(result.apps);
      setWarnings(result.warnings);
    } catch (e) {
      notify(String(e), true);
    } finally {
      setLoading(false);
    }
  };
  useEffect(() => {
    void load();
  }, []);
  const filtered = useMemo(
    () =>
      apps.filter(
        (app) =>
          (showPackages || !app.system) &&
          (manager === 'all' || app.manager === manager) &&
          `${app.name} ${app.package} ${app.location ?? ''}`
            .toLowerCase()
            .includes(query.toLowerCase()),
      ),
    [apps, manager, query, showPackages],
  );
  const action = async (app: Application, name: string) => {
    setBusy(app.id);
    try {
      await rpc('application_action', { id: app.id, action: name });
      notify('Action terminée.');
    } catch (e) {
      notify(String(e), true);
    } finally {
      setBusy('');
    }
  };
  const preview = async () => {
    if (!selected) return;
    setBusy(selected.id);
    try {
      const result = await rpc<{ details: string }>('application_action', {
        id: selected.id,
        action: 'preview_uninstall',
      });
      setDetails(result.details);
      setRemoveData(false);
      setRemoval(true);
    } catch (e) {
      notify(String(e), true);
    } finally {
      setBusy('');
    }
  };
  const uninstall = async () => {
    if (!selected) return;
    setBusy(selected.id);
    try {
      await rpc('application_action', {
        id: selected.id,
        action: 'uninstall',
        remove_data: removeData,
        confirmed: true,
      });
      setSelected(null);
      setRemoval(false);
      notify('Désinstallation terminée.');
      await load();
    } catch (e) {
      notify(String(e), true);
    } finally {
      setBusy('');
    }
  };
  return (
    <div className="page-enter utility-page applications-page">
      <SectionTitle
        eyebrow={tx('LOGICIELS')}
        title={tx('Vos applications, au même endroit.')}
        description={tx(
          'Retrouvez une installation, ouvrez une application ou utilisez son gestionnaire pour la désinstaller.',
        )}
      />
      <section className="panel utility-panel">
        <div className="application-toolbar">
          <label className="utility-search">
            <Search size={17} />
            <input
              placeholder={tx('Rechercher une application…')}
              value={query}
              onChange={(e) => {
                setQuery(e.target.value);
                setVisible(60);
              }}
            />
          </label>
          <select
            aria-label={tx('Gestionnaire')}
            value={manager}
            onChange={(e) => {
              setManager(e.target.value);
              setVisible(60);
            }}
          >
            <option value="all">{tx('Tous les gestionnaires')}</option>
            {[...new Set(apps.map((app) => app.manager))].sort().map((m) => (
              <option key={m} value={m}>
                {m}
              </option>
            ))}
          </select>
          <button className="button secondary small" disabled={loading} onClick={() => void load()}>
            {loading ? <Spinner size={15} /> : <RefreshCw size={15} />} {tx('Actualiser')}
          </button>
        </div>
        <div className="utility-actions">
          <label className="utility-check">
            <input
              type="checkbox"
              checked={showPackages}
              onChange={(e) => {
                setShowPackages(e.target.checked);
                setVisible(60);
              }}
            />
            {tx('Afficher aussi les paquets système')}
          </label>
          <span>{t('{{count}} applications', { count: filtered.length })}</span>
        </div>
        {warnings.map((w, i) => (
          <p className="utility-warning" key={i}>
            {w}
          </p>
        ))}
        {loading ? (
          <div className="utility-empty">
            <Spinner />
            <p>{tx('Inventaire en cours…')}</p>
          </div>
        ) : !filtered.length ? (
          <div className="utility-empty">
            <Package size={32} />
            <p>{tx('Aucune application trouvée.')}</p>
          </div>
        ) : (
          <div className="application-grid">
            {filtered.slice(0, visible).map((app) => (
              <button
                className="application-card"
                key={app.id}
                onClick={() => {
                  setSelected(app);
                  setRemoval(false);
                }}
              >
                <span className="application-avatar">
                  <AppWindow size={24} />
                </span>
                <div>
                  <h3>{app.name}</h3>
                  <p>{app.version || app.package}</p>
                  <span className="utility-tag">{app.manager}</span>
                </div>
                <ExternalLink size={15} />
              </button>
            ))}
          </div>
        )}
        {visible < filtered.length && (
          <button className="button secondary" onClick={() => setVisible((n) => n + 60)}>
            {tx('Voir plus')}
          </button>
        )}
      </section>
      {selected && (
        <Modal
          title={selected.name}
          onClose={() => {
            if (!busy) {
              setSelected(null);
              setRemoval(false);
            }
          }}
        >
          <div className="utility-modal">
            <div className="application-detail-heading">
              <span className="application-avatar">
                <AppWindow size={32} />
              </span>
              <div>
                <h2>{selected.name}</h2>
                <p>
                  {selected.version} · {selected.manager}
                </p>
              </div>
            </div>
            <dl className="application-facts">
              <dt>{tx('Identifiant')}</dt>
              <dd>{selected.package || selected.id}</dd>
              <dt>{tx('Installation')}</dt>
              <dd>{selected.location || tx('Inconnue')}</dd>
              <dt>{tx('Lanceur')}</dt>
              <dd>{selected.launcher || selected.launch_target || tx('Indisponible')}</dd>
            </dl>
            {removal ? (
              <>
                <h3>{tx('Vérifier la désinstallation')}</h3>
                <pre className="uninstall-preview">{details}</pre>
                <p>
                  {tx(
                    'Le gestionnaire peut retirer des dépendances. Vérifiez la liste avant de confirmer. Une autorisation du système peut être demandée.',
                  )}
                </p>
                <label className="utility-check">
                  <input
                    type="checkbox"
                    disabled={!selected.can_remove_data || !!busy}
                    checked={removeData}
                    onChange={(e) => setRemoveData(e.target.checked)}
                  />
                  <span>
                    {tx('Supprimer aussi les données gérées par le paquet')}
                    <small>
                      {selected.can_remove_data
                        ? tx('Les documents enregistrés ailleurs restent conservés.')
                        : tx(
                            'Le gestionnaire ne permet pas de repérer et supprimer toutes les données avec certitude.',
                          )}
                    </small>
                  </span>
                </label>
                <div className="utility-actions">
                  <button
                    className="button secondary"
                    disabled={!!busy}
                    onClick={() => setRemoval(false)}
                  >
                    {tx('Annuler')}
                  </button>
                  <button
                    className="button primary"
                    disabled={!!busy}
                    onClick={() => void uninstall()}
                  >
                    {busy && <Spinner />}
                    <Trash2 size={16} />
                    {tx('Confirmer la désinstallation')}
                  </button>
                </div>
              </>
            ) : (
              <div className="application-detail-actions">
                <button
                  className="button primary"
                  disabled={
                    !!busy ||
                    !(
                      selected.launcher ||
                      selected.launch_target ||
                      ['snap', 'flatpak'].includes(selected.manager)
                    )
                  }
                  onClick={() => void action(selected, 'launch')}
                >
                  <Play size={16} />
                  {tx('Lancer')}
                </button>
                <button
                  className="button secondary"
                  disabled={!!busy || !selected.location}
                  onClick={() => void action(selected, 'reveal')}
                >
                  <FolderOpen size={16} />
                  {tx('Ouvrir le dossier')}
                </button>
                <button
                  className="button secondary"
                  disabled={!!busy || !selected.can_uninstall}
                  onClick={() => void preview()}
                >
                  {busy ? <Spinner /> : <Trash2 size={16} />} {tx('Désinstaller')}
                </button>
              </div>
            )}
          </div>
        </Modal>
      )}
    </div>
  );
}
