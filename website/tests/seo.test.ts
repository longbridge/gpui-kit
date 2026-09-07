import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

const dist = new URL('../dist/', import.meta.url);
const read = (path) => readFileSync(new URL(path, dist), 'utf8');

function htmlFiles(directory) {
  return readdirSync(directory).flatMap((name) => {
    const path = join(directory, name);
    return statSync(path).isDirectory() ? htmlFiles(path) : path.endsWith('.html') ? [path] : [];
  });
}

function tag(html, pattern) {
  return html.match(pattern)?.[1]?.replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ').trim() ?? '';
}

test('primary landing pages contain server-rendered headings and copy', () => {
  for (const path of ['index.html', 'zh-CN/index.html', 'apps/index.html', 'skills/index.html', 'contributors/index.html']) {
    const html = read(path);
    assert.match(html, /<h1[\s>]/, `${path} must contain an H1 before JavaScript runs`);
    assert.ok(tag(html, /<main[^>]*>([\s\S]*?)<\/main>/).length > 100, `${path} must contain meaningful main content`);
  }
});

test('indexable pages have canonical and bilingual alternates', () => {
  for (const path of ['index.html', 'zh-CN/index.html', 'docs/index.html', 'zh-CN/docs/index.html', 'apps/index.html']) {
    const html = read(path);
    assert.match(html, /<link rel="canonical" href="https:\/\/gpui-kit\.com\//, `${path} canonical`);
    assert.match(html, /hreflang="en"/, `${path} English alternate`);
    assert.match(html, /hreflang="zh-CN"/, `${path} Chinese alternate`);
    assert.match(html, /hreflang="x-default"/, `${path} default alternate`);
  }
});

test('titles contain the GPUI Kit brand exactly once', () => {
  for (const file of htmlFiles(dist.pathname)) {
    if (file.endsWith('/404.html') || file.endsWith('/og-template.html')) continue;
    const html = readFileSync(file, 'utf8');
    const title = tag(html, /<title>([\s\S]*?)<\/title>/);
    assert.equal((title.match(/GPUI Kit/g) ?? []).length, 1, `${file} title: ${title}`);
  }
});

test('titles are unique within each language', () => {
  const seen = new Map();
  for (const file of htmlFiles(dist.pathname)) {
    if (file.endsWith('/404.html') || file.endsWith('/og-template.html')) continue;
    const relative = file.slice(dist.pathname.length);
    const locale = relative.startsWith('zh-CN/') ? 'zh-CN' : 'en';
    const title = tag(readFileSync(file, 'utf8'), /<title>([\s\S]*?)<\/title>/);
    const key = `${locale}:${title}`;
    assert.ok(!seen.has(key), `${file} duplicates title from ${seen.get(key)}: ${title}`);
    seen.set(key, file);
  }
});

test('every indexable HTML page has exactly one H1', () => {
  for (const file of htmlFiles(dist.pathname)) {
    if (file.endsWith('/404.html') || file.endsWith('/og-template.html')) continue;
    const count = (readFileSync(file, 'utf8').match(/<h1[\s>]/g) ?? []).length;
    assert.equal(count, 1, `${file} has ${count} H1 elements`);
  }
});

test('SEO discovery files and structured data are generated', () => {
  assert.ok(existsSync(new URL('robots.txt', dist)));
  assert.ok(existsSync(new URL('sitemap.xml', dist)));
  assert.match(read('index.html'), /<script type="application\/ld\+json">/);
  assert.match(read('docs/getting-started/index.html'), /BreadcrumbList/);
});

test('404 is excluded from indexing', () => {
  assert.match(read('404.html'), /<meta name="robots" content="noindex, nofollow"/);
});
