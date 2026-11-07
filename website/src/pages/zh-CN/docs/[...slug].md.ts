import type { APIRoute } from 'astro';
import { getCollection } from 'astro:content';
import { markdownResponse } from '../../../lib/markdown-endpoint';

export async function getStaticPaths() {
  const docs = await getCollection('zh-docs');
  const components = await getCollection('zh-component');
  const routes = docs.map((entry) => {
    const slug = entry.id.replace(/\.md$/, '');
    return { params: { slug }, props: { entry, route: `/zh-CN/docs/${slug}` } };
  }).filter((route) => route.params.slug !== 'index');

  // Plain Markdown clients do not follow HTML meta-refresh redirects. Keep the
  // old endpoints readable and advertise the canonical route in frontmatter.
  const legacy = components.flatMap((entry) => {
    const slug = entry.id.replace(/\.md$/, '');
    const route = `/zh-CN/component${slug === 'index' ? '' : `/${slug}`}`;
    const aliases = slug === 'index' ? ['components', 'components/index'] : [`components/${slug}`];
    return aliases.map((slug) => ({ params: { slug }, props: { entry, route } }));
  });
  const testing = routes.find((route) => route.params.slug === 'test');
  const testAliases = testing ? [{ ...testing, params: { slug: 'ui-testing' } }] : [];
  return [...routes, ...legacy, ...testAliases];
}

export const GET: APIRoute = ({ props }) => {
  const { entry, route } = props;
  return markdownResponse({
    filePath: entry.filePath,
    route,
    description: entry.data.description,
  });
};
