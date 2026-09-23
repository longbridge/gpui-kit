// Vite reads the source theme files at build time. Adding a JSON file or a
// variant to themes/ automatically makes it available on the website.
type Variant = {
  name: string;
  mode: 'light' | 'dark';
  colors: Record<string, string>;
  highlight?: Record<string, unknown>;
};
type ThemeFile = { name: string; themes: Variant[] };

const files = import.meta.glob<ThemeFile>('../../../themes/*.json', { eager: true, import: 'default' });
const slug = (value: string) => value.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');

export const themeSources = Object.entries(files).map(([path, data]) => ({
  source: path.split('/').pop()!.replace(/\.json$/, ''),
  data,
}));

export const themes = Object.entries(files)
  .flatMap(([path, file]) => file.themes.map((variant) => ({
    id: `${path.split('/').pop()!.replace(/\.json$/, '')}-${slug(variant.name)}`,
    source: path.split('/').pop()!.replace(/\.json$/, ''),
    family: file.name,
    name: variant.name,
    mode: variant.mode,
    colors: variant.colors,
    highlight: variant.highlight,
  })))
  .sort((a, b) => a.name.localeCompare(b.name));

export const themeInfo = Object.fromEntries(themes.map(({ id, mode, name, source }) => [id, { mode, name, source }]));

const tokens: Record<string, string> = {
  background: 'background',
  foreground: 'foreground',
  card: 'background',
  'card-foreground': 'foreground',
  popover: 'popover.background',
  'popover-foreground': 'popover.foreground',
  primary: 'primary.background',
  'primary-foreground': 'primary.foreground',
  secondary: 'secondary.background',
  'secondary-foreground': 'secondary.foreground',
  muted: 'muted.background',
  'muted-foreground': 'muted.foreground',
  accent: 'accent.background',
  'accent-foreground': 'accent.foreground',
  border: 'border',
  input: 'input.border',
  ring: 'ring',
  titlebar: 'title_bar.background',
  sidebar: 'muted.background',
  'sidebar-foreground': 'foreground',
  'sidebar-primary': 'primary.background',
  'sidebar-primary-foreground': 'primary.foreground',
  'sidebar-accent': 'accent.background',
  'sidebar-accent-foreground': 'accent.foreground',
  'sidebar-border': 'border',
  'sidebar-ring': 'ring',
  brand: 'primary.background',
  'brand-hover': 'primary.hover.background',
  'brand-contrast': 'primary.foreground',
  'brand-subtle': 'muted.background',
  selection: 'list.active.background',
  success: 'base.green',
  warning: 'base.yellow',
  destructive: 'base.red',
  'scrollbar-track': 'scrollbar.background',
  'scrollbar-thumb': 'scrollbar.thumb.background',
  'data-1': 'base.cyan',
  'data-2': 'base.blue',
  'data-3': 'base.green',
  'data-4': 'base.yellow',
  'data-5': 'base.magenta',
  'code-bg': 'editor.background',
  'code-fg': 'editor.foreground',
  'code-keyword': 'syntax.keyword',
  'code-string': 'syntax.string',
  'code-comment': 'syntax.comment',
  'code-fn': 'syntax.function',
  'code-type': 'syntax.type',
};

function highlightColor(highlight: Record<string, unknown> | undefined, key: string): string | undefined {
  const value = highlight?.[key] ?? key.split('.').reduce<unknown>((part, segment) =>
    part && typeof part === 'object' ? (part as Record<string, unknown>)[segment] : undefined, highlight);
  return typeof value === 'string' ? value : value && typeof value === 'object'
    ? (value as { color?: string }).color : undefined;
}

export const themeCss = themes.map(({ id, colors, highlight }) => {
  const declarations = Object.entries(tokens).flatMap(([token, source]) => {
    const value = source.startsWith('editor.') || source.startsWith('syntax.')
      ? highlightColor(highlight, source) : colors[source];
    return value ? [`--${token}:${value};`] : [];
  });
  return `html[data-theme="${id}"]{${declarations.join('')}}`;
}).join('\n');
