import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { bodyWithoutFrontmatter, expandSnippets } from './llms';

/**
 * Serves a page's markdown at its own `.md` address, the way the published site
 * does — `/docs/components/combobox.md` beside `/docs/components/combobox`.
 * The frontmatter is replaced with the two fields a reader of the raw file
 * needs: where it lives, and what it covers.
 */
export function markdownResponse(options: {
  /** The entry's own path, relative to the project root. */
  filePath: string;
  /** Public route of the rendered page, e.g. `/docs/components/combobox`. */
  route: string;
  description?: string;
}): Response {
  const absolute = join(process.cwd(), options.filePath);
  const source = readFileSync(absolute, 'utf-8');

  const front = [`url: ${options.route}.md`];
  if (options.description) front.push(`description: ${options.description}`);

  const body = expandSnippets(bodyWithoutFrontmatter(source), dirname(absolute));
  const text = `---\n${front.join('\n')}\n---\n\n${body}\n`;

  return new Response(text, {
    headers: {
      'Content-Type': 'text/markdown; charset=utf-8',
      'Cache-Control': 'public, max-age=3600',
    },
  });
}
