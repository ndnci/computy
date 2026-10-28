import { t, tx, currentLocale } from '../i18n.mjs';
import {
  ArrowUpRight,
  Check,
  Code2,
  Cpu,
  Heart,
  LockKeyhole,
  MemoryStick,
  ShieldCheck,
} from 'lucide-react';
import { Logo, Notice, PanelHeader, SectionTitle } from '../components';
export default function About() {
  return (
    <div className="page-enter about-page">
      <SectionTitle
        eyebrow={tx('UNE AUTRE FAÇON DE PRENDRE SOIN DE SON PC')}
        title={tx('Moins de mystère. Plus de clarté.')}
        description={tx(
          'Computy rend les diagnostics matériels accessibles, sans cacher leurs limites.',
        )}
      />
      <section className="about-brand-panel">
        <Logo size={138} />
        <div>
          <span className="eyebrow">{tx('COMPUTY \xB7 VERSION 0.1.0')}</span>
          <h2>
            {tx('Comprendre aujourd\u2019hui.')}
            <br />
            {tx('Mieux choisir demain.')}
          </h2>
          <p>
            {tx(
              'Un logiciel de bureau pour observer votre mat\xE9riel, v\xE9rifier son comportement et garder une trace de ses \xE9volutions.',
            )}
          </p>
          <a
            className="button primary"
            href="https://www.ndnci.com"
            target="_blank"
            rel="noreferrer"
          >
            {tx('Une cr\xE9ation NDNCI ')}
            <ArrowUpRight size={16} />
          </a>
        </div>
      </section>
      <div className="about-values">
        <article>
          <Code2 size={23} />
          <h3>{tx('Un moteur natif en Rust')}</h3>
          <p>
            {tx(
              'Calculs, lecture du mat\xE9riel et diagnostics dans Rust. Une application Tauri utilise le moteur web de l\u2019OS, sans embarquer Chromium.',
            )}
          </p>
        </article>
        <article>
          <LockKeyhole size={23} />
          <h3>{tx('Local, par conception')}</h3>
          <p>
            {tx(
              'Les diagnostics et leur historique restent sur votre machine. Vous d\xE9cidez quand exporter un rapport ou ouvrir une ressource externe.',
            )}
          </p>
        </article>
        <article>
          <ShieldCheck size={23} />
          <h3>{tx('Des limites explicites')}</h3>
          <p>
            {tx(
              'Une donn\xE9e indisponible reste inconnue. Un contr\xF4le interrompu reste incomplet. Chaque r\xE9sultat conserve ses mesures et sa port\xE9e.',
            )}
          </p>
        </article>
      </div>
      <section className="panel methodology-panel">
        <PanelHeader
          title={tx('Notre méthode de diagnostic')}
          subtitle={tx('Ce qui est contrôlé, et ce qui ne peut pas être certifié.')}
        />
        <div className="method-row">
          <span>{tx('01')}</span>
          <div>
            <h3>{tx('M\xE9moire : \xE9crire, relire, comparer')}</h3>
            <p>
              {tx(
                'Le moteur alloue une zone born\xE9e et y \xE9crit plusieurs motifs : adresses virtuelles, z\xE9ros et uns, damier, s\xE9quences pseudo-al\xE9atoires et bits mobiles. Il v\xE9rifie puis inverse les mots dans les deux sens, avec des acc\xE8s volatils pour emp\xEAcher la suppression des op\xE9rations par le compilateur.',
              )}
            </p>
            <p>
              {tx(
                'Le verrouillage physique est tent\xE9 et son r\xE9sultat est enregistr\xE9. Les caches restent actifs ; sans verrouillage, des pages peuvent \xEAtre \xE9chang\xE9es sur le disque. Les compteurs ECC Linux sont lus lorsqu\u2019ils sont accessibles.',
              )}
            </p>
            <a
              href="https://github.com/memtest86plus/memtest86plus#memory-testing-philosophy"
              target="_blank"
              rel="noreferrer"
            >
              {tx('R\xE9f\xE9rence : philosophie et algorithmes de Memtest86+ ')}
              <ArrowUpRight size={13} />
            </a>
          </div>
        </div>
        <div className="method-row">
          <span>{tx('02')}</span>
          <div>
            <h3>{tx('Stockage : t\xE9l\xE9m\xE9trie et int\xE9grit\xE9')}</h3>
            <p>
              {tx(
                'Computy interroge smartctl en JSON et interpr\xE8te son code de retour comme un masque de bits. Les alertes NVMe, les erreurs de m\xE9dia, les secteurs en attente et les historiques disponibles sont examin\xE9s. Les disques en veille peuvent rester non interrog\xE9s.',
              )}
            </p>
            <p>
              {tx(
                'Un fichier d\xE9di\xE9, cr\xE9\xE9 sans \xE9crasement, est rempli de motifs diff\xE9rents par bloc, synchronis\xE9 puis relu et compar\xE9 octet par octet. Il est ensuite supprim\xE9. Ce contr\xF4le porte sur le volume des donn\xE9es de Computy et ne constitue pas un test de tous les secteurs.',
              )}
            </p>
            <a href="https://www.smartmontools.org/" target="_blank" rel="noreferrer">
              {tx('R\xE9f\xE9rence : smartmontools et smartctl ')}
              <ArrowUpRight size={13} />
            </a>
          </div>
        </div>
        <div className="method-row">
          <span>{tx('03')}</span>
          <div>
            <h3>{tx('Processeur : des r\xE9ponses connues')}</h3>
            <p>
              {tx(
                'Jusqu\u2019\xE0 quatre threads calculent des SHA-256 d\u2019un million de caract\xE8res \xAB a \xBB et des sommes enti\xE8res, compar\xE9s \xE0 des r\xE9ponses fixes connues. La dur\xE9e, le nombre de calculs et les divergences sont enregistr\xE9s. Ce contr\xF4le ne couvre pas toutes les instructions ni la stabilit\xE9 prolong\xE9e.',
              )}
            </p>
          </div>
        </div>
      </section>
      <section className="panel score-method">
        <PanelHeader title={tx('Une note lisible, jamais une garantie')} />
        <div className="score-rules">
          <div>
            <span className="rule-score good">
              {tx('10')}
              <small>{tx('/10')}</small>
            </span>
            <b>{tx('Conforme au contr\xF4le')}</b>
            <p>{tx('Aucune anomalie d\xE9tect\xE9e par le contr\xF4le effectu\xE9.')}</p>
          </div>
          <div>
            <span className="rule-score warn">
              {tx('5')}
              <small>{tx('/10')}</small>
            </span>
            <b>{tx('\xC0 examiner')}</b>
            <p>{tx('Usure ou historique d\u2019erreurs n\xE9cessitant un examen.')}</p>
          </div>
          <div>
            <span className="rule-score bad">
              {tx('0')}
              <small>{tx('/10')}</small>
            </span>
            <b>{tx('Anomalie d\xE9tect\xE9e')}</b>
            <p>{tx('Une erreur v\xE9rifi\xE9e ou une alerte mat\xE9rielle est pr\xE9sente.')}</p>
          </div>
          <div>
            <span className="rule-score muted">{tx('\u2014')}</span>
            <b>{tx('Sans note')}</b>
            <p>{tx('Contr\xF4le indisponible, absent ou interrompu.')}</p>
          </div>
        </div>
        <Notice>
          {tx(
            'La note du bilan reprend la note la plus basse des contr\xF4les exploitables. Les contr\xF4les indisponibles r\xE9duisent la couverture et restent visibles. Un diagnostic interrompu n\u2019obtient pas de note globale de r\xE9ussite.',
          )}
        </Notice>
      </section>
      <div className="about-limits">
        <MemoryStick size={24} />
        <div>
          <h3>{tx('Pourquoi aucun logiciel s\xE9rieux ne promet \xAB 100 % fiable \xBB')}</h3>
          <p>
            {tx(
              'Un test lanc\xE9 dans l\u2019OS ne peut pas v\xE9rifier la m\xE9moire occup\xE9e par le syst\xE8me. M\xEAme un test autonome peut manquer une panne intermittente, d\xE9pendante de la temp\xE9rature ou d\u2019une combinaison de composants. Un r\xE9sultat correct signifie \xAB aucune erreur observ\xE9e dans ces conditions \xBB.',
            )}
          </p>
          <a
            href="https://www.memtest86.com/tech_execution-time.html"
            target="_blank"
            rel="noreferrer"
          >
            {tx('Comprendre la dur\xE9e et les pannes intermittentes ')}
            <ArrowUpRight size={13} />
          </a>
        </div>
      </div>
      <p className="about-signature">
        {tx('Pens\xE9 pour vos machines. Con\xE7u avec soin par ')}
        <a href="https://www.ndnci.com" target="_blank" rel="noreferrer">
          {tx('NDNCI ')}
          <ArrowUpRight size={13} />
        </a>
      </p>
    </div>
  );
}
