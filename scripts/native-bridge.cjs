const { spawn } = require('node:child_process');
const { existsSync } = require('node:fs');
const path = require('node:path');
const { createInterface } = require('node:readline');
const METHODS = new Set([
  'inventory',
  'monitor',
  'history',
  'report',
  'start',
  'job',
  'active',
  'cancel',
  'rename',
  'delete',
  'offline_status',
  'offline_collect',
  'offline_forget',
  'preferences',
  'save_preferences',
  'compare',
]);
class NativeBridge {
  constructor(options = {}) {
    this.options = options;
    this.pending = new Map();
    this.sequence = 0;
    this.child = null;
    this.closed = false;
  }
  start() {
    if (this.closed) throw new Error('Le moteur est arrêté.');
    if (this.child) return;
    const root = path.resolve(__dirname, '..');
    const binary = process.platform === 'win32' ? 'computy-cli.exe' : 'computy-cli';
    const candidates =
      this.options.binary || process.env.COMPUTY_ENGINE_PATH
        ? [this.options.binary || process.env.COMPUTY_ENGINE_PATH]
        : [
            path.join(root, 'native', 'target', 'release', binary),
            path.join(root, 'native', 'target', 'debug', binary),
          ];
    const executable = candidates.find(existsSync);
    if (!executable)
      throw new Error(
        'Moteur Rust introuvable. Exécutez « npm run build:engine » puis relancez Computy.',
      );
    const dataDir =
      this.options.dataDir || process.env.COMPUTY_DATA_DIR || path.join(root, '.computy');
    this.child = spawn(executable, ['--data-dir', dataDir, 'rpc'], {
      stdio: ['pipe', 'pipe', 'pipe'],
      windowsHide: true,
    });
    const child = this.child;
    const lines = createInterface({ input: child.stdout });
    lines.on('line', (line) => {
      try {
        const response = JSON.parse(line);
        const request = this.pending.get(response.id);
        if (!request) return;
        clearTimeout(request.timeout);
        this.pending.delete(response.id);
        response.error
          ? request.reject(new Error(response.error))
          : request.resolve(response.result);
      } catch {
        this.fail(new Error('Réponse du moteur invalide.'));
      }
    });
    child.stderr.on('data', (data) => {
      if (process.env.COMPUTY_DEBUG) process.stderr.write(data);
    });
    child.on('error', (error) => this.fail(new Error(`Moteur inaccessible : ${error.message}`)));
    child.on('exit', () => {
      if (this.child === child) this.child = null;
      this.fail(new Error('Le moteur natif s’est arrêté.'));
    });
  }
  fail(error) {
    for (const request of this.pending.values()) {
      clearTimeout(request.timeout);
      request.reject(error);
    }
    this.pending.clear();
  }
  call(method, params = {}) {
    if (!METHODS.has(method)) return Promise.reject(new Error('Commande non autorisée.'));
    try {
      this.start();
    } catch (error) {
      return Promise.reject(error);
    }
    return new Promise((resolve, reject) => {
      const id = String(++this.sequence);
      const timeout = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error('Le moteur n’a pas répondu à temps.'));
      }, 60000);
      this.pending.set(id, { resolve, reject, timeout });
      this.child.stdin.write(JSON.stringify({ id, method, params }) + '\n', (error) => {
        if (error) {
          clearTimeout(timeout);
          this.pending.delete(id);
          reject(error);
        }
      });
    });
  }
  close() {
    this.closed = true;
    this.child?.stdin.end();
    this.fail(new Error('Computy se ferme.'));
  }
}
module.exports = { NativeBridge, METHODS };
