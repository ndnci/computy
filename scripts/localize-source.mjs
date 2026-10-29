// One-time migration + repeatable extraction. Runtime translations are explicit React calls.
import fs from 'node:fs';
import path from 'node:path';
import { parse } from '@babel/parser';
import traverseModule from '@babel/traverse';
import generateModule from '@babel/generator';
import * as b from '@babel/types';
const traverse = traverseModule.default || traverseModule;
const generate = generateModule.default || generateModule;
const apply = process.argv.includes('--apply');
const keys = new Set();
const visibleAttrs = new Set([
  'title',
  'placeholder',
  'aria-label',
  'label',
  'description',
  'eyebrow',
  'subtitle',
  'alt',
]);
const human = (value) =>
  /[àâäéèêëîïôöùûüçœ’…]/i.test(value) ||
  (/^[A-ZÀ-Ÿ][\p{L}0-9'’()., :;!?%/—–·+-]+$/u.test(value) && !value.includes('://'));
function register(value) {
  const key = value.replace(/\s+/g, ' ').trim();
  if (key && key.length > 1) keys.add(key);
}
for (const file of [
  'src/App.tsx',
  'src/MonitorPopover.tsx',
  'src/monitor-components.tsx',
  'src/components.tsx',
  ...fs
    .readdirSync('src/pages')
    .filter((f) => f.endsWith('.tsx'))
    .map((f) => `src/pages/${f}`),
]) {
  const ast = parse(fs.readFileSync(file, 'utf8'), {
    sourceType: 'module',
    plugins: ['typescript', 'jsx'],
  });
  traverse(ast, {
    StringLiteral(p) {
      if (
        human(p.node.value) ||
        (p.parentPath.isCallExpression() &&
          b.isIdentifier(p.parentPath.node.callee) &&
          ['t', 'tx'].includes(p.parentPath.node.callee.name) &&
          p.parentPath.node.arguments[0] === p.node)
      )
        register(p.node.value);
    },
    TemplateLiteral(p) {
      if (
        !p.node.quasis.some((q) =>
          /[àâéèêëîïôùûüçœ]|disponible|module|libre|cœur|passe|contrôle|sélectionné|rapport|vérifié/.test(
            q.value.cooked || '',
          ),
        )
      )
        return;
      let key = '';
      const properties = [];
      p.node.quasis.forEach((q, i) => {
        key += q.value.cooked || '';
        if (i < p.node.expressions.length) {
          key += `{{v${i}}}`;
          properties.push(b.objectProperty(b.identifier(`v${i}`), p.node.expressions[i]));
        }
      });
      register(key);
      if (apply) {
        p.replaceWith(
          b.callExpression(b.identifier('t'), [
            b.stringLiteral(key),
            b.objectExpression(properties),
          ]),
        );
        p.skip();
      }
    },
    JSXText(p) {
      const raw = p.node.value;
      if (!raw.trim()) return;
      const value = raw.replace(/\s+/g, ' ');
      register(value);
      if (apply)
        p.replaceWith(
          b.jsxExpressionContainer(b.callExpression(b.identifier('tx'), [b.stringLiteral(value)])),
        );
    },
    JSXAttribute(p) {
      if (!visibleAttrs.has(p.node.name.name)) return;
      const value = p.node.value;
      if (b.isStringLiteral(value)) {
        register(value.value);
        if (apply)
          p.node.value = b.jsxExpressionContainer(b.callExpression(b.identifier('tx'), [value]));
      } else if (b.isJSXExpressionContainer(value) && !b.isJSXEmptyExpression(value.expression)) {
        if (apply && !b.isCallExpression(value.expression, { callee: b.identifier('tx') }))
          value.expression = b.callExpression(b.identifier('tx'), [value.expression]);
      }
    },
    JSXExpressionContainer: {
      exit(p) {
        if (!apply || !p.parentPath.isJSXElement() || b.isJSXEmptyExpression(p.node.expression))
          return;
        const expression = p.node.expression;
        if (
          b.isCallExpression(expression) &&
          b.isIdentifier(expression.callee) &&
          ['t', 'tx'].includes(expression.callee.name)
        )
          return;
        p.node.expression = b.callExpression(b.identifier('tx'), [expression]);
      },
    },
    CallExpression(p) {
      if (!apply) return;
      const callee = p.node.callee;
      if (
        b.isMemberExpression(callee) &&
        b.isIdentifier(callee.property, { name: 'toLocaleString' }) &&
        p.node.arguments.length > 0 &&
        b.isStringLiteral(p.node.arguments[0], { value: 'fr-FR' })
      )
        p.node.arguments[0] = b.callExpression(b.identifier('currentLocale'), []);
    },
  });
  if (apply) {
    const relative = file.startsWith('src/pages/') ? '../i18n.mjs' : './i18n.mjs';
    ast.program.body.unshift(
      b.importDeclaration(
        ['t', 'tx', 'currentLocale'].map((name) =>
          b.importSpecifier(b.identifier(name), b.identifier(name)),
        ),
        b.stringLiteral(relative),
      ),
    );
    fs.writeFileSync(file, generate(ast, { retainLines: false, comments: true }).code + '\n');
  }
}
for (const file of [
  'src/domain.mjs',
  ...fs
    .readdirSync('native/src/diagnostics')
    .filter((f) => f.endsWith('.rs'))
    .map((f) => `native/src/diagnostics/${f}`),
  'native/src/inventory.rs',
  'native/src/service.rs',
  'src-tauri/src/monitor_tray.rs',
]) {
  const text = fs.readFileSync(file, 'utf8');
  for (const match of text.matchAll(/"([^"\n]{3,})"|'([^'\n]{3,})'/g)) {
    const value = match[1] || match[2];
    if (human(value)) register(value);
  }
}
fs.mkdirSync('src/locales', { recursive: true });
fs.writeFileSync(
  'src/locales/source.json',
  JSON.stringify(Object.fromEntries([...keys].sort().map((key) => [key, key])), null, 2) + '\n',
);
console.log(`${keys.size} source messages extracted${apply ? ' and JSX migrated' : ''}.`);
