import { useEffect, useState } from 'react';
import {
  ArrowRight,
  BrushCleaning,
  CalendarClock,
  Check,
  Folder,
  Search,
  ShieldCheck,
  Trash2,
} from 'lucide-react';
import { rpc } from '../api';
import { useApp } from '../app-context';
import { t, tx } from '../i18n.mjs';
import { formatBytes, formatDate } from '../domain.mjs';
import { Modal, SectionTitle, Spinner } from '../components';
import RuleBuilder from './RuleBuilder';
import type { CleanPlan, CleanPolicy, CleanOutcome, CleanSchedule } from '../clean-types';
const lines = (text: string) =>
  text
    .split('\n')
    .map((s) => s.trim())
    .filter(Boolean);
export default function Clean() {
  const { notify } = useApp();
  const [policy, setPolicy] = useState<CleanPolicy>({
    roots: [],
    exclude: [],
    rule: { op: 'all', rules: [{ op: 'older_days', value: 7 }] },
    node_modules: false,
    docker: false,
    docker_volumes: false,
  });
  const [dockerAvailable, setDockerAvailable] = useState(false);
  const [plan, setPlan] = useState<CleanPlan | null>(null);
  const [excluded, setExcluded] = useState<number[]>([]);
  const [busy, setBusy] = useState('');
  const [modal, setModal] = useState<'delete' | 'schedule' | null>(null);
  const [outcome, setOutcome] = useState<CleanOutcome | null>(null);
  const [schedules, setSchedules] = useState<CleanSchedule[]>([]);
  const [hours, setHours] = useState(168);
  const [limit, setLimit] = useState(1024);
  const [visible, setVisible] = useState(100);
  const reloadSchedules = () =>
    rpc<CleanSchedule[]>('clean_schedules')
      .then(setSchedules)
      .catch((e) => notify(String(e), true));
  useEffect(() => {
    void rpc<{ roots: string[]; docker_available: boolean }>('clean_sources')
      .then((data) => {
        setPolicy((p) => (p.roots.length ? p : { ...p, roots: data.roots }));
        setDockerAvailable(data.docker_available);
      })
      .catch((e) => notify(String(e), true));
    void reloadSchedules();
  }, []);
  const update = (patch: Partial<CleanPolicy>) => {
    setPolicy((p) => ({ ...p, ...patch }));
    setPlan(null);
    setOutcome(null);
  };
  const preview = async () => {
    setBusy('preview');
    setPlan(null);
    setOutcome(null);
    try {
      const value = await rpc<CleanPlan>('clean_preview', policy);
      setPlan(value);
      setExcluded(value.docker.map((_, i) => value.files.length + i));
      setVisible(100);
      notify('Aperçu prêt. Aucun fichier supprimé.');
    } catch (e) {
      notify(String(e), true);
    } finally {
      setBusy('');
    }
  };
  const toggle = (id: number) =>
    setExcluded((values) =>
      values.includes(id) ? values.filter((i) => i !== id) : [...values, id],
    );
  const selectedBytes =
    plan?.files.reduce((n, f, i) => n + (excluded.includes(i) ? 0 : f.bytes), 0) ?? 0;
  const selectedCount = plan ? plan.files.length + plan.docker.length - excluded.length : 0;
  const commit = async () => {
    if (!plan) return;
    setBusy('execute');
    try {
      const result = await rpc<CleanOutcome>('clean_execute', {
        id: plan.id,
        excluded,
        confirmed: true,
      });
      setOutcome(result);
      setPlan(null);
      setModal(null);
      notify('Nettoyage terminé.');
    } catch (e) {
      notify(String(e), true);
      setPlan(null);
      setModal(null);
    } finally {
      setBusy('');
    }
  };
  const automate = async () => {
    if (!plan) return;
    setBusy('schedule');
    try {
      await rpc('clean_schedule', {
        id: plan.id,
        interval_hours: hours,
        max_bytes: Math.round(limit * 1048576),
        confirmed: true,
      });
      setModal(null);
      await reloadSchedules();
      notify('Automatisation enregistrée.');
    } catch (e) {
      notify(String(e), true);
    } finally {
      setBusy('');
    }
  };
  return (
    <div className="page-enter utility-page clean-page">
      <SectionTitle
        eyebrow={tx('ENTRETIEN')}
        title={tx('Faire de la place. Garder le contrôle.')}
        description={tx('Analysez, vérifiez la sélection, puis confirmez le nettoyage.')}
      />
      <div className="workflow-steps">
        <span className={!plan ? 'active' : ''}>
          <Search size={16} />
          01 · {tx('Analyser')}
        </span>
        <ArrowRight size={15} />
        <span className={plan ? 'active' : ''}>
          <ShieldCheck size={16} />
          02 · {tx('Vérifier')}
        </span>
        <ArrowRight size={15} />
        <span>
          <BrushCleaning size={16} />
          03 · {tx('Nettoyer')}
        </span>
      </div>
      <section className="panel utility-panel">
        <div className="utility-heading">
          <Folder size={22} />
          <div>
            <h2>{tx('Périmètre du nettoyage')}</h2>
            <p>{tx('Un chemin absolu par ligne. Les liens symboliques sont ignorés.')}</p>
          </div>
        </div>
        <div className="utility-columns">
          <label>
            {tx('Dossiers inclus')}
            <textarea
              disabled={!!busy}
              rows={3}
              value={policy.roots.join('\n')}
              onChange={(e) => update({ roots: lines(e.target.value) })}
            />
          </label>
          <label>
            {tx('Dossiers exclus')}
            <textarea
              disabled={!!busy}
              rows={3}
              value={policy.exclude.join('\n')}
              onChange={(e) => update({ exclude: lines(e.target.value) })}
            />
          </label>
        </div>
        <label className="utility-check">
          <input
            type="checkbox"
            disabled={!!busy}
            checked={policy.node_modules}
            onChange={(e) => update({ node_modules: e.target.checked })}
          />
          <span>
            {tx('Uniquement les fichiers dans node_modules')}
            <small>{tx('Indiquez vos dossiers de projets. Le code source est conservé.')}</small>
          </span>
        </label>
        <div className="utility-heading compact">
          <h3>{tx('Conditions')}</h3>
          <small>{tx('Une date de création inconnue ne satisfait pas un critère de date.')}</small>
        </div>
        <fieldset disabled={!!busy}>
          <RuleBuilder rule={policy.rule} onChange={(rule) => update({ rule })} />
        </fieldset>
        <label className="utility-check">
          <input
            type="checkbox"
            disabled={!!busy || !dockerAvailable}
            checked={policy.docker}
            onChange={(e) => update({ docker: e.target.checked, docker_volumes: false })}
          />
          <span>
            {tx('Docker')}
            <small>
              {dockerAvailable
                ? tx('Images inutilisées et conteneurs arrêtés. Sélection séparée après analyse.')
                : tx('Docker local indisponible.')}
            </small>
          </span>
        </label>
        {policy.docker && (
          <label className="utility-check risk-option">
            <input
              type="checkbox"
              disabled={!!busy}
              checked={policy.docker_volumes}
              onChange={(e) => update({ docker_volumes: e.target.checked })}
            />
            <span>
              {tx('Inclure les volumes Docker inutilisés')}
              <small>
                {tx('Ils peuvent contenir des bases de données. Aucune sélection automatique.')}
              </small>
            </span>
          </label>
        )}
        <div className="utility-actions">
          <span>
            <ShieldCheck size={16} />
            {tx('Aperçu obligatoire avant toute suppression')}
          </span>
          <button
            className="button primary"
            disabled={!!busy || (!policy.roots.length && !policy.docker)}
            onClick={() => void preview()}
          >
            {busy === 'preview' ? <Spinner /> : <Search size={17} />}{' '}
            {tx(busy === 'preview' ? 'Analyse en cours…' : 'Analyser sans supprimer')}
          </button>
        </div>
        {busy === 'preview' && (
          <progress className="scan-progress" aria-label={tx('Analyse en cours…')} />
        )}
      </section>
      {plan && (
        <section className="panel utility-panel">
          <div className="clean-summary">
            <div>
              <span className="eyebrow">{tx('APERÇU · AUCUNE SUPPRESSION')}</span>
              <h2>{formatBytes(selectedBytes)}</h2>
              <p>{t('{{count}} éléments sélectionnés', { count: selectedCount })}</p>
            </div>
            <Check size={34} />
          </div>
          <p className="utility-hint">
            {tx(
              'Taille logique des fichiers. L’espace réellement disponible dépend du système de fichiers. Les couches Docker partagées ne sont pas additionnées.',
            )}
          </p>
          {plan.truncated && (
            <div className="utility-warning">
              {tx('Analyse limitée. Réduisez le périmètre avant de continuer.')}
            </div>
          )}
          {plan.warnings.map((w, i) => (
            <p className="utility-warning" key={i}>
              {w}
            </p>
          ))}
          <div className="clean-selection">
            <label>
              <input
                type="checkbox"
                checked={plan.files.length > 0 && plan.files.every((_, i) => !excluded.includes(i))}
                onChange={(e) =>
                  setExcluded(
                    e.target.checked
                      ? plan.docker.map((_, i) => plan.files.length + i)
                      : [...plan.files, ...plan.docker].map((_, i) => i),
                  )
                }
              />
              {tx('Sélectionner les fichiers')}
            </label>
            <span>{tx('Docker nécessite une sélection explicite.')}</span>
          </div>
          <div className="utility-table-wrap">
            <table className="utility-table">
              <thead>
                <tr>
                  <th>{tx('Fichier / ressource')}</th>
                  <th>{tx('Type')}</th>
                  <th>{tx('Taille')}</th>
                </tr>
              </thead>
              <tbody>
                {plan.files.slice(0, visible).map((file, i) => (
                  <tr key={file.path}>
                    <td>
                      <label>
                        <input
                          type="checkbox"
                          checked={!excluded.includes(i)}
                          onChange={() => toggle(i)}
                        />
                        <span title={file.path}>{file.path}</span>
                      </label>
                    </td>
                    <td>{file.mime}</td>
                    <td>{formatBytes(file.bytes)}</td>
                  </tr>
                ))}
                {plan.docker.map((item, i) => (
                  <tr key={item.kind + item.id}>
                    <td>
                      <label>
                        <input
                          type="checkbox"
                          checked={!excluded.includes(plan.files.length + i)}
                          onChange={() => toggle(plan.files.length + i)}
                        />
                        <span title={item.id}>{item.label}</span>
                      </label>
                    </td>
                    <td>Docker · {item.kind}</td>
                    <td>{item.bytes ? `≈ ${formatBytes(item.bytes)}` : tx('Inconnue')}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {visible < plan.files.length && (
            <button className="button secondary small" onClick={() => setVisible((n) => n + 100)}>
              {tx('Voir plus')}
            </button>
          )}
          <div className="utility-actions">
            <button
              className="button secondary"
              disabled={!!busy || plan.truncated || policy.docker}
              onClick={() => setModal('schedule')}
            >
              <CalendarClock size={17} />
              {tx('Automatiser cette règle')}
            </button>
            <button
              className="button primary"
              disabled={!!busy || !selectedCount || plan.truncated}
              onClick={() => setModal('delete')}
            >
              <Trash2 size={17} />
              {tx('Vérifier et nettoyer')}
            </button>
          </div>
          {policy.docker && (
            <p className="utility-hint">
              {tx('Les ressources Docker restent en nettoyage manuel.')}
            </p>
          )}
        </section>
      )}
      {outcome && (
        <section className="panel utility-panel">
          <div className="utility-heading">
            <Check size={25} />
            <div>
              <h2>{tx('Nettoyage terminé.')}</h2>
              <p>
                {formatBytes(outcome.deleted_bytes)} ·{' '}
                {t('{{count}} éléments supprimés', { count: outcome.deleted })} ·{' '}
                {t('{{count}} ignorés', { count: outcome.skipped })}
              </p>
            </div>
          </div>
          {outcome.errors.map((e, i) => (
            <p className="utility-warning" key={i}>
              {e}
            </p>
          ))}
        </section>
      )}
      <section className="panel utility-panel">
        <div className="utility-heading">
          <CalendarClock size={23} />
          <div>
            <h2>{tx('Automatisations')}</h2>
            <p>
              {tx(
                'Exécutées quand Computy fonctionne, y compris en arrière-plan. Les échéances manquées sont reprises à la prochaine connexion.',
              )}
            </p>
          </div>
        </div>
        {!schedules.length && (
          <p className="utility-hint">
            {tx('Créez une automatisation à partir d’un aperçu vérifié.')}
          </p>
        )}
        {schedules.map((task) => (
          <article className="schedule-row" key={task.id}>
            <div>
              <b>{t('Toutes les {{hours}} heures', { hours: task.interval_hours })}</b>
              <p>{task.policy.roots.join(' · ')}</p>
              <small>
                {tx('Prochaine exécution')} · {formatDate(task.next_run)} ·{' '}
                {formatBytes(task.max_bytes)}
              </small>
              {task.last_result && (
                <p>
                  {formatBytes(task.last_result.deleted_bytes)} ·{' '}
                  {task.last_result.errors.join(' · ')}
                </p>
              )}
            </div>
            <button
              className="icon-button"
              aria-label={tx('Supprimer')}
              onClick={() =>
                void rpc('clean_unschedule', { id: task.id })
                  .then(reloadSchedules)
                  .catch((e) => notify(String(e), true))
              }
            >
              <Trash2 size={17} />
            </button>
          </article>
        ))}
      </section>
      {modal && plan && (
        <Modal
          title={tx(modal === 'delete' ? 'Confirmer le nettoyage' : 'Automatiser cette règle')}
          onClose={() => {
            if (!busy) setModal(null);
          }}
        >
          <div className="utility-modal">
            {modal === 'delete' ? (
              <>
                <h2>{formatBytes(selectedBytes)}</h2>
                <p>{t('{{count}} éléments sélectionnés', { count: selectedCount })}</p>
                <p>
                  {tx(
                    'Suppression définitive. Les fichiers modifiés depuis l’aperçu seront ignorés.',
                  )}
                </p>
                <button className="button primary" disabled={!!busy} onClick={() => void commit()}>
                  {busy && <Spinner />}
                  <Trash2 size={16} />
                  {tx('Confirmer la suppression')}
                </button>
              </>
            ) : (
              <>
                <p>
                  {tx(
                    'Autoriser les prochains nettoyages de ces dossiers avec les mêmes conditions. Chaque passage refait une analyse et respecte la limite ci-dessous. La sélection de fichiers de cet aperçu ne limite pas les passages futurs.',
                  )}
                </p>
                <p className="utility-hint">{plan.policy.roots.join('\n')}</p>
                <label>
                  {tx('Intervalle (heures)')}
                  <input
                    type="number"
                    min={1}
                    max={8760}
                    value={hours}
                    onChange={(e) => setHours(Number(e.target.value))}
                  />
                </label>
                <label>
                  {tx('Limite par passage (Mio)')}
                  <input
                    type="number"
                    min={1}
                    value={limit}
                    onChange={(e) => setLimit(Number(e.target.value))}
                  />
                </label>
                <button
                  className="button primary"
                  disabled={!!busy || hours < 1 || hours > 8760 || limit < 1}
                  onClick={() => void automate()}
                >
                  {busy && <Spinner />}
                  {tx('Autoriser cette automatisation')}
                </button>
              </>
            )}
          </div>
        </Modal>
      )}
    </div>
  );
}
