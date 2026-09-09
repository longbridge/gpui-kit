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
    if (/<meta[^>]+http-equiv="refresh"/i.test(readFileSync(file, 'utf8'))) continue;
    const html = readFileSync(file, 'utf8');
    const title = tag(html, /<title>([\s\S]*?)<\/title>/);
    assert.equal((title.match(/GPUI Kit/g) ?? []).length, 1, `${file} title: ${title}`);
  }
});

test('titles are unique within each language', () => {
  const seen = new Map();
  for (const file of htmlFiles(dist.pathname)) {
    if (file.endsWith('/404.html') || file.endsWith('/og-template.html')) continue;
    if (/<meta[^>]+http-equiv="refresh"/i.test(readFileSync(file, 'utf8'))) continue;
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
    if (/<meta[^>]+http-equiv="refresh"/i.test(readFileSync(file, 'utf8'))) continue;
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

test('component pages have independent routes, translated alternates and readable legacy URLs', () => {
  for (const locale of ['', 'zh-CN/']) {
    const source = new URL(`../${locale}component/`, import.meta.url);
    for (const name of readdirSync(source).filter(name => name.endsWith('.md'))) {
      const slug = name.slice(0, -3);
      const route = `${locale}component${slug === 'index' ? '' : `/${slug}`}`;
      const html = read(`${route}/index.html`);
      assert.ok(html.includes(`rel="canonical" href="https://gpui-kit.com/${route}"`), route);
      assert.match(html, /hreflang="en"/);
      assert.match(html, /hreflang="zh-CN"/);
      assert.match(read(`${route}.md`), new RegExp(`^---\nurl: /${route}\\.md\n`));
      assert.match(read(`${locale}docs/components/${slug}/index.html`), /http-equiv="refresh"/);
      assert.ok(read(`${locale}docs/components/${slug}/index.html`).includes(`url=/${route}`));
      assert.equal(read(`${locale}docs/components/${slug}.md`), read(`${route}.md`));
    }
    assert.ok(read(`${locale}docs/components/index.html`).includes(`url=/${locale}component`));
    assert.equal(read(`${locale}docs/components.md`), read(`${locale}component.md`));
  }
});

test('shared guides remain under docs and sidebars keep the sections separate', () => {
  for (const locale of ['', 'zh-CN/']) {
    for (const guide of ['coding-guides', 'design-guides']) {
      const html = read(`${locale}docs/${guide}/index.html`);
      assert.ok(html.includes(`rel="canonical" href="https://gpui-kit.com/${locale}docs/${guide}"`));
      const sidebar = html.match(/<aside class="docs-sidebar"[\s\S]*?<\/aside>/)?.[0] ?? '';
      assert.ok(sidebar.includes(`href="/${locale}docs/coding-guides"`));
      assert.ok(sidebar.includes(`href="/${locale}docs/design-guides"`));
      assert.ok(!sidebar.includes(`href="/${locale}component/`));
    }
    const html = read(`${locale}component/button/index.html`);
    const sidebar = html.match(/<aside class="docs-sidebar"[\s\S]*?<\/aside>/)?.[0] ?? '';
    assert.ok(sidebar.includes(`href="/${locale}component/input"`));
    assert.ok(!sidebar.includes(`href="/${locale}docs/`));
    assert.ok(read(`${locale}index.html`).includes(`href="/${locale}docs"`));
    assert.ok(read(`${locale}index.html`).includes(`href="/${locale}component"`));
  }
});

test('component links and discovery use canonical routes', () => {
  const sitemap = read('sitemap.xml');
  const index = read('llms.txt');
  const full = read('llms-full.txt');
  for (const locale of ['', 'zh-CN/']) {
    assert.ok(sitemap.includes(`https://gpui-kit.com/${locale}component/button`));
    assert.ok(index.includes(`/${locale}component.md`));
    assert.ok(index.includes(`/${locale}component/button.md`));
    assert.ok(full.includes(`Source: /${locale}component/button`));
    assert.ok(read(`${locale}base/primitives/input/index.html`).includes(`href="/${locale}component/input"`));
    assert.ok(read(`${locale}component/icon/index.html`).includes(`href="/${locale}docs/assets"`));
    assert.ok(read(`${locale}component/index.html`).includes(`href="/${locale}component/button"`));
  }
  assert.ok(!sitemap.includes('/docs/components'));
  assert.ok(!index.includes('/docs/components'));
  assert.ok(!full.includes('/docs/components'));
});

test('primary navigation follows the Kit section order', () => {
  for (const [locale, labels] of [
    ['', ['Docs', 'Component', 'Base', 'Shell', 'App Stories']],
    ['zh-CN/', ['文档', '组件', 'Base', 'Shell', '应用案例']],
  ]) {
    const html = read(`${locale}docs/index.html`);
    const nav = html.match(/<nav class="site-nav"[\s\S]*?<\/nav>/)?.[0] ?? '';
    const links = [...nav.matchAll(/<a\s+href="([^"]+)"\s+class="site-nav__link[^"]*"[^>]*>\s*([^<]+?)\s*<\/a>/g)];
    assert.deepEqual(links.slice(0, 5).map(match => match[2]), labels);
    assert.deepEqual(links.slice(0, 5).map(match => match[1]), ['docs', 'component', 'base', 'shell', 'apps'].map(section => `/${locale}${section}`));
  }
});


test('testing guide has canonical routes and readable legacy URLs', () => {
  for (const locale of ['', 'zh-CN/']) {
    const route = `${locale}docs/test`;
    const html = read(`${route}/index.html`);
    assert.ok(html.includes(`https://gpui-kit.com/${route}`));
    assert.ok(read(`${locale}docs/ui-testing/index.html`).includes(`url=/${route}`));
    assert.equal(read(`${locale}docs/ui-testing.md`), read(`${route}.md`));
    assert.ok(read("llms.txt").includes(`/${route}.md`));
  }
});
