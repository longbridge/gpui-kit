import assert from 'node:assert/strict';
import test from 'node:test';
import { selectShowcases, paginateFeatured } from '../src/lib/showcase-browser.ts';
const apps = [
  { id: 'picked', name: 'Picked', featured: true, category: 'dev', description: 'A database', author: 'Database Team', platforms: ['Linux'], stars: 1, publishedAt: '2026-01-01' },
  { id: 'older', name: 'Older', featured: false, category: 'dev', description: 'A terminal', platforms: ['Linux'], stars: 100, publishedAt: '2026-01-01' },
  { id: 'newer', name: 'Newer', featured: false, category: 'work', description: 'Notes', platforms: ['macOS'], stars: null, publishedAt: '2026-09-01' },
];
test('featured remains separate and community defaults to newest', () => {
  const result = selectShowcases(apps);
  assert.deepEqual(result.featured.map(a => a.id), ['picked']);
  assert.deepEqual(result.community.map(a => a.id), ['newer', 'older']);
});
test('star sorting puts unknown counts last without reordering featured', () => {
  const result = selectShowcases(apps, { sort: 'stars' });
  assert.deepEqual(result.community.map(a => a.id), ['older', 'newer']);
  assert.equal(result.featured[0].id, 'picked');
});
test('search matches descriptions and authors and combines with categories', () => {
  assert.equal(selectShowcases(apps, { query: 'Database Team' }).featured.length, 1);
  assert.equal(selectShowcases(apps, { query: '  TERMINAL ', category: 'dev' }).community[0].id, 'older');
  assert.equal(selectShowcases(apps, { query: 'notes', category: 'dev' }).community.length, 0);
});
test('all apps includes Featured in the requested order without changing editorial data', () => {
  const before = structuredClone(apps);
  assert.deepEqual(selectShowcases(apps, { sort: 'stars' }).all.map(a => a.id), ['older', 'picked', 'newer']);
  assert.deepEqual(selectShowcases(apps).all.map(a => a.id), ['newer', 'older', 'picked']);
  assert.deepEqual(selectShowcases(apps, { query: 'Database Team', category: 'dev' }).all.map(a => a.id), ['picked']);
  assert.deepEqual(apps, before);
});
test('Featured pagination shows six apps per page in editorial order', () => {
  const featuredApps = Array.from({ length: 16 }, (_, id) => ({ id, featured: true }));
  const catalog = [...featuredApps, { id: 99, featured: false }];
  assert.deepEqual(paginateFeatured(catalog, 1), { apps: featuredApps.slice(0, 6), page: 1, totalPages: 3, total: 16 });
  assert.deepEqual(paginateFeatured(catalog, 2).apps.map(a => a.id), [6, 7, 8, 9, 10, 11]);
  assert.deepEqual(paginateFeatured(catalog, 3).apps.map(a => a.id), [12, 13, 14, 15]);
  assert.equal(paginateFeatured(catalog, 9).page, 3);
  assert.equal(paginateFeatured(catalog, 0).page, 1);
  assert.deepEqual(paginateFeatured([], 1), { apps: [], page: 1, totalPages: 0, total: 0 });
  assert.equal(catalog.length, 17);
});


test('All apps pagination keeps sorted results in pages of nine and clamps boundaries', async () => {
  const { paginateAll } = await import('../src/lib/showcase-browser.ts');
  const apps = Array.from({ length: 23 }, (_, id) => ({ id }));
  const pages = [1, 2, 3].map(page => paginateAll(apps, page));
  assert.deepEqual(pages.map(page => page.apps.length), [9, 9, 5]);
  assert.deepEqual(pages.flatMap(page => page.apps), apps);
  assert.equal(paginateAll(apps, 99).page, 3);
  assert.equal(paginateAll(apps, 0).page, 1);
  assert.deepEqual(paginateAll([], 3), { apps: [], page: 1, totalPages: 0, total: 0 });
  assert.equal(paginateAll(apps.slice(0, 2), 3).page, 1);
});
