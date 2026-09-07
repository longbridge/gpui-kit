import { createMarkdownProcessor } from '@astrojs/markdown-remark';
import { shikiConfig, defaultHighlightLang } from './markdown.js';

const REPO = 'longbridge/gpui-kit';
const API_URL = `https://api.github.com/repos/${REPO}/releases?per_page=100`;

export interface Release {
  tag: string;
  name: string;
  /** The date alone, so the server and the browser cannot disagree on a format. */
  date: string;
  url: string;
  prerelease: boolean;
  html: string;
}

function requestHeaders(): HeadersInit {
  const headers: Record<string, string> = { Accept: 'application/vnd.github+json' };
  const token = import.meta.env.GITHUB_TOKEN ?? process.env.GITHUB_TOKEN;
  if (token) {
    headers.Authorization = `Bearer ${token}`;
  }
  return headers;
}

// GitHub turns `#1652` and `@login` into links when it shows a release; the
// API hands back the markdown as typed. Only bare references are linked, so a
// `#` inside a URL fragment or an `@` inside an address stays as it is.
function linkReferences(markdown: string): string {
  return markdown
    .replace(
      /(^|[\s(])#(\d+)(?=[\s.,;:)]|$)/gm,
      `$1[#$2](https://github.com/${REPO}/issues/$2)`,
    )
    .replace(
      /(^|[\s(])@([A-Za-z\d](?:[A-Za-z\d-]{0,37}[A-Za-z\d])?)(?=[\s.,;:)]|$)/gm,
      '$1[@$2](https://github.com/$2)',
    );
}

// The site renderer gives every heading an id. Release notes repeat their
// headings across versions, so those ids would collide; the version headings
// the page adds itself are the anchors readers need.
function normalizeReleaseHeadings(html: string): string {
  return html
    .replace(/<h([1-6]) id="[^"]*"/g, '<h$1')
    .replace(/<h([1-6]) id='[^']*'/g, '<h$1')
    .replace(/<\/?h([1-6])(?=[\s>])/g, (tag, level) => tag.replace(`h${level}`, `h${Math.min(Number(level) + 2, 6)}`));
}

interface GitHubRelease {
  tag_name: string;
  name: string | null;
  body: string | null;
  published_at: string;
  html_url: string;
  prerelease: boolean;
  draft: boolean;
}

async function fetchReleases(): Promise<GitHubRelease[]> {
  try {
    const res = await fetch(API_URL, { headers: requestHeaders() });
    const items = await res.json();
    if (!res.ok || !Array.isArray(items)) {
      console.warn(`[releases] GitHub API returned ${res.status}: ${items?.message ?? 'unexpected response'}`);
      return [];
    }
    return items;
  } catch (error) {
    console.warn(`[releases] Failed to fetch releases: ${error}`);
    return [];
  }
}

export async function loadReleases(): Promise<Release[]> {
  const items = await fetchReleases();
  if (items.length === 0) {
    return [];
  }

  // The same shiki settings the docs use, so code in the notes gets the docs'
  // highlighting and adaptive theme.
  const processor = await createMarkdownProcessor({ shikiConfig, defaultHighlightLang });

  return Promise.all(
    items
      .filter((item) => !item.draft)
      .sort((a, b) => Date.parse(b.published_at) - Date.parse(a.published_at))
      .map(async (item) => {
        const body = linkReferences((item.body ?? '').replace(/\r\n/g, '\n'));
        const rendered = await processor.render(body);
        return {
          tag: item.tag_name,
          name: item.name || item.tag_name,
          date: item.published_at.slice(0, 10),
          url: item.html_url,
          prerelease: item.prerelease,
          html: normalizeReleaseHeadings(rendered.code),
        };
      }),
  );
}
