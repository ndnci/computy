import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { NativeBridge, METHODS } = require('./scripts/native-bridge.cjs');

function nativeApi() {
  let bridge;
  return {
    name: 'computy-local-native-api',
    configureServer(server) {
      bridge = new NativeBridge();
      server.httpServer?.once('close', () => bridge.close());
      server.middlewares.use('/api/computy', async (req, res) => {
        res.setHeader('Content-Type', 'application/json');
        res.setHeader('Cache-Control', 'no-store');
        try {
          if (
            req.method !== 'POST' ||
            req.headers['x-computy-client'] !== 'local-ui' ||
            !req.headers['content-type']?.startsWith('application/json')
          )
            throw new Error('Requête refusée.');
          if (req.headers.origin && new URL(req.headers.origin).host !== req.headers.host)
            throw new Error('Origine refusée.');
          if (req.headers['sec-fetch-site'] === 'cross-site') throw new Error('Origine refusée.');
          const chunks = [];
          let size = 0;
          for await (const chunk of req) {
            size += chunk.length;
            if (size > 262144) throw new Error('Requête trop volumineuse.');
            chunks.push(chunk);
          }
          const { method, params = {} } = JSON.parse(Buffer.concat(chunks).toString());
          if (method !== 'clean_execute' && size > 16384)
            throw new Error('Requête trop volumineuse.');
          if (!METHODS.has(method)) throw new Error('Commande refusée.');
          const result = await bridge.call(method, params);
          res.end(JSON.stringify({ result }));
        } catch (error) {
          res.statusCode = 400;
          res.end(JSON.stringify({ error: error.message }));
        }
      });
    },
  };
}
export default defineConfig({
  plugins: [react(), nativeApi()],
  base: './',
  server: { host: '127.0.0.1', port: 1420, strictPort: true },
});
