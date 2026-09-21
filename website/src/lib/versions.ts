export interface SiteVersion {
  id: string;
  label: string;
}

const DEFAULT_VERSIONS: SiteVersion[] = [{ id: 'main', label: 'main' }];

function readVersions(): SiteVersion[] {
  const value = process.env.PUBLIC_SITE_VERSIONS;
  if (!value) return DEFAULT_VERSIONS;

  try {
    const versions = JSON.parse(value);
    if (!Array.isArray(versions)) return DEFAULT_VERSIONS;

    const valid = versions.filter(
      (version): version is SiteVersion =>
        typeof version?.id === 'string' && typeof version?.label === 'string',
    );
    return valid.length > 0 ? valid : DEFAULT_VERSIONS;
  } catch {
    return DEFAULT_VERSIONS;
  }
}

export const siteVersions = readVersions();
export const currentVersion = process.env.PUBLIC_SITE_VERSION || 'main';
export const latestVersion = process.env.PUBLIC_SITE_LATEST_VERSION || 'main';
export const isLatestVersion = process.env.PUBLIC_SITE_INDEXABLE !== 'false';

/** The path inside a version, without its version prefix. */
export function currentVersionPath(pathname: string, baseUrl: string): string {
  const prefix = baseUrl === '/' ? '' : baseUrl.replace(/\/$/, '');
  const path = prefix && pathname.startsWith(prefix)
    ? pathname.slice(prefix.length)
    : pathname;
  return `/${path.replace(/^\/+/, '')}`.replace(/\/$/, '') || '/';
}

/** Build a site-root URL for a path in a selected documentation version. */
export function versionHref(version: SiteVersion, pathname: string): string {
  const path = pathname === '/' ? '' : `/${pathname.replace(/^\/+/, '')}`;
  return version.id === latestVersion ? path || '/' : `/versions/${encodeURIComponent(version.id)}${path}`;
}
