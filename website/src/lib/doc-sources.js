import { existsSync, readdirSync, statSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import { unified } from 'unified';
import remarkParse from 'remark-parse';
import remarkGfm from 'remark-gfm';
import remarkFrontmatter from 'remark-frontmatter';
import remarkMath from 'remark-math';
import { visit } from 'unist-util-visit';

// Directories whose Markdown is published as versioned documentation. Every
// versioned build renders these below its own base (`/versions/<id>/...`).
export const CONTENT_DIRS = [
  'docs', 'component', 'base', 'shell',
  'zh-CN/docs', 'zh-CN/component', 'zh-CN/base', 'zh-CN/shell',
];

// A link written from the site root to versioned content. It points at the
// default version from every other version, so the source must link to the
// file instead and let the build place it inside the current version.
export const ROOT_CONTENT_LINK = /^\/(?:zh-CN(?:\/|$|#)|(?:zh-CN\/)?(?:docs|component|base|shell)(?:[/#]|$)|$|#)/;

/** Every published Markdown file, as paths relative to `root`. */
export function contentFiles(root) {
  const files = [];
  const walk = (dir) => {
    for (const name of readdirSync(dir)) {
      const path = join(dir, name);
      if (statSync(path).isDirectory()) walk(path);
      else if (name.endsWith('.md')) files.push(relative(root, path).split(sep).join('/'));
    }
  };
  for (const dir of CONTENT_DIRS) {
    const path = join(root, dir);
    if (existsSync(path)) walk(path);
  }
  // `zh-CN/docs` is walked on its own; keep one entry per file.
  return [...new Set(files)].sort();
}

const parser = unified().use(remarkParse).use(remarkGfm).use(remarkFrontmatter).use(remarkMath);

/**
 * Link destinations in a Markdown source, with the source offsets of the node
 * that holds each one. Code blocks and inline code are not links.
 */
export function markdownLinks(source) {
  const links = [];
  visit(parser.parse(source), (node) => {
    if ((node.type === 'link' || node.type === 'definition') && node.url) {
      links.push({
        url: node.url,
        line: node.position.start.line,
        start: node.position.start.offset,
        end: node.position.end.offset,
      });
    }
  });
  return links;
}
