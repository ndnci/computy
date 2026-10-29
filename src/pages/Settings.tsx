import { t, tx, currentLocale, languages } from '../i18n.mjs';
import { useEffect, useState } from 'react';
import {
  Check,
  HardDrive,
  Laptop,
  LockKeyhole,
  Moon,
  RotateCcw,
  Save,
  ShieldCheck,
  Sun,
} from 'lucide-react';
import { useApp } from '../app-context';
import { defaultPreferences, isDesktop, setStartup, startupStatus } from '../api';
import { Notice, Panel, PanelHeader, SectionTitle, Spinner } from '../components';
import MonitorSettings from './MonitorSettings';
import MaintenanceSettings from './MaintenanceSettings';
export default function SettingsPage() {
  const { preferences, savePreferences, notify } = useApp();
  const [draft, setDraft] = useState({
    ...preferences,
  });
  const [startup, setStartupValue] = useState(false);
  const [startupBusy, setStartupBusy] = useState(false);
  useEffect(() => setDraft({ ...preferences }), [preferences]);
  useEffect(() => {
    if (isDesktop())
      void startupStatus()
        .then(setStartupValue)
        .catch((e) => notify(e.message, true));
  }, [notify]);
  const toggleStartup = async () => {
    setStartupBusy(true);
    try {
      await setStartup(!startup);
      setStartupValue(!startup);
      notify(!startup ? 'Computy s’ouvrira à la connexion.' : 'Démarrage automatique désactivé.');
    } catch (e) {
      notify((e as Error).message, true);
    } finally {
      setStartupBusy(false);
    }
  };
  return (
    <div className="page-enter settings-page">
      <SectionTitle
        eyebrow={tx('À VOTRE FAÇON')}
        title={tx('Un espace qui vous ressemble.')}
        description={tx(
          'Personnalisez l’apparence et les paramètres de vos prochains diagnostics.',
        )}
      />
      <section className="panel">
        <PanelHeader
          title={tx('Langue de l’interface')}
          subtitle={tx(
            'Les traductions sont incluses dans l’application. Aucun service de traduction n’est appelé.',
          )}
        />
        <div className="language-settings">
          <label htmlFor="interface-language">{tx('Langue préférée')}</label>
          <select
            id="interface-language"
            value={draft.language}
            onChange={(event) => setDraft({ ...draft, language: event.target.value })}
          >
            <option value="system">{tx('Suivre le système')}</option>
            {languages.map((language) => (
              <option key={language.code} value={language.code} lang={language.code}>
                {language.name}
              </option>
            ))}
          </select>
          <p>
            {tx(
              '25 langues · dates et nombres localisés · langues de droite à gauche prises en charge',
            )}
          </p>
        </div>
      </section>
      <section className="panel">
        <PanelHeader
          title={tx('Apparence')}
          subtitle={tx('Le même espace, selon votre préférence.')}
        />
        <div className="theme-options">
          <button
            className={`theme-option ${draft.theme === 'system' ? 'selected' : ''}`}
            onClick={() => setDraft({ ...draft, theme: 'system' })}
          >
            <div className="theme-preview system-preview">
              <i />
              <span />
              <span />
              <span />
            </div>
            <span>
              <Laptop size={17} />
              {tx('Système')}
              {draft.theme === 'system' && <Check size={15} />}
            </span>
            <small>{tx('S’adapte automatiquement')}</small>
          </button>
          <button
            className={`theme-option ${draft.theme === 'light' ? 'selected' : ''}`}
            onClick={() =>
              setDraft({
                ...draft,
                theme: 'light',
              })
            }
          >
            <div className="theme-preview light-preview">
              <i />
              <span />
              <span />
              <span />
            </div>
            <span>
              <Sun size={17} />
              {tx('Clair')}
              {tx(draft.theme === 'light' && <Check size={15} />)}
            </span>
          </button>
          <button
            className={`theme-option ${draft.theme === 'dark' ? 'selected' : ''}`}
            onClick={() =>
              setDraft({
                ...draft,
                theme: 'dark',
              })
            }
          >
            <div className="theme-preview dark-preview">
              <i />
              <span />
              <span />
              <span />
            </div>
            <span>
              <Moon size={17} />
              {tx('Sombre')}
              {tx(draft.theme === 'dark' && <Check size={15} />)}
            </span>
          </button>
        </div>
      </section>
      <section className="panel">
        <PanelHeader
          title={tx('Votre profil « L’essentiel »')}
          subtitle={tx('Ces valeurs seront proposées lors d’un nouveau diagnostic.')}
        />
        <div className="fields-grid settings-fields">
          <label>
            {tx('M\xE9moire \xE0 allouer')}
            <select
              value={draft.memory_mib}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  memory_mib: Number(e.target.value),
                })
              }
            >
              {tx(
                [16, 32, 64, 128, 256, 512, 1024, 2048].map((v) => (
                  <option key={v} value={v}>
                    {tx(v)}
                    {tx(' Mio')}
                  </option>
                )),
              )}
            </select>
          </label>
          <label>
            {tx('Passes m\xE9moire')}
            <select
              value={draft.passes}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  passes: Number(e.target.value),
                })
              }
            >
              {tx(
                [1, 2, 3, 4, 6, 8].map((v) => (
                  <option key={v} value={v}>
                    {tx(v)}
                  </option>
                )),
              )}
            </select>
          </label>
          <label>
            {tx('Fichier temporaire')}
            <select
              value={draft.disk_mib}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  disk_mib: Number(e.target.value),
                })
              }
            >
              {tx(
                [16, 32, 64, 128, 256, 512].map((v) => (
                  <option key={v} value={v}>
                    {tx(v)}
                    {tx(' Mio')}
                  </option>
                )),
              )}
            </select>
          </label>
          <label>
            {tx('Calculs processeur')}
            <select
              value={draft.cpu_seconds}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  cpu_seconds: Number(e.target.value),
                })
              }
            >
              {tx(
                [1, 2, 5, 10, 15, 30, 60].map((v) => (
                  <option key={v} value={v}>
                    {tx(v)}
                    {tx(' secondes')}
                  </option>
                )),
              )}
            </select>
          </label>
        </div>
        <Notice>
          {tx(
            'Le moteur garde une r\xE9serve de m\xE9moire et d\u2019espace disque. Si l\u2019allocation est trop \xE9lev\xE9e pour les ressources disponibles, le contr\xF4le est refus\xE9 plut\xF4t que de saturer votre PC.',
          )}
        </Notice>
      </section>
      <section className="panel">
        <PanelHeader
          title={tx('Au retour dans votre session')}
          subtitle={tx('Retrouvez Computy après un test au démarrage.')}
        />
        <div className="setting-row">
          <span className="setting-icon">
            <Laptop size={22} />
          </span>
          <div>
            <b>{tx('Ouvrir Computy \xE0 la connexion')}</b>
            <p>
              {tx(
                'Rouvre Computy à la connexion. La recherche des résultats de tests au démarrage est automatique à chaque ouverture.',
              )}
            </p>
            {tx(
              !isDesktop() && (
                <small>{tx('Disponible dans l\u2019application Tauri install\xE9e.')}</small>
              ),
            )}
          </div>
          <button
            role="switch"
            aria-checked={startup}
            aria-label={tx('Ouvrir Computy à la connexion')}
            className={`toggle ${startup ? 'on' : ''}`}
            disabled={!isDesktop() || startupBusy}
            onClick={() => void toggleStartup()}
          >
            {tx(startupBusy ? <Spinner size={13} /> : <span />)}
          </button>
        </div>
      </section>
      <MonitorSettings draft={draft} onChange={setDraft} />
      <Panel className="privacy-settings">
        <LockKeyhole size={24} />
        <div>
          <h3>{tx('Vos donn\xE9es vous appartiennent.')}</h3>
          <p>
            {tx(
              'L\u2019historique et les pr\xE9f\xE9rences restent sur cet appareil. Aucun compte, aucune t\xE9l\xE9m\xE9trie et aucun envoi automatique. L\u2019export JSON vous permet de garder une copie lisible de chaque rapport.',
            )}
          </p>
        </div>
      </Panel>
      <div className="settings-actions">
        <button
          className="button secondary"
          onClick={() =>
            setDraft({
              ...defaultPreferences,
            })
          }
        >
          <RotateCcw size={16} />
          {tx('Valeurs par d\xE9faut')}
        </button>
        <button className="button primary" onClick={() => savePreferences(draft)}>
          <Save size={16} />
          {tx('Enregistrer les pr\xE9f\xE9rences')}
        </button>
      </div>
      <MaintenanceSettings />
    </div>
  );
}
