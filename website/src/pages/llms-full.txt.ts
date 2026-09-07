import type { APIRoute } from 'astro';
import { buildLlmsContent } from '../lib/llms';

export const GET: APIRoute = () => {
  // Not `import.meta.url`: Astro rearranges server assets during the build, so
  // the module's own location stops pointing at `src/pages` and walking up from
  // it produced a root with no markdown under it — the file built empty. Astro
  // runs from the project root, so that is the reliable anchor.
  const content = buildLlmsContent(process.cwd());
  return new Response(content, {
    headers: {
      'Content-Type': 'text/plain; charset=utf-8',
      'Cache-Control': 'public, max-age=3600',
    },
  });
};
