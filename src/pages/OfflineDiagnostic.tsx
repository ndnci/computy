import { t, tx } from '../i18n.mjs';
import { useEffect, useState } from 'react';
import {
  ArrowRight,
  ArrowUpRight,
  CircleHelp,
  MonitorUp,
  Power,
  RefreshCw,
  Usb,
  ShieldCheck,
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { isDesktop, openOfflineTool, rpc, setStartup } from '../api';
import { useApp } from '../app-context';
import { Notice, Spinner } from '../components';
import { formatBytes } from '../domain.mjs';
interface BootStatus {
  supported: boolean;
  pending: boolean;
  platform: string;
  linux?: {
    code: string;
    returned: boolean;
    confirmed: boolean;
    pending: boolean;
    facts: { secure_boot: boolean | null; payload_installed: boolean };
  };
}
interface UsbTarget {
  id: string;
  label: string;
  mount: string;
  size_bytes: number;
}
const reasons: Record<string, string> = {
  secure_boot:
    'Secure Boot est activé. Le moteur intégré n’est pas approuvé par votre firmware. Computy ne désactive pas cette protection : ni le démarrage interne ni la clé USB ne peuvent contourner ce blocage.',
  secure_boot_unknown:
    'Computy ne peut pas déterminer l’état de Secure Boot. La préparation automatique est bloquée pour éviter un démarrage incompatible.',
  helper_missing:
    'Installez le paquet Computy pour activer l’assistant système et le moteur de test intégré. Le binaire portable seul ne modifie pas le démarrage.',
  payload_missing:
    'Le composant de test intégré est absent. Réinstallez le paquet Computy ; aucun téléchargement séparé de Memtest86+ n’est nécessaire.',
  container: 'Ce parcours nécessite le PC réel. Il est désactivé dans un conteneur.',
  architecture:
    'Le démarrage automatique du composant intégré est actuellement disponible sur Linux x86-64 avec GRUB.',
  unsupported_boot_storage:
    'Le stockage de démarrage ne permet pas de garantir le retour automatique. Computy refuse les configurations Btrfs, RAID, LVM ou chiffrées pour ce parcours.',
  unsupported_grub_header:
    'Cette configuration GRUB ne garantit pas une sélection pour un seul démarrage. Aucun changement n’a été effectué.',
  entry_missing:
    'L’entrée Computy est absente du menu GRUB. L’installation du paquet doit créer cette entrée sans changer le système lancé par défaut.',
  another_boot_pending:
    'Un autre démarrage ponctuel est déjà programmé. Computy le conserve et refuse de l’écraser.',
  request_already_pending:
    'Une demande est déjà enregistrée. Consultez-la ou annulez-la avant de recommencer.',
  authorization_or_helper_failed:
    'L’autorisation administrateur a été refusée ou l’assistant système n’a pas pu démarrer.',
  usb_existing_loader:
    'Cette clé contient déjà un chargeur de démarrage. Computy refuse de l’écraser. Choisissez une autre clé.',
  usb_not_found: 'La clé sélectionnée n’est plus disponible. Recherchez les clés à nouveau.',
  usb_invalid_directory:
    'La structure de cette clé ne permet pas une préparation sûre. Aucun fichier existant n’a été remplacé.',
};
const explain = (error: string) => tx(reasons[error.replace(/^boot:/, '')] || error);
export default function OfflineDiagnostic() {
  const { notify, reloadHistory, navigate } = useApp();
  const [status, setStatus] = useState<BootStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [reopen, setReopen] = useState(true);
  const [route, setRoute] = useState<'internal' | 'usb'>('internal');
  const [confirmed, setConfirmed] = useState(false);
  const [targets, setTargets] = useState<UsbTarget[]>([]);
  const [selected, setSelected] = useState('');
  const [usbReady, setUsbReady] = useState(false);
  const [problem, setProblem] = useState('');
  const refresh = () =>
    rpc<BootStatus>('offline_status')
      .then(setStatus)
      .catch((error) => setProblem(error.message));
  useEffect(() => {
    void refresh();
  }, []);
  const windows = status?.platform === 'windows';
  const linux = status?.platform === 'linux';
  const code = status?.linux?.code || 'helper_missing';
  const blocked = linux && code !== 'ready';
  const selectedTarget = targets.find((target) => target.id === selected);
  const act = async (operation: () => Promise<void>) => {
    setBusy(true);
    setProblem('');
    try {
      await operation();
    } catch (error) {
      setProblem((error as Error).message);
    } finally {
      await refresh();
      setBusy(false);
    }
  };
  const launch = () =>
    act(async () => {
      await openOfflineTool();
      if (reopen) {
        try {
          await setStartup(true);
        } catch (error) {
          notify((error as Error).message, true);
        }
      }
      await refresh();
      setConfirmed(false);
      notify(
        windows
          ? 'Le dialogue Windows est ouvert. Choisissez vous-même quand redémarrer.'
          : 'Le prochain démarrage sélectionnera Computy une seule fois. Le système par défaut reste inchangé.',
      );
    });
  const collect = () =>
    act(async () => {
      const result = await rpc<{ imported: boolean; reason?: string }>('offline_collect');
      await refresh();
      await reloadHistory();
      notify(
        result.imported
          ? 'Le résultat Windows a été ajouté à l’historique.'
          : result.reason || 'Le résultat du diagnostic n’est pas encore disponible.',
      );
      if (result.imported) navigate('history');
    });
  return (
    <section className="offline-section guided-boot">
      <div className="offline-symbol">
        <Power size={25} />
      </div>
      <div className="offline-copy">
        <span className="eyebrow">{tx('TEST AVANT DÉMARRAGE')}</span>
        <h2>{tx('Tester la mémoire avant le lancement du système')}</h2>
        <p>
          {tx(
            'Computy prépare le parcours. Vous choisissez le support et le moment du redémarrage. Aucun test au démarrage ne part depuis un profil de session.',
          )}
        </p>
        {linux && (
          <div className="boot-routes" role="group" aria-label={tx('Support du test')}>
            <button
              className={`button ${route === 'internal' ? 'primary' : 'secondary'}`}
              aria-pressed={route === 'internal'}
              onClick={() => {
                setRoute('internal');
                setConfirmed(false);
                setProblem('');
              }}
            >
              <MonitorUp size={18} />
              {tx('Sans clé USB')}
            </button>
            <button
              className={`button ${route === 'usb' ? 'primary' : 'secondary'}`}
              aria-pressed={route === 'usb'}
              onClick={() => {
                setRoute('usb');
                setConfirmed(false);
                setProblem('');
              }}
            >
              <Usb size={18} />
              {tx('Avec clé USB')}
            </button>
          </div>
        )}
        {blocked && (
          <Notice tone="warn">
            <b>{tx('Vérification de compatibilité')}</b>
            <p>{tx(reasons[code] || code)}</p>
          </Notice>
        )}
        {problem && <Notice tone="warn">{explain(problem)}</Notice>}
        {windows ? (
          <>
            <ol className="offline-steps">
              <li>
                {tx(
                  'Computy ouvre le diagnostic mémoire de Windows. Vous choisissez le prochain redémarrage dans le dialogue système.',
                )}
              </li>
              <li>
                {tx('Le test s’exécute avant le retour sur le bureau, puis Windows redémarre.')}
              </li>
              <li>
                {tx(
                  'À l’ouverture de Computy, un nouvel événement de résultat est récupéré et ajouté à l’historique.',
                )}
              </li>
            </ol>
            {status?.pending ? (
              <button className="button secondary" disabled={busy} onClick={() => void collect()}>
                <RefreshCw size={16} />
                {tx('Rechercher le résultat')}
              </button>
            ) : (
              <>
                <label className="offline-checkbox">
                  <input
                    type="checkbox"
                    checked={reopen}
                    onChange={(event) => setReopen(event.target.checked)}
                  />
                  {tx('Rouvrir Computy à la connexion pour retrouver le résultat')}
                </label>
                <button
                  className="button primary"
                  disabled={busy || !isDesktop()}
                  onClick={() => void launch()}
                >
                  {busy ? <Spinner size={16} /> : <MonitorUp size={16} />}{' '}
                  {tx('Ouvrir le diagnostic Windows')}
                </button>
              </>
            )}
          </>
        ) : linux && route === 'internal' ? (
          <>
            <ol className="offline-steps">
              <li>
                {tx(
                  'Computy vérifie le moteur intégré, Secure Boot, GRUB et la possibilité de revenir au système habituel.',
                )}
              </li>
              <li>
                {tx(
                  'Après votre accord et l’autorisation administrateur, seule l’entrée du prochain démarrage change. Aucune clé USB n’est nécessaire.',
                )}
              </li>
              <li>
                {tx(
                  'Vous redémarrez quand vous êtes prêt. Le moteur Computy vérifie la RAM avant Linux ; Échap permet de quitter et redémarrer.',
                )}
              </li>
              <li>
                {tx(
                  'Le démarrage suivant revient au système habituel. Les résultats Linux ne sont pas importés automatiquement : notez le résultat affiché. Une absence de rapport ne devient jamais un succès.',
                )}
              </li>
            </ol>
            {status?.pending ? (
              <>
                <Notice>
                  {tx(
                    status.linux?.returned
                      ? 'Vous êtes revenu dans le système. Computy n’a pas de preuve automatique du résultat affiché avant démarrage.'
                      : 'Une préparation est enregistrée. Vous pouvez encore l’annuler.',
                  )}
                </Notice>
                <div className="boot-routes">
                  <button
                    className="button secondary"
                    disabled={busy}
                    onClick={() =>
                      void act(async () => {
                        await invoke('boot_action', { action: 'cancel' });
                        await refresh();
                      })
                    }
                  >
                    {tx('Annuler la préparation')}
                  </button>
                  {!status.linux?.returned && (
                    <button
                      className="button primary"
                      disabled={busy || blocked || !status.linux?.confirmed || !confirmed}
                      onClick={() =>
                        void act(async () => {
                          await invoke('boot_action', { action: 'reboot' });
                        })
                      }
                    >
                      <Power size={16} />
                      {tx('Redémarrer maintenant')}
                    </button>
                  )}
                </div>
                {!status.linux?.returned && (
                  <label className="offline-checkbox">
                    <input
                      type="checkbox"
                      checked={confirmed}
                      onChange={(event) => setConfirmed(event.target.checked)}
                    />
                    {tx('J’ai enregistré mon travail et je confirme le redémarrage de ce PC.')}
                  </label>
                )}
              </>
            ) : (
              <>
                <label className="offline-checkbox">
                  <input
                    type="checkbox"
                    checked={reopen}
                    onChange={(event) => setReopen(event.target.checked)}
                  />
                  {tx('Rouvrir Computy à la connexion pour retrouver le résultat')}
                </label>
                <label className="offline-checkbox">
                  <input
                    type="checkbox"
                    checked={confirmed}
                    disabled={blocked || !isDesktop()}
                    onChange={(event) => setConfirmed(event.target.checked)}
                  />
                  {tx(
                    'J’autorise Computy à préparer un seul démarrage de test. Cela ne redémarre pas le PC maintenant.',
                  )}
                </label>
                <button
                  className="button primary"
                  disabled={blocked || busy || !confirmed || !isDesktop()}
                  onClick={() => void launch()}
                >
                  {busy ? <Spinner size={16} /> : <ShieldCheck size={16} />}{' '}
                  {tx('Préparer le prochain démarrage')}
                </button>
              </>
            )}
          </>
        ) : linux ? (
          <>
            <ol className="offline-steps">
              <li>
                {tx(
                  'Branchez une clé USB déjà formatée en FAT32. Computy recherche uniquement les volumes USB montés et exclut le disque qui contient le système.',
                )}
              </li>
              <li>
                {tx(
                  'Choisissez la clé, vérifiez son nom et son emplacement, puis confirmez. Computy copie son moteur sans formater la clé ni remplacer un chargeur existant.',
                )}
              </li>
              <li>
                {tx(
                  'Après confirmation, redémarrez et choisissez la clé dans le menu de démarrage du firmware. La touche dépend du modèle du PC ; elle est souvent indiquée à l’allumage.',
                )}
              </li>
              <li>
                {tx(
                  'Le moteur affiche ses résultats. F6 peut enregistrer un rapport sur une clé FAT32 ; Échap quitte le test. Le retour dans Computy ne signifie pas automatiquement que le test a réussi.',
                )}
              </li>
            </ol>
            <button
              className="button secondary"
              disabled={busy || !isDesktop()}
              onClick={() =>
                void act(async () => {
                  setTargets(await invoke<UsbTarget[]>('usb_targets'));
                  setSelected('');
                  setConfirmed(false);
                })
              }
            >
              <RefreshCw size={16} />
              {tx('Rechercher les clés compatibles')}
            </button>
            <div className="usb-targets">
              {targets.map((target) => (
                <label key={target.id}>
                  <input
                    type="radio"
                    name="boot-usb"
                    value={target.id}
                    checked={selected === target.id}
                    onChange={() => {
                      setSelected(target.id);
                      setConfirmed(false);
                      setUsbReady(false);
                    }}
                  />
                  <span>
                    <b>{target.label}</b>
                    <small>
                      {target.mount} · {formatBytes(target.size_bytes)}
                    </small>
                  </span>
                </label>
              ))}
            </div>
            {!targets.length && (
              <p>
                {tx(
                  'Si aucune clé n’apparaît, montez une clé FAT32 ou utilisez le parcours sans clé. Computy ne formate aucun disque automatiquement.',
                )}
              </p>
            )}
            {selectedTarget && (
              <>
                <Notice>
                  {tx('Destination vérifiée')} : <b>{selectedTarget.label}</b> —{' '}
                  {selectedTarget.mount}
                  <p>
                    {tx(
                      'Fichier ajouté : EFI/BOOT/BOOTX64.EFI. Les autres fichiers restent en place. Une clé avec un chargeur existant est refusée.',
                    )}
                  </p>
                </Notice>
                <label className="offline-checkbox">
                  <input
                    type="checkbox"
                    checked={confirmed}
                    disabled={blocked}
                    onChange={(event) => setConfirmed(event.target.checked)}
                  />
                  {tx('Je confirme l’écriture du moteur Computy sur cette clé USB.')}
                </label>
                <button
                  className="button primary"
                  disabled={busy || blocked || !confirmed}
                  onClick={() =>
                    void act(async () => {
                      await invoke('prepare_usb', { id: selected });
                      setUsbReady(true);
                    })
                  }
                >
                  {busy ? <Spinner size={16} /> : <Usb size={16} />} {tx('Préparer cette clé USB')}
                </button>
              </>
            )}
            {usbReady && (
              <Notice>
                {tx(
                  'Clé préparée. Éjectez-la proprement, puis choisissez-la dans le menu de démarrage de votre PC. Aucun redémarrage n’a été déclenché.',
                )}
              </Notice>
            )}
          </>
        ) : (
          <Notice>
            {tx(
              'Sur Mac, Apple Diagnostics doit être lancé avec les commandes de démarrage du modèle. Cette version ne programme pas ce parcours et n’importe pas les résultats Apple.',
            )}
          </Notice>
        )}
        <p className="offline-caveat">
          {tx(
            'Moteur avant démarrage intégré : dérivé de Memtest86+ 9.00, sous GPLv2. Les sources correspondantes sont fournies avec le paquet Computy.',
          )}
        </p>
        {!isDesktop() && (
          <Notice>
            {tx(
              'Aperçu de développement : préparez le démarrage depuis l’application Computy installée.',
            )}
          </Notice>
        )}
      </div>
    </section>
  );
}
