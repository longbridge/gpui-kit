import type { APIRoute } from 'astro';
import { buildLlmsIndex } from '../lib/llms';

export const GET: APIRoute = () => {
  const content = buildLlmsIndex(process.cwd());
  return new Response(content, {
    headers: {
      'Content-Type': 'text/plain; charset=utf-8',
      'Cache-Control': 'public, max-age=3600',
    },
  });
};
