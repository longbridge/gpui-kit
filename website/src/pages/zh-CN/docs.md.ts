import type { APIRoute } from 'astro';
import { getCollection } from 'astro:content';
import { markdownResponse } from '../../lib/markdown-endpoint';

// The tree's own index, served at `/zh-CN/docs.md` — the address the published
// site uses, rather than the `/zh-CN/docs/.md` an empty dynamic segment produces.
export const GET: APIRoute = async () => {
  const entries = await getCollection('zh-docs');
  const index = entries.find((entry) => entry.id.replace(/\.md$/, '') === 'index');
  if (!index) return new Response('Not found', { status: 404 });
  return markdownResponse({
    filePath: index.filePath,
    route: '/zh-CN/docs',
    description: index.data.description,
  });
};
