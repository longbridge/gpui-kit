import { readdirSync, readFileSync, statSync } from 'node:fs';
import { extname, join, relative } from 'node:path';

const SITE_TITLE = 'GPUI Kit';
const SITE_DESCRIPTION =
  'A comprehensive Rust framework for building fantastic, high-performance desktop apps with GPUI.';
const BASE_URL = import.meta.env.BASE_URL.replace(/\/$/, '');

interface PageEntry {
  title: string;
  url: string;
  body: string;
  description?: string;
}

function parseFrontmatterField(content: string, field: string): string | undefined {
  const match = content.match(/^---\r?\n([\s\S]*?)\r?\n---/);
  if (!match) return undefined;
  // A folded value (`description: >-`) continues on the indented lines below it.
  const folded = match[1].match(new RegExp(`^${field}:\\s*>-?\\s*\\n((?:[ \\t]+.*\\n?)+)`, 'm'));
  if (folded) return folded[1].split('\n').map((line) => line.trim()).filter(Boolean).join(' ');
  return match[1]
    .match(new RegExp(`^${field}:\\s*(.+)$`, 'm'))?.[1]
    ?.trim()
    .replace(/^["']|["']$/g, '');
}

function parseFrontmatterTitle(content: string): string | undefined {
  const match = content.match(/^---\r?\n([\s\S]*?)\r?\n---/);
  if (!match) return undefined;
  return match[1].match(/^title:\s*(.+)$/m)?.[1]?.trim().replace(/^["']|["']$/g, '');
}

export function bodyWithoutFrontmatter(content: string): string {
  return content.replace(/^---\r?\n[\s\S]*?\r?\n---\r?\n?/, '').trim();
}

/** The entry prints the title itself, so a leading `# Title` would repeat it. */
function withoutLeadingHeading(body: string, title: string): string {
  const match = body.match(/^#\s+(.+?)\s*(?:\r?\n|$)/);
  if (!match || match[1].trim() !== title.trim()) return body;
  return body.slice(match[0].length).trimStart();
}

/**
 * Cleans up markup a reader of the plain-text bundle cannot resolve:
 * VitePress container fences, which are noise without their renderer, and
 * in-repo `.md` links, which point at source paths rather than pages.
 */
function forPlainText(body: string, url: string): string {
  const dir = url.replace(/\/[^/]*$/, '');
  return body
    // `:::tip` / `::: warning Title` open a callout; `:::` closes it. Keep any
    // title as a plain line so the emphasis is not lost entirely.
    .replace(/^:::[ \t]*[a-z]+[ \t]*(.*)$/gim, (_, title: string) => (title.trim() ? `**${title.trim()}**` : ''))
    .replace(/^:::[ \t]*$/gm, '')
    .replace(/\]\(([^)]+?)\.md(#[^)]*)?\)/g, (whole: string, target: string, hash = '') => {
      if (/^[a-z][a-z\d+.-]*:/i.test(target) || target.startsWith('//')) return whole;
      const path = target.startsWith('/')
        ? target
        : new URL(target, `https://x${dir}/`).pathname.replace(/\/index$/, '');
      return `](${path}${hash})`;
    })
    .replace(/\n{3,}/g, '\n\n')
    .trim();
}

/**
 * Expands VitePress snippet imports (`<<< ../path.rs{rust}`) into fenced code,
 * so the bundle carries the source a reader of the page would see rather than
 * a path they cannot follow.
 */
export function expandSnippets(body: string, fileDir: string): string {
  return body.replace(/^<<<\s+(\S+?)(?:\{([^}]*)\})?[ \t]*$/gm, (whole, target: string, braces = '') => {
    const [rel] = target.split('#');
    const lang =
      braces.trim().split(/\s+/).find((part) => /^[a-z][\w+-]*$/i.test(part)) ??
      { rs: 'rust', ts: 'typescript', js: 'javascript', toml: 'toml' }[rel.split('.').pop()?.toLowerCase() ?? ''] ??
      '';
    try {
      const source = readFileSync(join(fileDir, rel), 'utf-8').replace(/\s+$/, '');
      return '```' + lang + '\n' + source + '\n```';
    } catch {
      return whole;
    }
  });
}

function scanDir(dir: string, baseDir: string, urlPrefix: string): PageEntry[] {
  const results: PageEntry[] = [];
  let entries: string[];
  try {
    entries = readdirSync(dir);
  } catch {
    return results;
  }

  for (const name of entries) {
    const fullPath = join(dir, name);
    let stat: ReturnType<typeof statSync>;
    try { stat = statSync(fullPath); } catch { continue; }

    if (stat.isDirectory()) {
      // `relPath` below is already relative to `baseDir`, so the prefix must
      // stay the tree's root — appending the directory here counted it twice
      // and produced `/docs/components/components/accordion`.
      const sub = scanDir(fullPath, baseDir, urlPrefix);
      results.push(...sub);
    } else if (extname(name) === '.md') {
      let content = '';
      try { content = readFileSync(fullPath, 'utf-8'); } catch { continue; }

      const title =
        parseFrontmatterTitle(content) ||
        content.match(/^#\s+(.+)$/m)?.[1]?.trim() ||
        name.replace(/\.md$/, '');

      const relPath = relative(baseDir, fullPath)
        .replace(/\.md$/, '')
        .replace(/index$/, '');
      const url = `${BASE_URL}/${urlPrefix}/${relPath}`.replace(/\/+/g, '/');
      const body = expandSnippets(bodyWithoutFrontmatter(content), dir);

      try {
        results.push({ title, url, body, description: parseFrontmatterField(content, 'description') });
      } catch (err) {
        console.warn(`[llms] skipping ${fullPath}:`, err);
      }
    }
  }
  return results;
}

const SECTIONS = (root: string) => [
  { dir: join(root, 'docs'), prefix: 'docs' },
  { dir: join(root, 'shell'), prefix: 'shell' },
  { dir: join(root, 'base'), prefix: 'base' },
  { dir: join(root, 'zh-CN/docs'), prefix: 'zh-CN/docs' },
  { dir: join(root, 'zh-CN/shell'), prefix: 'zh-CN/shell' },
  { dir: join(root, 'zh-CN/base'), prefix: 'zh-CN/base' },
];

/**
 * `llms.txt`: the site's table of contents, one line per page pointing at the
 * markdown behind it. A model reads this to decide what to fetch, where
 * `llms-full.txt` is everything at once.
 */
export function buildLlmsIndex(websiteRoot: string): string {
  const entries = SECTIONS(websiteRoot).flatMap(({ dir, prefix }) => scanDir(dir, dir, prefix));
  const lines = entries
    .sort((a, b) => a.title.localeCompare(b.title, 'en') || a.url.localeCompare(b.url))
    .map((entry) => `- [${entry.title}](${entry.url}.md)${entry.description ? `: ${entry.description}` : ''}`);

  return `# ${SITE_TITLE}\n\n> ${SITE_DESCRIPTION}\n\n## Table of Contents\n\n${lines.join('\n')}\n`;
}

export function buildLlmsContent(websiteRoot: string): string {
  const sections = [
    { dir: join(websiteRoot, 'docs'), prefix: 'docs' },
    { dir: join(websiteRoot, 'shell'), prefix: 'shell' },
    { dir: join(websiteRoot, 'base'), prefix: 'base' },
    { dir: join(websiteRoot, 'zh-CN/docs'), prefix: 'zh-CN/docs' },
    { dir: join(websiteRoot, 'zh-CN/shell'), prefix: 'zh-CN/shell' },
    { dir: join(websiteRoot, 'zh-CN/base'), prefix: 'zh-CN/base' },
  ];

  const header = `# ${SITE_TITLE}\n\n> ${SITE_DESCRIPTION}\n\n---\n`;

  const pages: string[] = [];
  for (const { dir, prefix } of sections) {
    const entries = scanDir(dir, dir, prefix);
    for (const entry of entries) {
      const body = forPlainText(withoutLeadingHeading(entry.body, entry.title), entry.url);
      pages.push(`# ${entry.title}\n\nSource: ${entry.url}\n\n${body}`);
    }
  }

  return header + pages.join('\n\n---\n\n');
}
