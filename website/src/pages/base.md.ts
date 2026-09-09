import type { APIRoute } from 'astro';
import { getCollection } from 'astro:content';
import { markdownResponse } from '../lib/markdown-endpoint';

// The tree's own index, served at `/base.md` — the address the published
// site uses, rather than the `/base/.md` an empty dynamic segment produces.
export const GET: APIRoute = async () => {
  const entries = await getCollection('base');
  const index = entries.find((entry) => entry.id.replace(/\.md$/, '') === 'index');
  if (!index) return new Response('Not found', { status: 404 });
  return markdownResponse({
    filePath: index.filePath,
    route: '/base',
    description: index.data.description,
  });
};
