import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { loadShowcases } from '../src/lib/showcases.ts';

const revision = 'a'.repeat(40);
async function fixture(t) {
  const root = await mkdtemp(join(tmpdir(), 'showcases-test-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  await mkdir(join(root, 'apps'));
  await writeFile(join(root, 'featured.json'), '[]');
  return root;
}
async function app(root, id, extra = {}) {
  const dir = join(root, 'apps', id);
  await mkdir(dir);
  const manifest = { id, name: id, author: 'Example', category: 'dev', platforms: ['Linux'], website: null,
    source: 'https://github.com/example/app', description: 'An app',
    previews: ['preview0.png'], ...extra };
  await writeFile(join(dir, 'manifest.json'), JSON.stringify(manifest));
  await writeFile(join(dir, 'preview0.png'), Buffer.from('89504e470d0a1a0a', 'hex'));
}

test('loads new manifests automatically, preserves editorial order and pins cover URLs', async t => {
  const root = await fixture(t);
  await app(root, 'alpha');
  await app(root, 'beta');
  await app(root, 'new-app');
  await writeFile(join(root, 'featured.json'), JSON.stringify(['beta', 'alpha']));
  const apps = await loadShowcases(root, revision);
  assert.deepEqual(apps.map(a => a.id), ['beta', 'alpha', 'new-app']);
  assert.deepEqual(apps.map(a => a.featured), [true, true, false]);
  assert.equal(apps[0].image, `https://raw.githubusercontent.com/longbridge/gpui-kit-showcases/${revision}/apps/beta/preview0.png`);
  assert.equal(apps[0].description, 'An app');
});

test('rejects missing screenshots instead of publishing a broken page', async t => {
  const root = await fixture(t);
  await app(root, 'broken', { previews: ['preview1.png'] });
  await assert.rejects(loadShowcases(root, revision), /preview1.png/);
});

test('rejects unsafe links, paths and missing descriptions', async t => {
  for (const extra of [{ website: 'javascript:alert(1)' }, { previews: ['../outside.png'] }, { description: '' }]) {
    const root = await fixture(t);
    await app(root, 'bad', extra);
    await assert.rejects(loadShowcases(root, revision), /bad/);
  }
});

test('rejects stale editorial order and empty catalogs', async t => {
  const root = await fixture(t);
  await assert.rejects(loadShowcases(root, revision), /empty/i);
  await app(root, 'alpha');
  await writeFile(join(root, 'featured.json'), '["removed-app"]');
  await assert.rejects(loadShowcases(root, revision), /featured/);
  await writeFile(join(root, 'featured.json'), '["alpha", "alpha"]');
  await assert.rejects(loadShowcases(root, revision), /featured/);
});

test('reads Featured from the root list and publication dates from manifests', async t => {
  const root = await fixture(t);
  await app(root, 'picked', { publishedAt: '2026-09-01T00:00:00Z' });
  await writeFile(join(root, 'featured.json'), '["picked"]');
  const [result] = await loadShowcases(root, revision);
  assert.equal(result.featured, true);
  assert.equal(result.publishedAt, '2026-09-01T00:00:00Z');
});
test('rejects Featured flags in app manifests', async t => {
  const root = await fixture(t);
  await app(root, 'self-picked', { featured: true });
  await assert.rejects(loadShowcases(root, revision), /featured/);
});

test('uses archived GitHub Stars without requesting live metadata', async t => {
  const root = await fixture(t);
  await app(root, 'open-source', { stars: 1234, starsUpdatedAt: '2026-09-05T00:00:00Z' });
  await app(root, 'unknown');
  const apps = await loadShowcases(root, revision);
  assert.equal(apps[0].stars, 1234);
  assert.equal(apps[0].starsUpdatedAt, '2026-09-05T00:00:00Z');
  assert.equal(apps[1].stars, null);
});
