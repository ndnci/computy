import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { PackageMinus, Trash2 } from 'lucide-react';
import { tx } from '../i18n.mjs';
import { isDesktop } from '../api';
import { useApp } from '../app-context';
import { Modal, Panel, PanelHeader, Spinner } from '../components';

export default function MaintenanceSettings() {
  const { job, notify } = useApp();
  const [kind, setKind] = useState('portable');
  const [action, setAction] = useState<'uninstall' | 'erase' | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    if (isDesktop())
      void invoke<string>('maintenance_info')
        .then(setKind)
        .catch((e) => notify(String(e), true));
  }, [notify]);
  const title = action === 'erase' ? 'Supprimer toutes les données' : 'Désinstaller Computy';
  const explain =
    'L’historique, les préférences et les imports en attente seront supprimés. Les exports personnels sont conservés.';
  const close = () => {
    if (!busy) {
      setAction(null);
      setConfirmed(false);
    }
  };
  const run = async () => {
    if (!action || !confirmed || busy) return;
    setBusy(true);
    try {
      await invoke('maintenance_action', { action, confirmed: true });
      setAction(null);
      setConfirmed(false);
    } catch (e) {
      notify(String(e), true);
    } finally {
      setBusy(false);
    }
  };
  return (
    <Panel className="maintenance-settings">
      <PanelHeader title={tx('Installation et données')} />
      <div className="setting-row">
        <span className="setting-icon">
          <PackageMinus size={22} />
        </span>
        <div>
          <b>{tx('Désinstaller Computy')}</b>
          <p>
            {tx(
              kind === 'portable'
                ? 'Version portable : supprimez le fichier ou le dossier de l’application.'
                : 'Le système demandera votre autorisation. Les données sont conservées.',
            )}
          </p>
        </div>
        <button
          className="button secondary small"
          disabled={!isDesktop() || kind === 'portable' || !!job?.running}
          onClick={() => setAction('uninstall')}
        >
          <PackageMinus size={16} />
          {tx('Désinstaller Computy')}
        </button>
      </div>
      <div className="setting-row">
        <span className="setting-icon">
          <Trash2 size={22} />
        </span>
        <div>
          <b>{tx('Supprimer toutes les données')}</b>
          <p>{tx(explain)}</p>
        </div>
        <button
          className="button danger-button small"
          disabled={!isDesktop() || !!job?.running}
          onClick={() => setAction('erase')}
        >
          <Trash2 size={16} />
          {tx('Supprimer toutes les données')}
        </button>
      </div>
      {action && (
        <Modal title={title} onClose={close}>
          <div className="maintenance-confirm">
            <p>
              {tx(
                action === 'erase'
                  ? explain
                  : 'Le système demandera votre autorisation. Les données sont conservées.',
              )}
            </p>
            {action === 'erase' && (
              <p>
                <strong>{tx('Cette action est irréversible.')}</strong>
              </p>
            )}
            <label className="check-option">
              <input
                type="checkbox"
                checked={confirmed}
                disabled={busy}
                onChange={(e) => setConfirmed(e.target.checked)}
              />
              {tx(title)}
            </label>
            <div className="settings-actions">
              <button className="button secondary" disabled={busy} onClick={close}>
                {tx('Annuler')}
              </button>
              <button
                className="button danger-button"
                disabled={!confirmed || busy}
                onClick={() => void run()}
              >
                {busy ? <Spinner size={16} /> : <Trash2 size={16} />}
                {tx(title)}
              </button>
            </div>
          </div>
        </Modal>
      )}
    </Panel>
  );
}
