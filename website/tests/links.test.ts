import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

// Link check for a built site. Run it against the root build and against a
// build at a version base (`PUBLIC_SITE_BASE=/versions/test/`): every link a
// page renders must reach a page of the same build, and a versioned page may
// leave its version only for the routes every version shares.

const dist = new URL(process.env.SITE_TEST_DIST || '../dist/', import.meta.url).pathname;
const base = `/${(process.env.SITE_TEST_BASE || '/').replace(/^\/+|\/+$/g, '')}/`.replace(/^\/\/$/, '/');

// Deployed once at the site root and shared by every version, so they are
// neither in a versioned build nor expected to be.
const SHARED_ROOT = /^\/(?:gallery|examples|apps|zh-CN\/apps)(?:[/?#]|$)/;

function htmlFiles(directory: string): string[] {
  return readdirSync(directory).flatMap((name) => {
    // The root build does not contain the versioned builds under it.
    if (directory === dist.replace(/\/$/, '') && name === 'versions') return [];
    const path = join(directory, name);
    return statSync(path).isDirectory() ? htmlFiles(path) : path.endsWith('.html') ? [path] : [];
  });
}

function decode(value: string) {
  return value.replace(/&amp;/g, '&').replace(/&#x2F;/gi, '/').replace(/&quot;/g, '"');
}

/** Whether a site path (without its base) is a file in the build. */
function exists(path: string) {
  const clean = decodeURIComponent(path.replace(/[?#].*$/, '')).replace(/\/+$/, '');
  const file = join(dist, clean);
  return [file, `${file}.html`, join(file, 'index.html')].some((candidate) => existsSync(candidate) && statSync(candidate).isFile());
}

const pages = htmlFiles(dist.replace(/\/$/, ''));

test('the build has pages to check', () => {
  assert.ok(pages.length > 100, `expected a built site in ${dist}`);
});

test('internal links stay in their version and reach a page', () => {
  const escaped = new Set<string>();
  const broken = new Set<string>();
  for (const page of pages) {
    const html = readFileSync(page, 'utf8');
    const from = page.slice(dist.length - 1);
    for (const [tag] of html.matchAll(/<a\s[^>]*>/g)) {
      const href = tag.match(/\shref="([^"]*)"/)?.[1];
      if (!href || !href.startsWith('/') || href.startsWith('//')) continue;
      // The version menu links the same page in every version on purpose.
      if (/\sdata-version-link[\s>=]/.test(tag)) continue;
      const url = decode(href);
      if (SHARED_ROOT.test(url)) continue;

      if (!url.startsWith(base) && `${url}/` !== base) {
        escaped.add(`${from} → ${url}`);
        continue;
      }
      const path = url === base.replace(/\/$/, '') ? '/' : `/${url.slice(base.length)}`;
      if (!exists(path)) broken.add(`${from} → ${url}`);
    }
  }
  assert.deepEqual([...escaped], [], `Links that leave ${base}:\n${[...escaped].join('\n')}`);
  assert.deepEqual([...broken], [], `Links to pages that were not built:\n${[...broken].join('\n')}`);
});

test('redirect pages stay in their version', () => {
  const failures: string[] = [];
  for (const page of pages) {
    const html = readFileSync(page, 'utf8');
    const target = html.match(/<meta http-equiv="refresh" content="\d+;url=([^"]+)"/)?.[1];
    if (target && target.startsWith('/') && !target.startsWith(base)) {
      failures.push(`${page.slice(dist.length - 1)} → ${target}`);
    }
  }
  assert.deepEqual(failures, [], failures.join('\n'));
});
