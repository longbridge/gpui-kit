import { readFileSync } from 'node:fs';
import { dirname, relative, resolve } from 'node:path';

// VitePress's snippet import — `<<< ../path/to/file.rs{rust}` — pulls a file
// into the page as a code block. Astro has no such syntax, so the line was
// rendered as literal text on 77 pages. This reads the file and replaces the
// line with a real code node, which then goes through Shiki like any other
// fenced block.
//
// The brace suffix carries the language and/or the lines to highlight, as in
// `{rust}`, `{1,3-5}` or `{1,3-5 rust}`. A `#region` fragment selects a named
// region marked in the source file.

const SNIPPET = /^<<<\s+(\S+?)(?:\{([^}]*)\})?$/;

/** `{1,3-5 rust}` → `{ meta: '{1,3-5}', lang: 'rust' }` */
function parseBraces(raw) {
  if (!raw) return { meta: undefined, lang: undefined };
  const parts = raw.trim().split(/\s+/);
  const lang = parts.find((part) => /^[a-z][\w+-]*$/i.test(part) && !/^[\d,-]+$/.test(part));
  const lines = parts.filter((part) => part !== lang).join(' ');
  return { meta: lines ? `{${lines}}` : undefined, lang };
}

/** `// #region name` … `// #endregion name` in the source file. */
function selectRegion(source, region) {
  const start = new RegExp(`^.*#region\\s+${region}\\s*$`, 'm').exec(source);
  if (!start) return undefined;
  const after = source.slice(start.index + start[0].length);
  const end = new RegExp(`^.*#endregion\\s+${region}\\s*$`, 'm').exec(after);
  return (end ? after.slice(0, end.index) : after).replace(/^\n/, '').trimEnd();
}

const LANG_BY_EXTENSION = {
  rs: 'rust', ts: 'typescript', js: 'javascript', mjs: 'javascript',
  jsx: 'jsx', tsx: 'tsx', json: 'json', toml: 'toml', md: 'markdown',
  sh: 'bash', bash: 'bash', css: 'css', html: 'html', vue: 'vue', yml: 'yaml', yaml: 'yaml',
};

export function remarkSnippets({ root = process.cwd() } = {}) {
  const repoRoot = resolve(root, '..');

  return (tree, file) => {
    const from = file?.path ?? file?.history?.[0];
    if (!from) return;
    const fromDir = dirname(from);

    tree.children = tree.children.map((node) => {
      if (node.type !== 'paragraph' || node.children?.length !== 1) return node;
      const text = node.children[0];
      if (text.type !== 'text') return node;
      const match = SNIPPET.exec(text.value.trim());
      if (!match) return node;

      const [, target, braces] = match;
      const [path, region] = target.split('#');
      const absolute = resolve(fromDir, path);

      // Snippets name files inside the repository; anything else is a mistake
      // in the page, not a path to follow.
      if (relative(repoRoot, absolute).startsWith('..')) {
        console.warn(`[snippets] ${from}: refusing to read outside the repository: ${target}`);
        return node;
      }

      let source;
      try {
        source = readFileSync(absolute, 'utf8');
      } catch {
        console.warn(`[snippets] ${from}: cannot read ${target}`);
        return node;
      }

      if (region) {
        const selected = selectRegion(source, region);
        if (selected === undefined) {
          console.warn(`[snippets] ${from}: no region "${region}" in ${path}`);
          return node;
        }
        source = selected;
      }

      const { meta, lang } = parseBraces(braces);
      return {
        type: 'code',
        lang: lang ?? LANG_BY_EXTENSION[path.split('.').pop()?.toLowerCase()] ?? null,
        meta: meta ?? null,
        value: source.replace(/\s+$/, ''),
      };
    });
  };
}
