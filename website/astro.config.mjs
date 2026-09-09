import { readdirSync } from 'node:fs';
import { defineConfig } from 'astro/config';
import vue from '@astrojs/vue';
import tailwindcss from '@tailwindcss/vite';
import remarkMath from 'remark-math';
import rehypeMathjax from 'rehype-mathjax';
import pagefind from 'astro-pagefind';
import { rehypeHeadingIds, unified } from '@astrojs/markdown-remark';
import { remarkCallouts } from './src/lib/remark-callouts.js';
import { remarkDocLinks } from './src/lib/remark-doc-links.js';
import { remarkSnippets } from './src/lib/remark-snippets.js';
import { rehypeHeadingAnchors } from './src/lib/rehype-heading-anchors.js';
import { wasmExamplesDevServer } from './src/lib/wasm-middleware.js';
import { shikiConfig, defaultHighlightLang } from './src/lib/markdown.js';

const BASE = '/';

// GitHub Pages serves static HTML redirects for old component bookmarks.
const componentRedirects = Object.fromEntries(
  ['', 'zh-CN/'].flatMap((locale) => {
    const entries = readdirSync(new URL(`./${locale}component/`, import.meta.url))
      .filter((name) => name.endsWith('.md'))
      .map((name) => {
        const slug = name.slice(0, -3);
        return [
          `/${locale}docs/components/${slug}`,
          `/${locale}component${slug === 'index' ? '' : `/${slug}`}`,
        ];
      });
    return [[`/${locale}docs/components`, `/${locale}component`], ...entries];
  }),
);

// PR #3010 (root/theme/dock migration): these three guides lived directly
// under /docs before this move, so they need a literal old->new mapping —
// componentRedirects only recognizes the /docs/components/<slug> prefix.
const legacyDocRedirects = Object.fromEntries(
  ['', 'zh-CN/'].flatMap((locale) =>
    ['root', 'theme', 'dock'].map((slug) => [
      `/${locale}docs/${slug}`,
      `/${locale}component/${slug}`,
    ]),
  ),
);

export default defineConfig({
  site: 'https://gpui-kit.com',
  base: BASE,
  output: 'static',
  trailingSlash: 'never',
  redirects: {
    ...componentRedirects,
    ...legacyDocRedirects,
    '/docs/ui-testing': '/docs/test',
    '/zh-CN/docs/ui-testing': '/zh-CN/docs/test',
  },

  integrations: [
    vue({ devtools: false }),
    pagefind(),
  ],

  markdown: {
    // Astro 7 made Sätteri the default processor; the remark/rehype pipeline is
    // opt-in now, and the math plugins only run on it.
    processor: unified({
      remarkPlugins: [remarkMath, remarkSnippets, remarkCallouts, [remarkDocLinks, { base: BASE }]],
      rehypePlugins: [rehypeMathjax, rehypeHeadingIds, rehypeHeadingAnchors],
    }),
    shikiConfig,
    defaultHighlightLang,
  },

  vite: {
    plugins: [tailwindcss(), wasmExamplesDevServer(BASE)],
  },
});
