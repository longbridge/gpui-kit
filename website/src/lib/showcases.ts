import twitterText from 'twitter-text';
import { pathToFileURL } from 'node:url';
import { execFileSync } from 'node:child_process';
import { readFile, readdir, realpath } from 'node:fs/promises';
import { join, resolve, sep } from 'node:path';

/** Load only reviewed manifests from the checked-out Showcase repository. */
export async function loadShowcases(root, revision) {
  root = resolve(root);
  revision ??= execFileSync('git', ['-C', root, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
  if (!/^[a-f0-9]{40}$/.test(revision)) throw new Error('Invalid Showcase commit SHA');
  const entries = await readdir(join(root, 'apps'), { withFileTypes: true });
  const apps = [];
  for (const entry of entries.sort((a, b) => a.name.localeCompare(b.name))) {
    if (!entry.isDirectory()) throw new Error(`Unexpected Showcase entry: ${entry.name}`);
    const id = entry.name;
    const dir = join(root, 'apps', id);
    const manifest = JSON.parse(await readFile(join(dir, 'manifest.json'), 'utf8'));
    const fail = message => { throw new Error(`Showcase ${id}: ${message}`); };
    const text = value => typeof value === 'string' && value.trim().length > 0;
    if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(id) || manifest.id !== id) fail('invalid app ID');
    if (!text(manifest.author)) fail('author is required');
    if (!text(manifest.name) || !['dev', 'terminal', 'system', 'work'].includes(manifest.category)) fail('invalid name or category');
    if (!Array.isArray(manifest.platforms) || !manifest.platforms.length || !manifest.platforms.every(text)) fail('missing platforms');
    if (!text(manifest.description)) fail('an English description is required');
    if (twitterText.parseTweet(manifest.description).weightedLength > 280) fail('description exceeds 280 weighted characters');
    if (!manifest.website && !manifest.source) fail('a project link is required');
    for (const link of [manifest.website, manifest.source]) {
      if (link === null || link === undefined) continue;
      try { if (!['https:', 'http:'].includes(new URL(link).protocol)) fail('invalid project URL'); }
      catch { fail('invalid project URL'); }
    }
    if (manifest.stars != null && (!Number.isInteger(manifest.stars) || manifest.stars < 0)) fail('stars must be a nonnegative integer');
    if (manifest.starsUpdatedAt != null && !Number.isFinite(Date.parse(manifest.starsUpdatedAt))) fail('invalid starsUpdatedAt');
    if (Object.hasOwn(manifest, 'featured')) fail('featured belongs in root featured.json, not the manifest');
    if (manifest.publishedAt !== undefined && (typeof manifest.publishedAt !== 'string' || !/^\d{4}-\d{2}-\d{2}T/.test(manifest.publishedAt) || !Number.isFinite(Date.parse(manifest.publishedAt)))) fail('invalid publishedAt');
    if (manifest.building !== undefined && typeof manifest.building !== 'boolean') fail('building must be a boolean');
    if (!Array.isArray(manifest.previews) || !manifest.previews.length) fail('at least one preview is required');
    for (const name of manifest.previews) {
      if (typeof name !== 'string' || !/^preview\d*\.(png|jpe?g|webp)$/i.test(name)) fail('invalid preview filename');
      const path = await realpath(join(dir, name));
      if (!path.startsWith(await realpath(dir) + sep)) fail('preview must stay inside the app folder');
      const bytes = await readFile(path);
      const valid = /\.png$/i.test(name) ? bytes.subarray(0, 8).equals(Buffer.from('89504e470d0a1a0a', 'hex'))
        : /\.jpe?g$/i.test(name) ? bytes[0] === 255 && bytes[1] === 216
        : bytes.toString('ascii', 0, 4) === 'RIFF' && bytes.toString('ascii', 8, 12) === 'WEBP';
      if (!valid) fail(`invalid image format: ${name}`);
    }
    let hasReadme = false;
    try {
      const readme = await readFile(join(dir, 'README.md'));
      if (readme.length > 10 * 1024) fail('README exceeds 10 KB');
      hasReadme = true;
    } catch (error) { if (error.code !== 'ENOENT') throw error; }
    const mediaBase = `https://raw.githubusercontent.com/longbridge/gpui-kit-showcases/${revision}/apps/${id}`;
    apps.push({ hasReadme, mediaBase, previews: manifest.previews.map(name => `${mediaBase}/${name}`), id, name: manifest.name, author: manifest.author, category: manifest.category, platforms: manifest.platforms,
      website: manifest.website ?? null, source: manifest.source ?? null, description: manifest.description,
      building: manifest.building ?? false, featured: false,
      publishedAt: manifest.publishedAt ?? null, stars: manifest.stars ?? null,
      starsUpdatedAt: manifest.starsUpdatedAt ?? null,
      image: `https://raw.githubusercontent.com/longbridge/gpui-kit-showcases/${revision}/apps/${id}/${manifest.previews[0]}` });
  }
  if (!apps.length) throw new Error('Showcase catalog is empty');
  const order = JSON.parse(await readFile(join(root, 'featured.json'), 'utf8'));
  if (!Array.isArray(order) || new Set(order).size !== order.length || order.some(id => !apps.some(app => app.id === id))) {
    throw new Error('Invalid Showcase featured.json');
  }
  const rank = id => { const index = order.indexOf(id); return index < 0 ? order.length : index; };
  for (const app of apps) app.featured = order.includes(app.id);
  return apps.sort((a, b) => rank(a.id) - rank(b.id));
}

const showcaseRoot = () => process.env.SHOWCASES_DIR ?? resolve(process.cwd(), '../../gpui-kit-showcases');
// These checked-in modules belong to the same reviewed catalog. CI installs its
// locked dependencies before loading it; no contributor code runs before merge.
async function catalogModule(name) {
  return import(/* @vite-ignore */ pathToFileURL(join(showcaseRoot(), 'scripts', name)).href);
}
let catalog;
export function getShowcaseApps() {
  return catalog ??= (async () => {
    const { validateCatalog } = await catalogModule('validate.ts');
    await validateCatalog(showcaseRoot());
    return loadShowcases(showcaseRoot());
  })();
}
export async function getShowcaseReadme(app) {
  const { renderReadme } = await catalogModule('readme.ts');
  const markdown = await readFile(join(showcaseRoot(), 'apps', app.id, 'README.md'), 'utf8');
  return renderReadme(markdown, app, app.mediaBase);
}
