import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, relative, sep } from 'node:path';
import test from 'node:test';
import { contentFiles, markdownLinks, ROOT_CONTENT_LINK } from '../src/lib/doc-sources.js';
import { DOC_VARIABLE_NAMES, docVariable } from '../src/lib/doc-variables.js';

// Source checks for the published Markdown. They run without a build, so a
// pull request learns about a link that would leave its version, or a pinned
// version written out by hand, before the site is built.

const root = new URL('..', import.meta.url).pathname;
const files = contentFiles(root);
const read = (file: string) => readFileSync(join(root, file), 'utf8');
const ASSET = /\.(?:png|jpe?g|gif|svg|webp|avif|ico|pdf|zip|gz|txt|json|toml|ya?ml|rs|css|js|mjs|ts)$/i;

// Deliberate references to a GPUI snapshot other than the one the workspace
// pins. Anything else must use `{{gpui_pre_version}}`.
const INTENTIONAL_GPUI_PINS: Record<string, string[]> = {
  // The mobile platform fork is pinned to an older snapshot than Kit, and
  // `gpui-pre-mobile` has its own crate version.
  'docs/mobile.md': ['0.3.4', '0.1.0'],
  'zh-CN/docs/mobile.md': ['0.3.4', '0.1.0'],
};

/** The source file a relative link resolves to, or undefined. */
function linkTarget(file: string, url: string) {
  const path = url.split('#')[0];
  const resolved = join(dirname(join(root, file)), path.replace(/\.md$/, ''));
  return [`${resolved}.md`, join(resolved, 'index.md')].find(existsSync);
}

test('documentation links stay inside the version being read', () => {
  const failures: string[] = [];
  for (const file of files) {
    for (const link of markdownLinks(read(file))) {
      if (ROOT_CONTENT_LINK.test(link.url)) {
        failures.push(`${file}:${link.line} ${link.url} — link to the .md file with a relative path`);
      }
    }
  }
  assert.deepEqual(failures, [], `Site-root links send versioned pages to the default version:\n${failures.join('\n')}`);
});

test('relative documentation links resolve to a page in the same locale', () => {
  const failures: string[] = [];
  for (const file of files) {
    const zh = file.startsWith('zh-CN/');
    for (const link of markdownLinks(read(file))) {
      const { url } = link;
      if (/^[a-z][a-z\d+.-]*:/i.test(url) || url.startsWith('/') || url.startsWith('#')) continue;
      const path = url.split('#')[0];
      if (!path || ASSET.test(path)) continue;
      // Links out of the website (to crates or examples) are not pages.
      if (relative(root, join(dirname(join(root, file)), path)).startsWith('..')) continue;

      const target = linkTarget(file, url);
      if (!target) {
        failures.push(`${file}:${link.line} ${url} — no such page`);
        continue;
      }
      const targetFile = relative(root, target).split(sep).join('/');
      if (zh && !targetFile.startsWith('zh-CN/') && existsSync(join(root, 'zh-CN', targetFile))) {
        failures.push(`${file}:${link.line} ${url} — links the English page; zh-CN/${targetFile} exists`);
      }
    }
  }
  assert.deepEqual(failures, [], failures.join('\n'));
});

test('GPUI snapshot versions come from the workspace pin', () => {
  const pinned = docVariable('gpui_pre_version');
  const failures: string[] = [];
  for (const file of files) {
    const allowed = INTENTIONAL_GPUI_PINS[file] ?? [];
    read(file).split('\n').forEach((line, index) => {
      const at = `${file}:${index + 1}`;
      if (new RegExp(`(?<![\\d.])${pinned.replace(/\./g, '\\.')}(?!\\d|\\.\\d)`).test(line)) {
        failures.push(`${at} writes the pinned ${pinned} — use {{gpui_pre_version}}`);
      }
      if (/docs\.rs\/(?:crate\/)?gpui-pre[\w-]*\/\d/.test(line)) {
        failures.push(`${at} links a fixed gpui-pre version on docs.rs — use {{gpui_pre_version}}`);
      }
      if (/gpui-pre/.test(line)) {
        for (const version of line.match(/(?<![\w.])\d+\.\d+\.\d+(?![\w-]|\.\d)/g) ?? []) {
          if (version !== pinned && !allowed.includes(version)) {
            failures.push(`${at} names gpui-pre ${version} — use {{gpui_pre_version}}, or list an intentional pin in this test`);
          }
        }
      }
      for (const [, name] of line.matchAll(/\{\{\s*([a-z_]+)\s*\}\}/g)) {
        if (!DOC_VARIABLE_NAMES.includes(name)) failures.push(`${at} uses unknown variable {{${name}}}`);
      }
    });
  }
  assert.deepEqual(failures, [], failures.join('\n'));
});

test('both locales mark a page with the same maturity', () => {
  const maturity = (file: string) =>
    read(file).match(/^---\n[\s\S]*?^maturity:\s*(.+)$[\s\S]*?^---/m)?.[1]?.trim() ?? '';
  const failures: string[] = [];
  for (const file of files.filter((file) => !file.startsWith('zh-CN/'))) {
    const zh = `zh-CN/${file}`;
    if (!files.includes(zh)) continue;
    if (maturity(file) !== maturity(zh)) {
      failures.push(`${file} (${maturity(file) || 'none'}) and ${zh} (${maturity(zh) || 'none'})`);
    }
  }
  assert.deepEqual(failures, [], `Maturity differs between locales:\n${failures.join('\n')}`);
});
