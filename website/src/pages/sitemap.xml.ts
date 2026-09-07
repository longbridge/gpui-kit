import type { APIRoute } from 'astro';
import { getCollection } from 'astro:content';
import { SITE_URL } from '../lib/site';

const collections = ['docs', 'shell', 'base', 'zh-docs', 'zh-shell', 'zh-base'] as const;
const standalone = ['', 'apps', 'contributors', 'releases', 'skills'];

function escapeXml(value: string) {
  return value.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

export const GET: APIRoute = async () => {
  const paths = new Set(standalone.flatMap(path => [path, `zh-CN${path ? `/${path}` : ''}`]));

  for (const collection of collections) {
    const entries = await getCollection(collection);
    const isZh = collection.startsWith('zh-');
    const section = collection.replace(/^zh-/, '');
    for (const entry of entries) {
      const slug = entry.id.replace(/\.md$/, '');
      paths.add(`${isZh ? 'zh-CN/' : ''}${section}${slug === 'index' ? '' : `/${slug}`}`);
    }
  }

  const urls = [...paths].sort().map(path => {
    const pathname = `/${path}`.replace(/\/$/, '') || '/';
    const isZh = pathname.startsWith('/zh-CN');
    const englishPath = isZh ? pathname.replace(/^\/zh-CN(?=\/|$)/, '') || '/' : pathname;
    const chinesePath = isZh ? pathname : `/zh-CN${pathname === '/' ? '' : pathname}`;
    const loc = new URL(pathname, SITE_URL).href;
    const en = new URL(englishPath, SITE_URL).href;
    const zh = new URL(chinesePath, SITE_URL).href;
    return `<url><loc>${escapeXml(loc)}</loc><xhtml:link rel="alternate" hreflang="en" href="${escapeXml(en)}"/><xhtml:link rel="alternate" hreflang="zh-CN" href="${escapeXml(zh)}"/><xhtml:link rel="alternate" hreflang="x-default" href="${escapeXml(en)}"/></url>`;
  }).join('');

  const xml = `<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">${urls}</urlset>`;
  return new Response(xml, { headers: { 'Content-Type': 'application/xml; charset=utf-8' } });
};
