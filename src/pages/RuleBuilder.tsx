import { Plus, Trash2 } from 'lucide-react';
import { tx } from '../i18n.mjs';
import type { Rule } from '../clean-types';
const fields = [
  ['older_days', 'Ancienneté (jours)'],
  ['name', 'Nom exact'],
  ['extension', 'Extension'],
  ['path_regex', 'Chemin (regex)'],
  ['mime', 'Type MIME'],
  ['min_size', 'Taille minimum (octets)'],
  ['max_size', 'Taille maximum (octets)'],
  ['modified_before', 'Modifié avant'],
  ['modified_after', 'Modifié après'],
  ['created_before', 'Créé avant'],
  ['created_after', 'Créé après'],
];
export default function RuleBuilder({
  rule,
  onChange,
  depth = 0,
}: {
  rule: Rule;
  onChange: (rule: Rule) => void;
  depth?: number;
}) {
  if ('rules' in rule)
    return (
      <div className="rule-group">
        <div className="rule-group-heading">
          <select
            aria-label={tx('Conditions')}
            value={rule.op}
            onChange={(e) => onChange({ ...rule, op: e.target.value as 'all' | 'any' })}
          >
            <option value="all">{tx('Toutes les conditions (ET)')}</option>
            <option value="any">{tx('Une condition au moins (OU)')}</option>
          </select>
          <button
            className="button secondary small"
            disabled={rule.rules.length >= 16}
            onClick={() =>
              onChange({ ...rule, rules: [...rule.rules, { op: 'extension', value: 'log' }] })
            }
          >
            <Plus size={14} />
            {tx('Condition')}
          </button>
          {depth < 6 && (
            <button
              className="button secondary small"
              onClick={() =>
                onChange({
                  ...rule,
                  rules: [...rule.rules, { op: 'any', rules: [{ op: 'name', value: '' }] }],
                })
              }
            >
              <Plus size={14} />
              {tx('Groupe')}
            </button>
          )}
        </div>
        {rule.rules.map((child, index) => (
          <div className="rule-child" key={index}>
            <RuleBuilder
              rule={child}
              depth={depth + 1}
              onChange={(value) =>
                onChange({ ...rule, rules: rule.rules.map((r, i) => (i === index ? value : r)) })
              }
            />
            <button
              className="icon-button"
              aria-label={tx('Supprimer')}
              onClick={() => onChange({ ...rule, rules: rule.rules.filter((_, i) => i !== index) })}
            >
              <Trash2 size={15} />
            </button>
          </div>
        ))}
      </div>
    );
  if ('rule' in rule)
    return (
      <div className="rule-negated">
        <b>{tx('Exclure si')}</b>
        <RuleBuilder
          rule={rule.rule}
          depth={depth + 1}
          onChange={(r) => onChange({ op: 'not', rule: r })}
        />
        <button className="button text small" onClick={() => onChange(rule.rule)}>
          {tx('Annuler')}
        </button>
      </div>
    );
  const date = /^(modified|created)_/.test(rule.op);
  const numeric = /size|days/.test(rule.op);
  return (
    <div className="rule-leaf">
      <select
        aria-label={tx('Critère')}
        value={rule.op}
        onChange={(e) =>
          onChange({
            op: e.target.value,
            value: /size|days|before|after/.test(e.target.value) ? 0 : '',
          })
        }
      >
        {fields.map(([id, label]) => (
          <option key={id} value={id}>
            {tx(label)}
          </option>
        ))}
      </select>
      <input
        aria-label={tx('Valeur')}
        type={date ? 'datetime-local' : numeric ? 'number' : 'text'}
        min={numeric ? 0 : undefined}
        value={
          date
            ? Number(rule.value)
              ? new Date(Number(rule.value) - new Date().getTimezoneOffset() * 60000)
                  .toISOString()
                  .slice(0, 16)
              : ''
            : rule.value
        }
        onChange={(e) =>
          onChange({
            ...rule,
            value: date
              ? new Date(e.target.value).getTime()
              : numeric
                ? Number(e.target.value)
                : e.target.value,
          })
        }
      />
      <button className="button text small" onClick={() => onChange({ op: 'not', rule })}>
        {tx('Exclure si')}
      </button>
    </div>
  );
}
