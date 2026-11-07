import type { APIRoute } from 'astro';
import { themeSources } from '../../lib/theme-catalog';

export function getStaticPaths() {
  return themeSources.map(({ source, data }) => ({
    params: { name: source },
    props: { data },
  }));
}

export const GET: APIRoute = ({ props }) => new Response(JSON.stringify(props.data), {
  headers: { 'Content-Type': 'application/json; charset=utf-8' },
});
