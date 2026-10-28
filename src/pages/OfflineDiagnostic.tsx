import { t, tx, currentLocale } from '../i18n.mjs';
import { useEffect, useState } from 'react';
import {
  ArrowRight,
  ArrowUpRight,
  CircleHelp,
  MonitorUp,
  Power,
  RefreshCw,
  RotateCcw,
} from 'lucide-react';
import { isDesktop, openOfflineTool, rpc, setStartup } from '../api';
import { useApp } from '../app-context';
import { Notice, Spinner } from '../components';
interface BootStatus {
  supported: boolean;
  pending: boolean;
  requested_at: number | null;
  last_report_id: string | null;
  platform: string;
}
export default function OfflineDiagnostic() {
  const { inventory, notify, reloadHistory, navigate } = useApp();
  const [status, setStatus] = useState<BootStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [reopen, setReopen] = useState(false);
  const refresh = () =>
    rpc<BootStatus>('offline_status')
      .then(setStatus)
      .catch((e) => notify(e.message, true));
  useEffect(() => {
    void refresh();
  }, []);
  const windows = status?.platform === 'windows';
  const mac = status?.platform === 'macos' || inventory?.os.toLowerCase().includes('mac');
  const launch = async () => {
    setBusy(true);
    try {
      if (reopen) await setStartup(true);
      await openOfflineTool();
      await refresh();
      notify('Le dialogue Windows est ouvert. Choisissez vous-même quand redémarrer.');
    } catch (e) {
      notify((e as Error).message, true);
    } finally {
      setBusy(false);
    }
  };
  const collect = async () => {
    setBusy(true);
    try {
      const result = await rpc<{
        imported: boolean;
        reason?: string;
      }>('offline_collect');
      await refresh();
      await reloadHistory();
      notify(
        result.imported
          ? 'Le résultat Windows a été ajouté à l’historique.'
          : result.reason || 'Le résultat du diagnostic n’est pas encore disponible.',
      );
      if (result.imported) navigate('history');
    } catch (e) {
      notify((e as Error).message, true);
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="offline-section">
      <div className="offline-symbol">
        <Power size={25} />
      </div>
      <div className="offline-copy">
        <span className="eyebrow">{tx('ALLER PLUS LOIN')}</span>
        <h2>{tx('Et si on testait avant le r\xE9veil du PC ?')}</h2>
        <p>
          {tx(
            'Un test autonome acc\xE8de \xE0 plus de m\xE9moire que Computy dans l\u2019OS. Le parcours d\xE9pend de votre syst\xE8me.',
          )}
        </p>
        {tx(
          windows ? (
            <>
              <ol className="offline-steps">
                <li>
                  {tx(
                    'Computy ouvre le diagnostic m\xE9moire de Windows. Vous choisissez le prochain red\xE9marrage dans le dialogue syst\xE8me.',
                  )}
                </li>
                <li>
                  {tx(
                    'Le test s\u2019ex\xE9cute avant le retour sur le bureau, puis Windows red\xE9marre.',
                  )}
                </li>
                <li>
                  {tx(
                    '\xC0 l\u2019ouverture de Computy, un nouvel \xE9v\xE9nement de r\xE9sultat est r\xE9cup\xE9r\xE9 et ajout\xE9 \xE0 l\u2019historique.',
                  )}
                </li>
              </ol>
              {status?.pending ? (
                <div className="offline-pending">
                  <span className="badge warn">{tx('R\xE9sultat Windows en attente')}</span>
                  <button
                    className="button secondary small"
                    disabled={busy}
                    onClick={() => void collect()}
                  >
                    {tx(busy ? <Spinner size={14} /> : <RefreshCw size={14} />)}
                    {tx('Rechercher le r\xE9sultat')}
                  </button>
                  <button
                    className="text-button small"
                    onClick={() =>
                      void rpc('offline_forget')
                        .then(() => refresh())
                        .then(() =>
                          notify(
                            'Attente oubliée dans Computy. Cela ne désactive pas un test déjà planifié dans Windows.',
                          ),
                        )
                        .catch((e) => notify(e.message, true))
                    }
                  >
                    {tx('Oublier cette attente')}
                  </button>
                </div>
              ) : (
                <>
                  <label className="offline-checkbox">
                    <input
                      type="checkbox"
                      checked={reopen}
                      disabled={!isDesktop()}
                      onChange={(e) => setReopen(e.target.checked)}
                    />
                    {tx(
                      'Ouvrir Computy \xE0 ma prochaine connexion pour retrouver le r\xE9sultat.',
                    )}
                  </label>
                  <button
                    className="button primary"
                    disabled={busy || !isDesktop()}
                    onClick={() => void launch()}
                  >
                    {tx(busy ? <Spinner size={16} /> : <MonitorUp size={16} />)}
                    {tx('Ouvrir le diagnostic Windows ')}
                    <ArrowRight size={16} />
                  </button>
                </>
              )}
              <small className="offline-caveat">
                {tx(
                  'Le retour automatique n\xE9cessite la version install\xE9e et l\u2019ouverture \xE0 la connexion. Un journal absent ou refus\xE9 reste en attente.',
                )}
              </small>
            </>
          ) : (
            <>
              <div className="offline-platform-note">
                <CircleHelp size={17} />
                <p>
                  {tx(
                    mac
                      ? 'Sur Mac, Apple Diagnostics se lance avec les commandes de démarrage propres au modèle. Apple Silicon utilise de la mémoire intégrée. La récupération automatique des résultats Apple n’est pas fournie par cette version.'
                      : 'Sur Linux, Memtest86+ se lance depuis une clé USB ou une entrée de démarrage compatible. L’installation du chargeur et la récupération de ses rapports dépendent de la distribution et du firmware ; cette version ne modifie pas votre démarrage.',
                  )}
                </p>
              </div>
              <a
                className="button secondary"
                href={mac ? 'https://support.apple.com/fr-fr/102550' : 'https://www.memtest.org/'}
                target="_blank"
                rel="noreferrer"
              >
                {tx(mac ? 'Guide Apple Diagnostics' : 'Parcours Memtest86+')}
                <ArrowUpRight size={16} />
              </a>
              <span className="offline-caveat">
                {tx(
                  'L\u2019ouverture de Computy \xE0 la connexion est disponible dans les pr\xE9f\xE9rences de l\u2019application install\xE9e.',
                )}
              </span>
            </>
          ),
        )}
        {tx(
          !isDesktop() && (
            <span className="preview-note">
              {tx(
                'Aper\xE7u de d\xE9veloppement : les r\xE9glages de d\xE9marrage n\xE9cessitent l\u2019application Tauri.',
              )}
            </span>
          ),
        )}
      </div>
    </section>
  );
}
