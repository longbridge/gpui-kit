import { readFileSync } from 'node:fs';
import { join } from 'node:path';

// The site ships its own Shiki themes; `global.css` derives its `--code-*`
// values from them, so a stock theme here would put the code block's own
// colours out of step with the chrome around it. Read from the project root,
// which is where Astro runs (walking up from `import.meta.url` breaks once the
// build rearranges server assets).
const theme = (name) =>
  JSON.parse(readFileSync(join(process.cwd(), 'src', `${name}.theme.json`), 'utf8'));

// Shiki settings shared by the docs pipeline in `astro.config.mjs` and the
// release-notes renderer in `releases.ts`, so a code block in a release note is
// highlighted exactly like the same block in the docs.
export const shikiConfig = {
  themes: {
    light: theme('light'),
    dark: theme('dark'),
  },
  defaultColor: 'light',
  langs: ['rust'],
  langAlias: { rs: 'rust' },
};

export const defaultHighlightLang = 'rust';
