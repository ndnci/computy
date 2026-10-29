import { t, tx, currentLocale } from '../i18n.mjs';
import { useState } from 'react';
import {
  Activity,
  ArrowDownToLine,
  ArrowLeftRight,
  ArrowRight,
  Check,
  ChevronRight,
  Clock3,
  History,
  Pencil,
  Plus,
  RotateCcw,
  Search,
  SlidersHorizontal,
  Trash2,
  X,
} from 'lucide-react';
import { useApp } from '../app-context';
import { rpc } from '../api';
import {
  Empty,
  Modal,
  Notice,
  PanelHeader,
  Score,
  SectionTitle,
  Spinner,
  StatusBadge,
} from '../components';
import { duration, formatDate } from '../domain.mjs';
import type { Report } from '../types';
export default function HistoryPage() {
  const {
    reports,
    unreadable,
    navigate,
    openReport,
    exportReport,
    selectedIds,
    toggleSelection,
    setSelectedIds,
    reloadHistory,
    notify,
    relaunch,
    launching,
    job,
  } = useApp();
  const [query, setQuery] = useState('');
  const [filter, setFilter] = useState('all');
  const [rename, setRename] = useState<Report | null>(null);
  const [name, setName] = useState('');
  const [deleting, setDeleting] = useState<Report | null>(null);
  const [busy, setBusy] = useState(false);
  const filtered = reports.filter(
    (report) =>
      (!query ||
        `${report.label} ${report.inventory.host_name} ${formatDate(report.created_at)}`
          .toLowerCase()
          .includes(query.toLowerCase())) &&
      (filter === 'all' || report.summary.status === filter),
  );
  const renameReport = async () => {
    if (!rename || !name.trim()) return;
    setBusy(true);
    try {
      await rpc('rename', {
        id: rename.id,
        label: name.trim(),
      });
      await reloadHistory();
      setRename(null);
      notify('Diagnostic renommé.');
    } catch (e) {
      notify((e as Error).message, true);
    } finally {
      setBusy(false);
    }
  };
  const deleteReport = async () => {
    if (!deleting) return;
    setBusy(true);
    try {
      await rpc('delete', {
        id: deleting.id,
      });
      setSelectedIds(selectedIds.filter((id) => id !== deleting.id));
      await reloadHistory();
      setDeleting(null);
      notify('Diagnostic supprimé de cet appareil.');
    } catch (e) {
      notify((e as Error).message, true);
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="page-enter">
      <SectionTitle
        eyebrow={tx('LES REPÈRES QUI COMPTENT')}
        title={tx('L’histoire de votre matériel.')}
        description={tx('Retrouvez chaque bilan, chaque configuration et ce qui a changé.')}
        action={
          <button className="button primary" onClick={() => navigate('diagnostics')}>
            <Plus size={17} />
            {tx('Nouveau diagnostic')}
          </button>
        }
      />
      {tx(
        unreadable > 0 && (
          <Notice tone="warn">
            {tx(unreadable)}
            {tx(
              ' fichier(s) d\u2019historique illisible(s) ou alt\xE9r\xE9(s) ont \xE9t\xE9 \xE9cart\xE9s. Les fichiers originaux ont \xE9t\xE9 conserv\xE9s pour examen.',
            )}
          </Notice>
        ),
      )}
      <div className="history-stats">
        <div>
          <span className="stat-icon">
            <History size={20} />
          </span>
          <span>
            <b>{tx(reports.length)}</b>
            <small>{tx('Diagnostics enregistrés')}</small>
          </span>
        </div>
        <div>
          <span className="stat-icon">
            <Clock3 size={20} />
          </span>
          <span>
            <b>
              {tx(
                reports[0]
                  ? formatDate(reports[0].created_at, true)
                  : 'Votre premier bilan vous attend',
              )}
            </b>
            <small>{tx('dernier diagnostic')}</small>
          </span>
        </div>
        <div>
          <span className="stat-icon">
            <ArrowLeftRight size={20} />
          </span>
          <span>
            <b>{tx('Jusqu\u2019\xE0 4 bilans')}</b>
            <small>{tx('\xE0 comparer c\xF4te \xE0 c\xF4te')}</small>
          </span>
        </div>
      </div>
      <section className="panel history-panel">
        <div className="history-toolbar">
          <div className="search-input">
            <Search size={17} />
            <input
              aria-label={tx('Rechercher un diagnostic')}
              placeholder={tx('Rechercher un diagnostic…')}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
            {tx(
              query && (
                <button
                  className="icon-button"
                  aria-label={tx('Effacer la recherche')}
                  onClick={() => setQuery('')}
                >
                  <X size={14} />
                </button>
              ),
            )}
          </div>
          <div className="filter-select">
            <SlidersHorizontal size={15} />
            <select
              aria-label={tx('Filtrer les résultats')}
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
            >
              <option value="all">{tx('Tous les r\xE9sultats')}</option>
              <option value="passed">{tx('Conformes aux tests')}</option>
              <option value="warning">{tx('\xC0 examiner')}</option>
              <option value="failed">{tx('Anomalies d\xE9tect\xE9es')}</option>
              <option value="unavailable">{tx('Non v\xE9rifi\xE9s')}</option>
              <option value="cancelled">{tx('Interrompus')}</option>
            </select>
          </div>
          <span>
            {tx('Résultats')} : {tx(filtered.length)}
          </span>
        </div>
        {tx(
          reports.length === 0 ? (
            <Empty
              icon={History}
              title={tx('Le premier chapitre reste à écrire.')}
              action={
                <button className="button primary" onClick={() => navigate('diagnostics')}>
                  <Activity size={16} />
                  {tx('Cr\xE9er mon premier bilan')}
                </button>
              }
            >
              {tx(
                'Lancez un diagnostic pour conserver un instantan\xE9 du mat\xE9riel, des contr\xF4les et de leurs r\xE9sultats.',
              )}
            </Empty>
          ) : filtered.length === 0 ? (
            <Empty icon={Search} title={tx('Aucun diagnostic ne correspond.')}>
              {tx('Essayez un autre nom, une date ou un filtre diff\xE9rent.')}
            </Empty>
          ) : (
            <div className="table-scroll">
              <table className="history-table">
                <thead>
                  <tr>
                    <th aria-label={tx('Sélection')} />
                    <th>{tx('Diagnostic')}</th>
                    <th>{tx('Date et dur\xE9e')}</th>
                    <th>{tx('R\xE9sultat')}</th>
                    <th>{tx('Note des contr\xF4les')}</th>
                    <th aria-label={tx('Actions')} />
                  </tr>
                </thead>
                <tbody>
                  {tx(
                    filtered.map((report) => (
                      <tr
                        key={report.id}
                        className={`report-row ${selectedIds.includes(report.id) ? 'row-selected' : ''}`}
                        onClick={(event) => {
                          if (!(event.target as HTMLElement).closest('button,input,label,a'))
                            openReport(report);
                        }}
                      >
                        <td>
                          <label className="table-checkbox">
                            <input
                              type="checkbox"
                              checked={selectedIds.includes(report.id)}
                              onChange={() => toggleSelection(report.id)}
                              aria-label={tx(`Comparer ${report.label} (${report.id})`)}
                            />
                            <span>
                              <Check size={12} />
                            </span>
                          </label>
                        </td>
                        <td>
                          <button className="report-name" onClick={() => openReport(report)}>
                            <span className="recent-icon">
                              <Activity size={18} />
                            </span>
                            <span>
                              <b>{report.label}</b>
                              <small>
                                {tx(report.inventory.host_name)}
                                {tx(' \xB7 ')}
                                {tx(
                                  report.inventory.memory.modules
                                    .map((m) => m.kind)
                                    .filter((v, i, a) => a.indexOf(v) === i)
                                    .join(' / ') || report.inventory.os,
                                )}
                              </small>
                            </span>
                          </button>
                        </td>
                        <td>
                          <span className="table-date">
                            {tx(formatDate(report.created_at, true))}
                          </span>
                          <span className="table-secondary">
                            {tx(duration(report.duration_ms))}
                            {tx(' \xB7 ')}
                            {tx(report.summary.checked)}
                            {tx('/')}
                            {tx(report.summary.total)}
                            {tx(' contr\xF4les exploitables')}
                          </span>
                        </td>
                        <td>
                          <StatusBadge
                            status={report.summary.status}
                            checks={report.checks}
                            completed={report.completed}
                            compact
                          />
                        </td>
                        <td>
                          <Score score={report.summary.score} status={report.summary.status} />
                        </td>
                        <td>
                          <div className="row-actions">
                            <button
                              className="icon-button"
                              aria-label={t('Relancer le diagnostic : {{label}}', {
                                label: report.label,
                              })}
                              title={tx('Relancer')}
                              disabled={launching || !!job?.running}
                              onClick={() => void relaunch(report)}
                            >
                              <RotateCcw size={16} />
                            </button>
                            <button
                              className="icon-button"
                              aria-label={tx(`Exporter ${report.label}`)}
                              title={tx('Exporter en JSON')}
                              onClick={() => exportReport(report)}
                            >
                              <ArrowDownToLine size={16} />
                            </button>
                            <button
                              className="icon-button"
                              aria-label={tx(`Renommer ${report.label}`)}
                              title={tx('Renommer')}
                              onClick={() => {
                                setRename(report);
                                setName(report.label);
                              }}
                            >
                              <Pencil size={15} />
                            </button>
                            <button
                              className="icon-button danger"
                              aria-label={tx(`Supprimer ${report.label}`)}
                              title={tx('Supprimer')}
                              onClick={() => setDeleting(report)}
                            >
                              <Trash2 size={15} />
                            </button>
                            <button
                              className="icon-button"
                              aria-label={tx(`Ouvrir ${report.label}`)}
                              title={tx('Voir le rapport')}
                              onClick={() => openReport(report)}
                            >
                              <ChevronRight size={17} />
                            </button>
                          </div>
                        </td>
                      </tr>
                    )),
                  )}
                </tbody>
              </table>
            </div>
          ),
        )}
      </section>
      {tx(
        selectedIds.length > 0 ? (
          <div className="selection-bar">
            <span className="selection-count">{tx(selectedIds.length)}</span>
            <div>
              <b>{tx('Diagnostics sélectionnés')}</b>
              <p>
                {tx(
                  selectedIds.length < 2
                    ? 'Sélectionnez un second bilan pour comparer.'
                    : 'Le premier bilan sélectionné sera votre référence.',
                )}
              </p>
            </div>
            <button className="text-button" onClick={() => setSelectedIds([])}>
              {tx('Tout d\xE9s\xE9lectionner')}
            </button>
            <button
              className="button primary"
              disabled={selectedIds.length < 2}
              onClick={() => navigate('compare')}
            >
              <ArrowLeftRight size={16} />
              {tx('Comparer les bilans ')}
              <ArrowRight size={16} />
            </button>
          </div>
        ) : (
          <div className="history-hint">
            <ArrowLeftRight size={17} />
            <p>
              {tx(
                'Un changement de RAM ? S\xE9lectionnez les bilans avant et apr\xE8s pour comparer vos configurations.',
              )}
            </p>
          </div>
        ),
      )}
      {tx(
        rename && (
          <Modal
            title={tx('Renommer ce diagnostic')}
            onClose={() => {
              if (!busy) setRename(null);
            }}
          >
            <form
              onSubmit={(e) => {
                e.preventDefault();
                void renameReport();
              }}
            >
              <label className="field-label" htmlFor="report-name">
                {tx('Nom du diagnostic')}
              </label>
              <input
                id="report-name"
                className="text-input"
                value={name}
                onChange={(e) => setName(e.target.value)}
                maxLength={100}
              />
              <div className="modal-actions">
                <button
                  type="button"
                  className="button secondary"
                  disabled={busy}
                  onClick={() => setRename(null)}
                >
                  {tx('Annuler')}
                </button>
                <button className="button primary" type="submit" disabled={busy || !name.trim()}>
                  {tx(busy && <Spinner size={15} />)}
                  {tx('Enregistrer')}
                </button>
              </div>
            </form>
          </Modal>
        ),
      )}
      {tx(
        deleting && (
          <Modal
            title={tx('Supprimer ce diagnostic ?')}
            onClose={() => {
              if (!busy) setDeleting(null);
            }}
          >
            <p className="modal-text">
              {tx('Le bilan \xAB ')}
              {tx(deleting.label)}
              {tx(
                ' \xBB sera supprim\xE9 de cet ordinateur. Exportez-le avant de le supprimer si vous souhaitez en garder une copie.',
              )}
            </p>
            <div className="modal-actions">
              <button
                className="button secondary"
                disabled={busy}
                onClick={() => setDeleting(null)}
              >
                {tx('Conserver')}
              </button>
              <button
                className="button danger-button"
                disabled={busy}
                onClick={() => void deleteReport()}
              >
                {tx(busy ? <Spinner size={15} /> : <Trash2 size={15} />)}
                {tx('Supprimer le bilan')}
              </button>
            </div>
          </Modal>
        ),
      )}
    </div>
  );
}
