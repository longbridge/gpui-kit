const REPO = 'longbridge/gpui-kit';
const API_BASE = `https://api.github.com/repos/${REPO}`;

const MAX_CONTRIBUTORS = 24;
const IGNORE_LOGINS = ['dependabot[bot]', 'copilot'];

// An unauthenticated build shares one small rate limit with every other job on
// the runner, and GitHub answers a 403 with a JSON object rather than the array
// the caller expects. Send the token when CI supplies one, and treat any
// unexpected shape as "no data" so a rate-limited build still produces a page.
function requestHeaders(): HeadersInit {
  const headers: Record<string, string> = { Accept: 'application/vnd.github+json' };
  const token = import.meta.env.GITHUB_TOKEN ?? process.env.GITHUB_TOKEN;
  if (token) {
    headers.Authorization = `Bearer ${token}`;
  }
  return headers;
}

let starCount: Promise<number> | undefined;

// Every page renders the nav, so calling this per page would put one request
// per page on the wire — hundreds in a full build, far past the 60/hour an
// unauthenticated build gets. The count is baked into static output, so one
// request per process is all it can ever need.
export function fetchStarCount(): Promise<number> {
  return (starCount ??= requestStarCount());
}

async function requestStarCount(): Promise<number> {
  try {
    const res = await fetch(API_BASE, { headers: requestHeaders() });
    const data = await res.json();
    if (!res.ok) {
      console.warn(`[github] Repository API returned ${res.status}: ${data?.message ?? 'unexpected response'}`);
      return 0;
    }
    return typeof data.stargazers_count === 'number' ? data.stargazers_count : 0;
  } catch (error) {
    console.warn(`[github] Failed to fetch the star count: ${error}`);
    return 0;
  }
}

export interface Contributor {
  login: string;
  avatar_url: string;
  html_url: string;
  contributions: number;
}

let contributors: Promise<Contributor[]> | undefined;

export function fetchContributors(): Promise<Contributor[]> {
  return (contributors ??= requestContributors());
}

async function requestContributors(): Promise<Contributor[]> {
  try {
    const res = await fetch(`${API_BASE}/contributors`, { headers: requestHeaders() });
    const items = await res.json();
    if (!res.ok || !Array.isArray(items)) {
      console.warn(`[contributors] GitHub API returned ${res.status}: ${items?.message ?? 'unexpected response'}`);
      return [];
    }
    return items
      .filter((item: Contributor) => !IGNORE_LOGINS.includes(item.login?.toLowerCase()))
      .slice(0, MAX_CONTRIBUTORS);
  } catch (error) {
    console.warn(`[contributors] Failed to fetch contributors: ${error}`);
    return [];
  }
}

export function formatStarCount(count: number): string {
  if (count >= 1000) return `${(count / 1000).toFixed(1)}k`;
  return count.toString();
}
