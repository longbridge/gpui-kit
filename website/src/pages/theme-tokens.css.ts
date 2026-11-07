import type { APIRoute } from 'astro';
import { themeCss } from '../lib/theme-catalog';

export const GET: APIRoute = () => new Response(themeCss, {
  headers: { 'Content-Type': 'text/css; charset=utf-8' },
});
