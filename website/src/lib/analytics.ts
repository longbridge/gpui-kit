import { createNsMetry } from './nsmetry';

// Privacy-first page analytics (https://github.com/huacnlee/nsmetry): no cookies,
// no fingerprinting. The only id is a random UUID kept in localStorage
// (`nsmetry.uid`), which the server hashes per site and never stores raw.
//
// Only the production site reports, so local dev, `astro preview`, CI builds
// and forks never send events.
export function startAnalytics(): void {
  if (location.hostname !== 'gpui-kit.com') return;

  const nsmetry = createNsMetry({
    endpoint: 'https://nsmetry.huacnlee-account.workers.dev',
    site: 'gpui-kit.com',
    persistUserId: true,
    appVersion: import.meta.env.PUBLIC_SITE_VERSION,
    // Each navigation loads a new page and flushes on page hide; the timer only
    // matters for a page left open.
    flushInterval: 30_000,
  });

  // Paths carry no ids or free text: they are the docs routes themselves.
  nsmetry.screen(location.pathname);
}
