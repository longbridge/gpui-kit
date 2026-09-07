import { readdirSync, readFileSync, statSync } from 'node:fs';
import { extname, join, relative } from 'node:path';

export interface SidebarItem {
  text: string;
  link?: string;
  items?: SidebarItem[];
  collapsed?: boolean;
}

export interface SidebarGeneratorConfig {
  /** Path relative to website/ root, e.g. "docs" */
  contentDir: string;
  /** Absolute URL prefix, e.g. "/gpui-component/docs" */
  baseUrl: string;
  /** Top-level group label */
  rootGroupText: string;
  /** If set, prepend this as the first item pointing to baseUrl */
  rootLinkText?: string;
}

function parseFrontmatter(content: string): { title?: string; order?: number } {
  const match = content.match(/^---\r?\n([\s\S]*?)\r?\n---/);
  if (!match) return {};
  const raw = match[1];
  const title = raw.match(/^title:\s*(.+)$/m)?.[1]?.trim().replace(/^["']|["']$/g, '');
  // `\d+` alone never matched the values actually in use — they are negative
  // and often fractional (`-2.1`).
  const orderStr = raw.match(/^order:\s*(-?\d+(?:\.\d+)?)/m)?.[1];
  return {
    title,
    order: orderStr ? parseFloat(orderStr) : undefined,
  };
}

function titleFromHeading(content: string): string | undefined {
  const match = content.match(/^#\s+(.+)$/m);
  return match?.[1]?.trim();
}

function titleCase(name: string): string {
  return name
    .replace(/[-_]/g, ' ')
    .replace(/\b\w/g, (c) => c.toUpperCase());
}

function getFileTitle(filePath: string, content: string): string {
  const fm = parseFrontmatter(content);
  if (fm.title) return fm.title;
  const heading = titleFromHeading(content);
  if (heading) return heading;
  const name = filePath.split('/').pop()!.replace(/\.md$/, '');
  return titleCase(name);
}

/**
 * Sort weight for a page.
 *
 * The published site generates its sidebar with `vitepress-sidebar`, which
 * reads `order` as a magnitude — a page at `-1` leads and one at `-7` trails.
 * Every `order` in this repository was written against that behaviour, so the
 * same reading is what preserves the published order; sorting them as signed
 * numbers would reverse the section.
 */
function getFileOrder(content: string): number {
  const { order } = parseFrontmatter(content);
  return order === undefined ? 999 : Math.abs(order);
}

interface FileEntry {
  name: string;
  path: string;
  isDir: boolean;
  order: number;
  title: string;
  items?: FileEntry[];
}

function scanDir(dir: string, baseDir: string): FileEntry[] {
  let entries: FileEntry[];
  try {
    entries = readdirSync(dir).map((name) => {
      const fullPath = join(dir, name);
      const relPath = relative(baseDir, fullPath);
      const isDir = statSync(fullPath).isDirectory();
      if (isDir) {
        const children = scanDir(fullPath, baseDir);
        return { name, path: relPath, isDir: true, order: 999, title: titleCase(name), items: children };
      }
      if (extname(name) !== '.md') return null;
      if (name === 'index.md') return null;
      let content = '';
      try { content = readFileSync(fullPath, 'utf-8'); } catch {}
      return {
        name,
        path: relPath,
        isDir: false,
        order: getFileOrder(content),
        title: getFileTitle(relPath, content),
      };
    }).filter(Boolean) as FileEntry[];
  } catch {
    return [];
  }
  return entries;
}

function entriesToSidebarItems(
  entries: FileEntry[],
  baseUrl: string,
  isComponentsDir = false
): SidebarItem[] {
  const dirs = entries.filter((e) => e.isDir);
  const files = entries.filter((e) => !e.isDir);

  const CATALOG_DIRS = ['components', 'primitives'];
  const catalogDir = dirs.find((d) => CATALOG_DIRS.includes(d.name.toLowerCase()));
  const otherDirs = dirs.filter((d) => d !== catalogDir);

  // Sort non-catalog items by order then name
  const sortByOrder = (a: FileEntry, b: FileEntry) =>
    a.order !== b.order ? a.order - b.order : a.name.localeCompare(b.name, 'en');

  files.sort(sortByOrder);
  otherDirs.sort(sortByOrder);

  const fileItems: SidebarItem[] = files.map((f) => ({
    text: f.title,
    link: `${baseUrl}/${f.path.replace(/\.md$/, '')}`,
    collapsed: false,
  }));

  const otherDirItems: SidebarItem[] = otherDirs.map((d) => ({
    text: d.title,
    collapsed: false,
    items: entriesToSidebarItems(d.items ?? [], baseUrl),
  }));

  let result: SidebarItem[] = [...fileItems, ...otherDirItems];

  if (catalogDir) {
    const catalogItems = (catalogDir.items ?? [])
      .filter((e) => !e.isDir)
      .sort((a, b) => a.title.localeCompare(b.title, 'en', { sensitivity: 'base' }))
      .map((f) => ({
        text: f.title,
        link: `${baseUrl}/${f.path.replace(/\.md$/, '')}`,
        collapsed: false,
      }));

    const label = catalogDir.name.toLowerCase() === 'primitives' ? 'Primitives' : 'Components';
    result.push({
      text: label,
      collapsed: false,
      items: catalogItems,
    });
  }

  return result;
}

export function generateSidebar(config: SidebarGeneratorConfig): SidebarItem[] {
  const entries = scanDir(config.contentDir, config.contentDir);
  const items = entriesToSidebarItems(entries, config.baseUrl);

  const rootGroup: SidebarItem = {
    text: config.rootGroupText,
    collapsed: false,
    items,
  };

  if (config.rootLinkText) {
    rootGroup.items = [
      { text: config.rootLinkText, link: config.baseUrl },
      ...(rootGroup.items ?? []),
    ];
  }

  return [rootGroup];
}

// Pre-generate sidebars at build time.
//
// Not `import.meta.url`: Astro rearranges server assets during the build, so
// walking up from this module's own location stopped landing on the content
// directories and every sidebar generated empty. Astro runs from the project
// root, which is the stable anchor. (`llms.ts` had the same fault.)
const WEBSITE_ROOT = process.cwd();
const BASE = import.meta.env.BASE_URL.replace(/\/$/, '');

export const enDocsSidebar = generateSidebar({
  contentDir: join(WEBSITE_ROOT, 'docs'),
  baseUrl: `${BASE}/docs`,
  rootGroupText: 'Introduction',
});

export const enShellSidebar = generateSidebar({
  contentDir: join(WEBSITE_ROOT, 'shell'),
  baseUrl: `${BASE}/shell`,
  rootGroupText: 'GPUI Shell',
  rootLinkText: 'Introduction',
});

export const enBaseSidebar = generateSidebar({
  contentDir: join(WEBSITE_ROOT, 'base'),
  baseUrl: `${BASE}/base`,
  rootGroupText: 'GPUI Base',
});

export const zhDocsSidebar = generateSidebar({
  contentDir: join(WEBSITE_ROOT, 'zh-CN/docs'),
  baseUrl: `${BASE}/zh-CN/docs`,
  rootGroupText: '文档',
});

export const zhShellSidebar = generateSidebar({
  contentDir: join(WEBSITE_ROOT, 'zh-CN/shell'),
  baseUrl: `${BASE}/zh-CN/shell`,
  rootGroupText: 'GPUI Shell',
  rootLinkText: '简介',
});

export const zhBaseSidebar = generateSidebar({
  contentDir: join(WEBSITE_ROOT, 'zh-CN/base'),
  baseUrl: `${BASE}/zh-CN/base`,
  rootGroupText: 'GPUI Base',
});
