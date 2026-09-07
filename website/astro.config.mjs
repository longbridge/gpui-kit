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

export default defineConfig({
  site: 'https://gpui-kit.com',
  base: BASE,
  output: 'static',
  trailingSlash: 'never',

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
