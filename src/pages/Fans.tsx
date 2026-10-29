import { useEffect, useState } from 'react';
import { Fan, ShieldCheck } from 'lucide-react';
import { rpc } from '../api';
import { tx } from '../i18n.mjs';
import { SectionTitle, Spinner } from '../components';
import { useApp } from '../app-context';
interface FanReading {
  id: string;
  name: string;
  driver: string;
  rpm: number | null;
  pwm_percent: number | null;
  mode: number | null;
  manual_supported: boolean;
}
export default function Fans() {
  const { notify } = useApp();
  const [fans, setFans] = useState<FanReading[] | null>(null);
  useEffect(() => {
    let stopped = false;
    const read = () =>
      rpc<{ fans: FanReading[] }>('fans')
        .then((result) => {
          if (!stopped) setFans(result.fans);
        })
        .catch((e) => {
          if (!stopped) {
            notify(String(e), true);
            setFans([]);
          }
        });
    void read();
    const timer = setInterval(() => void read(), 5000);
    return () => {
      stopped = true;
      clearInterval(timer);
    };
  }, []);
  return (
    <div className="page-enter utility-page">
      <SectionTitle
        eyebrow={tx('REFROIDISSEMENT')}
        title={tx('Ventilateurs')}
        description={tx('Vitesse mesurée et capacités exposées par le matériel.')}
      />
      <section className="panel utility-panel">
        <div className="utility-heading">
          <ShieldCheck size={24} />
          <div>
            <h2>{tx('Contrôle automatique conservé')}</h2>
            <p>
              {tx(
                'Le réglage manuel nécessite un adaptateur validé pour votre matériel et un retour automatique fiable. Aucun réglage de vitesse n’est exposé ici.',
              )}
            </p>
          </div>
        </div>
      </section>
      {fans === null ? (
        <div className="utility-empty">
          <Spinner />
        </div>
      ) : !fans.length ? (
        <section className="panel utility-panel utility-empty">
          <Fan size={38} />
          <h3>{tx('Aucun capteur de ventilation disponible')}</h3>
          <p>
            {tx(
              'Le firmware ou le système ne fournit pas de mesure exploitable. Cela ne signifie pas une panne.',
            )}
          </p>
        </section>
      ) : (
        <div className="application-grid">
          {fans.map((fan) => (
            <section className="panel utility-panel fan-card" key={fan.id}>
              <Fan size={29} />
              <span className="utility-tag">{fan.driver}</span>
              <h2>{fan.name}</h2>
              <strong>
                {fan.rpm === null ? '—' : new Intl.NumberFormat().format(fan.rpm)}{' '}
                <small>RPM</small>
              </strong>
              <p>
                {tx('Commande PWM')} ·{' '}
                {fan.pwm_percent === null ? tx('Indisponible') : `${Math.round(fan.pwm_percent)} %`}
              </p>
              <span className="utility-hint">{tx('Lecture seule')}</span>
            </section>
          ))}
        </div>
      )}
    </div>
  );
}
