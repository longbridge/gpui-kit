import { createReadStream, existsSync, statSync } from 'node:fs';
import { extname, join, resolve } from 'node:path';

const CONTENT_TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.wasm': 'application/wasm',
  '.css': 'text/css; charset=utf-8',
  '.svg': 'image/svg+xml',
};

export function wasmExamplesDevServer(base) {
  const prefix = base.replace(/\/$/, '');
  const roots = new Map([
    [`${prefix}/examples/base`, resolve('../crates/base/examples/wasm/www/dist')],
    [`${prefix}/gallery`, resolve('../crates/story-web/www/dist')],
  ]);

  return {
    name: 'wasm-examples-dev-server',
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        const pathname = new URL(req.url ?? '/', 'http://localhost').pathname;
        const entry = [...roots].find(
          ([prefix]) => pathname === prefix || pathname.startsWith(`${prefix}/`)
        );
        if (!entry) return next();

        const [prefix, root] = entry;
        const relative = pathname.slice(prefix.length).replace(/^\/+/, '');
        let file = join(root, relative || 'index.html');
        if (!existsSync(file) || !statSync(file).isFile()) {
          file = join(root, 'index.html');
        }
        if (!existsSync(file)) {
          res.statusCode = 503;
          res.end('WASM example is not built. Run its Makefile build target first.');
          return;
        }

        res.setHeader('Cache-Control', 'no-store');
        res.setHeader(
          'Content-Type',
          CONTENT_TYPES[extname(file)] ?? 'application/octet-stream'
        );
        createReadStream(file).pipe(res);
      });
    },
  };
}
