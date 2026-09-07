import { dirname, relative, resolve, sep } from 'node:path';

// VitePress rewrote in-repo links like `./assets.md` or `../base/index.md` to
// their published route. Astro leaves them as written, so every one of the 400+
// cross-references in the docs shipped as a dead `.md` link. This maps a link's
// target file back to the route that renders it.
//
// The content lives in directories that mirror the routes — `docs/x.md` is
// `/docs/x`, `zh-CN/base/y.md` is `/zh-CN/base/y` — so the route is just the
// path relative to the site root, minus the extension, with `index` dropped.

const SITE_ROOT = resolve(process.cwd());

// Links to real files (images, downloads) keep their extension and are left
// to Astro's asset handling.
const ASSET = /\.(?:png|jpe?g|gif|svg|webp|avif|ico|pdf|zip|gz|txt|json|toml|ya?ml|rs|css|js|mjs|ts)$/i;

function isExternal(url) {
  return /^[a-z][a-z\d+.-]*:/i.test(url) || url.startsWith('//');
}

export function remarkDocLinks({ base = '/' } = {}) {
  const prefix = base.replace(/\/+$/, '');

  return (tree, file) => {
    const from = file?.path ?? file?.history?.[0];
    if (!from) return;
    const fromDir = dirname(from);

    const rewrite = (node) => {
      const url = node.url;
      if (!url || isExternal(url) || url.startsWith('#') || url.startsWith('/')) return;

      const hashAt = url.indexOf('#');
      const target = hashAt === -1 ? url : url.slice(0, hashAt);
      const hash = hashAt === -1 ? '' : url.slice(hashAt);
      if (!target || ASSET.test(target)) return;

      // Every relative link is resolved here rather than left to the browser.
      // The pages are served without a trailing slash, so a bare `accordion`
      // on `/docs/components` would otherwise resolve against `/docs/`, not
      // against the directory the source file sits in — which is the
      // relationship the author wrote.
      const withoutExt = target.endsWith('.md') ? target.slice(0, -'.md'.length) : target;
      const rel = relative(SITE_ROOT, resolve(fromDir, withoutExt));
      // A target outside the site directory has no route to point at.
      if (rel.startsWith('..')) return;

      const route = `/${rel.split(sep).join('/')}`.replace(/\/index$/, '');
      node.url = `${prefix}${route || '/'}${hash}`;
    };

    const walk = (node) => {
      if (node.type === 'link') rewrite(node);
      // `[text]: ./target.md` reference definitions carry a url too.
      if (node.type === 'definition') rewrite(node);
      for (const child of node.children ?? []) walk(child);
    };
    walk(tree);
  };
}
