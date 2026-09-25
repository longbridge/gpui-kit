import { readdirSync, readFileSync, statSync } from 'node:fs';
import { extname, join, relative, resolve } from 'node:path';
import { expandDocVariables } from './doc-variables.js';

const SITE_TITLE = 'GPUI Kit';
const SITE_DESCRIPTION =
  'A comprehensive Rust framework for building fantastic, high-performance desktop apps with GPUI.';
const BASE_URL = import.meta.env.BASE_URL.replace(/\/$/, '');

interface PageEntry {
  title: string;
  url: string;
  body: string;
  description?: string;
  recipes: string[];
}

interface RecipeInventoryEntry {
  id: string;
  documents: string[];
  trust: string;
}

const TESTED_RECIPE_LABEL = 'Tested consumer recipe';

/** Keep the machine-readable documentation exports in step with the HTML footer. */
export function documentationLicenseNotice(lang: 'en' | 'zh-CN', source: string): string {
  if (lang === 'zh-CN') {
    return `文档许可：GPUI Kit 有权授权的原创正文与图示另以 CC BY 4.0 提供。复制或改编时请署名 GPUI Kit，链接原文（${source}）及 https://creativecommons.org/licenses/by/4.0/，并注明修改。代码示例与软件源码采用 Apache-2.0；第三方内容保留原许可；既有 Apache-2.0 使用权不受影响。`;
  }
  return `Documentation license: original prose and illustrations for which GPUI Kit holds licensing rights are also offered under CC BY 4.0. When copying or adapting, credit GPUI Kit, link the source (${source}) and https://creativecommons.org/licenses/by/4.0/, and indicate changes. Code examples and software source use Apache-2.0; third-party material retains its terms; existing Apache-2.0 permissions remain.`;
}

function documentationLicenseSummary(lang: 'en' | 'zh-CN', source: string): string {
  return lang === 'zh-CN'
    ? `许可：GPUI Kit 有权授权的正文与原创图示另适用 CC BY 4.0；引用请署名 GPUI Kit、链接 ${source} 与 https://creativecommons.org/licenses/by/4.0/ 并注明修改。代码示例适用 Apache-2.0。`
    : `License: GPUI Kit-authorized prose and original illustrations are also CC BY 4.0; credit GPUI Kit, link ${source} and https://creativecommons.org/licenses/by/4.0/, and indicate changes. Code examples use Apache-2.0.`;
}

function recipeDestinations(websiteRoot: string): Map<string, string[]> {
  const destinations = new Map<string, string[]>();
  try {
    const inventory = JSON.parse(
      readFileSync(join(websiteRoot, '..', 'examples/ai_recipes/recipes.json'), 'utf8'),
    ) as RecipeInventoryEntry[];
    for (const recipe of inventory) {
      if (recipe.trust !== TESTED_RECIPE_LABEL) continue;
      for (const document of recipe.documents) {
        const path = resolve(websiteRoot, '..', document);
        destinations.set(path, [...(destinations.get(path) ?? []), recipe.id]);
      }
    }
  } catch {
    // The website can still build when checked out independently of the recipes workspace.
  }
  return destinations;
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

function scanDir(
  dir: string,
  baseDir: string,
  urlPrefix: string,
  recipePaths: Map<string, string[]>,
): PageEntry[] {
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
      // and repeated the nested directory in every URL.
      const sub = scanDir(fullPath, baseDir, urlPrefix, recipePaths);
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
      const url = `${BASE_URL}/${urlPrefix}/${relPath}`.replace(/\/+/g, '/').replace(/\/$/, '');
      const body = expandDocVariables(expandSnippets(bodyWithoutFrontmatter(content), dir));

      try {
        results.push({
          title,
          url,
          body,
          description: parseFrontmatterField(content, 'description'),
          recipes: recipePaths.get(resolve(fullPath)) ?? [],
        });
      } catch (err) {
        console.warn(`[llms] skipping ${fullPath}:`, err);
      }
    }
  }
  return results;
}

const SECTIONS = (root: string) => [
  { dir: join(root, 'docs'), prefix: 'docs' },
  { dir: join(root, 'component'), prefix: 'component' },
  { dir: join(root, 'shell'), prefix: 'shell' },
  { dir: join(root, 'base'), prefix: 'base' },
  { dir: join(root, 'zh-CN/docs'), prefix: 'zh-CN/docs' },
  { dir: join(root, 'zh-CN/component'), prefix: 'zh-CN/component' },
  { dir: join(root, 'zh-CN/shell'), prefix: 'zh-CN/shell' },
  { dir: join(root, 'zh-CN/base'), prefix: 'zh-CN/base' },
];

/**
 * `llms.txt`: the site's table of contents, one line per page pointing at the
 * markdown behind it. A model reads this to decide what to fetch, where
 * `llms-full.txt` is everything at once.
 */
export function buildLlmsIndex(websiteRoot: string): string {
  const recipePaths = recipeDestinations(websiteRoot);
  const entries = SECTIONS(websiteRoot).flatMap(({ dir, prefix }) => scanDir(dir, dir, prefix, recipePaths));
  const lines = entries
    .sort((a, b) => a.title.localeCompare(b.title, 'en') || a.url.localeCompare(b.url))
    .map((entry) =>
      `- [${entry.title}](${entry.url}.md)${entry.description ? `: ${entry.description}` : ''}${
        entry.recipes.length ? ` — ${TESTED_RECIPE_LABEL}: ${entry.recipes.join(', ')}` : ''
      }`,
    );

  return `# ${SITE_TITLE}\n\n> ${SITE_DESCRIPTION}\n\n${documentationLicenseNotice('en', 'https://gpui-kit.com')}\n\n## Table of Contents\n\n${lines.join('\n')}\n`;
}

export function buildLlmsContent(websiteRoot: string): string {
  const recipePaths = recipeDestinations(websiteRoot);
  const sections = [
    { dir: join(websiteRoot, 'docs'), prefix: 'docs' },
    { dir: join(websiteRoot, 'component'), prefix: 'component' },
    { dir: join(websiteRoot, 'shell'), prefix: 'shell' },
    { dir: join(websiteRoot, 'base'), prefix: 'base' },
    { dir: join(websiteRoot, 'zh-CN/docs'), prefix: 'zh-CN/docs' },
    { dir: join(websiteRoot, 'zh-CN/component'), prefix: 'zh-CN/component' },
    { dir: join(websiteRoot, 'zh-CN/shell'), prefix: 'zh-CN/shell' },
    { dir: join(websiteRoot, 'zh-CN/base'), prefix: 'zh-CN/base' },
  ];

  const header = `# ${SITE_TITLE}\n\n> ${SITE_DESCRIPTION}\n\n${documentationLicenseNotice('en', 'https://gpui-kit.com')}\n\n---\n`;

  const pages: string[] = [];
  for (const { dir, prefix } of sections) {
    const entries = scanDir(dir, dir, prefix, recipePaths);
    for (const entry of entries) {
      const body = forPlainText(withoutLeadingHeading(entry.body, entry.title), entry.url);
      const provenance = entry.recipes.length
        ? `\n\n${TESTED_RECIPE_LABEL}: ${entry.recipes.join(', ')}`
        : '';
      const lang = entry.url.startsWith(`${BASE_URL}/zh-CN/`) ? 'zh-CN' : 'en';
      const source = `https://gpui-kit.com${entry.url}`;
      pages.push(`# ${entry.title}\n\nSource: ${entry.url}${provenance}\n\n${documentationLicenseSummary(lang, source)}\n\n${body}`);
    }
  }

  return header + pages.join('\n\n---\n\n');
}
