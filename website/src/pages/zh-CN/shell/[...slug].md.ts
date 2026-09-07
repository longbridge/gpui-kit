import type { APIRoute } from 'astro';
import { getCollection } from 'astro:content';
import { markdownResponse } from '../../../lib/markdown-endpoint';

// The published site serves each page's markdown beside the rendered page, so
// a reader — or a model — can fetch the source of what they are looking at.
export async function getStaticPaths() {
  const entries = await getCollection('zh-shell');
  return entries.map((entry) => {
    const slug = entry.id.replace(/\.md$/, '');
    return { params: { slug }, props: { entry } };
  }).filter((route) => route.params.slug !== 'index');
}

export const GET: APIRoute = ({ props }) => {
  const entry = (props as any).entry;
  const slug = entry.id.replace(/\.md$/, '');
  return markdownResponse({
    filePath: entry.filePath,
    route: `/zh-CN/shell${slug === 'index' ? '' : `/${slug}`}`,
    description: entry.data.description,
  });
};
